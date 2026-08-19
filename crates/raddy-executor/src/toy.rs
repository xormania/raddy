use std::sync::Arc;
use std::time::Duration;

use raddy_abi::{HeadCodec, JsonV1};
use tokio::sync::{Semaphore, mpsc, oneshot};
use wasmtime::Module;

use crate::engine::EngineFacade;
use crate::host::HostState;
use crate::limits::MAX_REQ_BODY_BYTES;
use crate::slot::InstanceSlot;
use crate::{ExecError, ExecRequest, ExecResponse, Executor, RestoreStrategy, Warm};

/// Runs a compiled toy (or any ABI-compatible) wasm module.
#[derive(Clone)]
pub struct ToyExecutor {
    warm: InstanceSlot<Warm>,
    deadline: Duration,
    admission: Arc<Semaphore>,
}

impl std::fmt::Debug for ToyExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToyExecutor")
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

/// Built-in toy guests compiled by this crate's `build.rs`.
#[must_use]
pub fn toy_guest_wasm(name: &str) -> Option<&'static [u8]> {
    match name {
        "echo" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/echo.wasm"))),
        "spin" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/spin.wasm"))),
        "trap" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/trap.wasm"))),
        "no_end" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/no_end.wasm"))),
        "nonzero" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/nonzero.wasm"))),
        "bad_head" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/bad_head.wasm"))),
        "huge_len" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/huge_len.wasm"))),
        "oob_write" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/oob_write.wasm"))),
        "no_body_read" => Some(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/no_body_read.wasm"
        ))),
        "two_reads" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/two_reads.wasm"))),
        "flood_write" => Some(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/flood_write.wasm"
        ))),
        _ => None,
    }
}

impl ToyExecutor {
    pub fn new(
        facade: EngineFacade,
        module: Module,
        deadline: Duration,
    ) -> Result<Self, ExecError> {
        Self::with_strategy(facade, module, deadline, RestoreStrategy::Fresh)
    }

    pub fn with_strategy(
        facade: EngineFacade,
        module: Module,
        deadline: Duration,
        strategy: RestoreStrategy,
    ) -> Result<Self, ExecError> {
        let warm = InstanceSlot::cold(facade, module, strategy).instantiate()?;
        let workers = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        Ok(Self {
            warm,
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

    #[must_use]
    pub fn worker_limit(&self) -> usize {
        self.admission.available_permits()
    }
}

impl Executor for ToyExecutor {
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

        let (head_tx, head_rx) = oneshot::channel();
        let (body_tx, body_rx) = mpsc::channel(16);
        let (done_tx, done_rx) = oneshot::channel();

        let warm = self.warm.clone();
        let deadline = self.deadline;
        let head_json = JsonV1
            .encode_envelope(&req.head)
            .map_err(|err| ExecError::Protocol(err.to_string()))?;
        let body = req.body;
        let runtime = tokio::runtime::Handle::current();

        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let result = run_guest(warm, head_json, body, runtime, head_tx, body_tx, deadline);
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
                    "guest returned without emitting resp_head".into(),
                )),
                Err(_) => Err(ExecError::Trap("guest worker dropped".into())),
            },
        }
    }
}

fn run_guest(
    warm: InstanceSlot<Warm>,
    head_json: Vec<u8>,
    body: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
    runtime: tokio::runtime::Handle,
    head_tx: oneshot::Sender<raddy_abi::ResponseHead>,
    body_tx: mpsc::Sender<bytes::Bytes>,
    deadline: Duration,
) -> Result<(), ExecError> {
    let host = HostState::new(head_json, body, runtime, head_tx, body_tx, deadline);
    let mut exec = warm.begin(host, deadline)?;
    let call = exec.call_execute();
    if let Some(fail) = exec.take_fail() {
        exec.finish();
        return Err(fail);
    }
    let proto = exec.protocol();
    exec.finish();
    match call {
        Ok(0) if proto == crate::Protocol::Ended => Ok(()),
        Ok(0) => Err(ExecError::Protocol("missing resp_end".into())),
        Ok(code) => Err(ExecError::Protocol(format!(
            "raddy_execute returned {code}"
        ))),
        Err(err) => {
            if is_epoch_interrupt(&err) {
                Err(ExecError::DeadlinePreHead)
            } else {
                Err(ExecError::Trap(err.to_string()))
            }
        }
    }
}

fn is_epoch_interrupt(err: &wasmtime::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<wasmtime::Trap>()
            .is_some_and(|trap| *trap == wasmtime::Trap::Interrupt)
    })
}
