//! Stock WLR `php-cgi` guest. Stage 3 fallback (ADR 0008).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use raddy_abi::Envelope;
use tokio::io::AsyncReadExt;
use tokio::sync::{Semaphore, mpsc, oneshot};
use wasmtime::{InstancePre, Linker, Module, Store};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

use crate::cgi_parse::parse_cgi_response;
use crate::engine::EngineFacade;
use crate::limits::{MAX_REQ_BODY_BYTES, MAX_RESP_HEAD_BYTES};
use crate::{ExecError, ExecRequest, ExecResponse, Executor};

const CGI_SCRIPT: &str = "/app/handle.php";
const CGI_STDOUT_CAP: usize = MAX_REQ_BODY_BYTES + MAX_RESP_HEAD_BYTES;
const CGI_STDERR_CAP: usize = 64 * 1024;

/// Runs one request through WLR `php-cgi.wasm` with a `/app` preopen.
#[derive(Clone)]
pub struct CgiExecutor {
    facade: EngineFacade,
    pre: InstancePre<WasiP1Ctx>,
    app_dir: PathBuf,
    deadline: Duration,
    admission: Arc<Semaphore>,
}

impl std::fmt::Debug for CgiExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CgiExecutor")
            .field("app_dir", &self.app_dir)
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

impl CgiExecutor {
    pub fn new(
        facade: EngineFacade,
        module: Module,
        app_dir: PathBuf,
        deadline: Duration,
    ) -> Result<Self, ExecError> {
        let mut linker = Linker::new(facade.engine());
        p1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|err| ExecError::Artifact(err.to_string()))?;
        let pre = linker
            .instantiate_pre(&module)
            .map_err(|err| ExecError::Artifact(err.to_string()))?;
        let workers = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        Ok(Self {
            facade,
            pre,
            app_dir,
            deadline,
            admission: Arc::new(Semaphore::new(workers)),
        })
    }

    #[must_use]
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    #[must_use]
    pub fn with_workers(self, n: usize) -> Self {
        Self {
            admission: Arc::new(Semaphore::new(n.max(1))),
            ..self
        }
    }
}

/// [`CgiExecutor`] under the plan's `PhpExecutor` name.
#[derive(Clone, Debug)]
pub struct PhpExecutor {
    cgi: CgiExecutor,
}

impl PhpExecutor {
    pub fn from_cgi(cgi: CgiExecutor) -> Self {
        Self { cgi }
    }

    pub fn discover() -> Result<Self, ExecError> {
        let wasm = discover_repo_path("guest/php/out/php-cgi-8.2.6.wasm").ok_or_else(|| {
            ExecError::Artifact(
                "guest/php/out/php-cgi-8.2.6.wasm missing; run `just guest-php`".into(),
            )
        })?;
        let app_dir = discover_repo_path("guest/apps/plain")
            .ok_or_else(|| ExecError::Artifact("guest/apps/plain missing".into()))?;
        let facade = crate::EngineBuilder::new().pooling(false).build()?;
        let module = facade.load_wasm_file(&wasm)?;
        Ok(Self::from_cgi(CgiExecutor::new(
            facade,
            module,
            app_dir,
            Duration::from_secs(30),
        )?))
    }

    #[must_use]
    pub fn with_deadline(self, deadline: Duration) -> Self {
        Self {
            cgi: self.cgi.with_deadline(deadline),
        }
    }
}

impl Executor for PhpExecutor {
    async fn execute(&self, req: ExecRequest) -> Result<ExecResponse, ExecError> {
        self.cgi.execute(req).await
    }
}

impl Executor for CgiExecutor {
    async fn execute(&self, req: ExecRequest) -> Result<ExecResponse, ExecError> {
        if req
            .head
            .body
            .len
            .is_some_and(|n| n as usize > MAX_REQ_BODY_BYTES)
        {
            return Err(ExecError::Protocol("request body exceeds maximum".into()));
        }

        let permit = self
            .admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| ExecError::Saturated)?;

        let body = read_body(req.body).await?;
        let env = cgi_env(&req.head, body.len());
        let app_dir = self.app_dir.clone();
        let pre = self.pre.clone();
        let engine = self.facade.engine().clone();
        let tick = self.facade.epoch_tick();
        let deadline = self.deadline;

        let (head_tx, head_rx) = oneshot::channel();
        let (body_tx, body_rx) = mpsc::channel(16);
        let (done_tx, done_rx) = oneshot::channel();

        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let result = run_cgi(CgiJob {
                engine,
                pre,
                app_dir,
                env,
                body,
                tick,
                deadline,
                head_tx,
                body_tx,
            });
            let _ = done_tx.send(result);
        });

        match head_rx.await {
            Ok(head) => Ok(ExecResponse {
                head,
                body: body_rx,
                done: done_rx,
            }),
            Err(_) => match done_rx.await {
                Ok(Err(err)) => Err(err),
                Ok(Ok(())) => Err(ExecError::Protocol(
                    "cgi guest returned without a response head".into(),
                )),
                Err(_) => Err(ExecError::Trap("cgi worker dropped".into())),
            },
        }
    }
}

struct CgiJob {
    engine: wasmtime::Engine,
    pre: InstancePre<WasiP1Ctx>,
    app_dir: PathBuf,
    env: Vec<(String, String)>,
    body: Vec<u8>,
    tick: Duration,
    deadline: Duration,
    head_tx: oneshot::Sender<raddy_abi::ResponseHead>,
    body_tx: mpsc::Sender<Bytes>,
}

fn run_cgi(job: CgiJob) -> Result<(), ExecError> {
    let CgiJob {
        engine,
        pre,
        app_dir,
        env,
        body,
        tick,
        deadline,
        head_tx,
        body_tx,
    } = job;
    let stdout = MemoryOutputPipe::new(CGI_STDOUT_CAP);
    let stderr = MemoryOutputPipe::new(CGI_STDERR_CAP);
    let stdin = MemoryInputPipe::new(Bytes::from(body));

    let mut builder = WasiCtxBuilder::new();
    builder
        .stdin(stdin)
        .stdout(stdout.clone())
        .stderr(stderr.clone())
        .allow_blocking_current_thread(true)
        .args(&[
            "php-cgi",
            "-d",
            "max_execution_time=0",
            "-d",
            "display_errors=0",
            CGI_SCRIPT,
        ])
        .envs(&env)
        .initial_cwd("/app");
    builder
        .preopened_dir(&app_dir, "/app", DirPerms::READ, FilePerms::READ)
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    let wasi = builder.build_p1();
    let mut store = Store::new(&engine, wasi);
    store.set_epoch_deadline(deadline_ticks(deadline, tick));

    let instance = pre
        .instantiate(&mut store)
        .map_err(|err| ExecError::Artifact(err.to_string()))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    match start.call(&mut store, ()) {
        Ok(()) => {}
        Err(err) => {
            if is_epoch_interrupt(&err) {
                return Err(ExecError::DeadlinePreHead);
            }
            if let Some(exit) = err.downcast_ref::<wasmtime_wasi::I32Exit>() {
                if exit.0 != 0 {
                    let err_out = String::from_utf8_lossy(&stderr.contents()).into_owned();
                    return Err(ExecError::Trap(format!(
                        "php-cgi exited {}: {err_out}",
                        exit.0
                    )));
                }
            } else {
                return Err(ExecError::Trap(err.to_string()));
            }
        }
    }

    let parsed = parse_cgi_response(&stdout.contents())?;
    let _ = head_tx.send(parsed.head);
    if !parsed.body.is_empty() {
        let _ = body_tx.blocking_send(Bytes::from(parsed.body));
    }
    Ok(())
}

fn cgi_env(head: &Envelope, body_len: usize) -> Vec<(String, String)> {
    let (_path, query) = split_target(&head.target);
    let (server_name, server_port) = split_authority(&head.authority);
    let mut env = vec![
        ("GATEWAY_INTERFACE".into(), "CGI/1.1".into()),
        ("REQUEST_METHOD".into(), head.method.clone()),
        ("SCRIPT_NAME".into(), "/handle.php".into()),
        ("SCRIPT_FILENAME".into(), CGI_SCRIPT.into()),
        ("DOCUMENT_ROOT".into(), "/app".into()),
        ("REQUEST_URI".into(), head.target.clone()),
        ("QUERY_STRING".into(), query.into()),
        ("PATH_INFO".into(), String::new()),
        ("REDIRECT_STATUS".into(), "200".into()),
        ("SERVER_SOFTWARE".into(), "raddy".into()),
        ("SERVER_PROTOCOL".into(), "HTTP/1.1".into()),
        ("CONTENT_LENGTH".into(), body_len.to_string()),
        ("REMOTE_ADDR".into(), head.remote_addr.ip().to_string()),
        ("REMOTE_PORT".into(), head.remote_addr.port().to_string()),
        ("SERVER_NAME".into(), server_name.into()),
        ("SERVER_PORT".into(), server_port.into()),
    ];
    for (name, value) in &head.headers {
        if name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        if name.eq_ignore_ascii_case("content-type") {
            env.push(("CONTENT_TYPE".into(), value.clone()));
            continue;
        }
        let key = format!("HTTP_{}", name.to_ascii_uppercase().replace('-', "_"));
        env.push((key, value.clone()));
    }
    env
}

fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    }
}

fn split_authority(authority: &str) -> (&str, &str) {
    match authority.rsplit_once(':') {
        Some((name, port)) if !name.is_empty() => (name, port),
        _ => (authority, "80"),
    }
}

fn deadline_ticks(deadline: Duration, tick: Duration) -> u64 {
    let ticks = deadline.as_nanos() / tick.as_nanos().max(1);
    ticks.max(1) as u64
}

fn is_epoch_interrupt(err: &wasmtime::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<wasmtime::Trap>()
            .is_some_and(|trap| *trap == wasmtime::Trap::Interrupt)
    })
}

async fn read_body(
    mut body: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
) -> Result<Vec<u8>, ExecError> {
    let mut buf = Vec::new();
    let mut tmp = [0_u8; 8192];
    loop {
        let n = body
            .read(&mut tmp)
            .await
            .map_err(|err| ExecError::Protocol(err.to_string()))?;
        if n == 0 {
            break;
        }
        if buf.len() + n > MAX_REQ_BODY_BYTES {
            return Err(ExecError::Protocol("request body exceeds maximum".into()));
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    Ok(buf)
}

/// Walk from cwd and this crate's manifest dir to a repo-relative path.
pub fn discover_repo_path(rel: &str) -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    starts.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    for start in starts {
        let mut dir: &Path = &start;
        loop {
            let cand = dir.join(rel);
            if cand.exists() {
                return Some(cand);
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }
    }
    None
}
