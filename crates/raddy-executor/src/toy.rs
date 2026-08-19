use std::time::Duration;

use raddy_abi::{HeadCodec, JsonV1};
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot};
use wasmtime::{InstancePre, Linker, Module, Store};

use crate::engine::EngineFacade;
use crate::host::{self, HostState};
use crate::{ExecError, ExecRequest, ExecResponse, Executor};

/// Runs a compiled toy (or any ABI-compatible) wasm module.
/// Runs a compiled toy (or any ABI-compatible) wasm module.
#[derive(Clone)]
pub struct ToyExecutor {
    facade: EngineFacade,
    pre: InstancePre<HostState>,
    deadline: Duration,
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
        _ => None,
    }
}

impl ToyExecutor {
    pub fn new(
        facade: EngineFacade,
        module: Module,
        deadline: Duration,
    ) -> Result<Self, ExecError> {
        let mut linker = Linker::new(facade.engine());
        host::register(&mut linker)?;
        let pre = linker
            .instantiate_pre(&module)
            .map_err(|err| ExecError::Artifact(err.to_string()))?;
        Ok(Self {
            facade,
            pre,
            deadline,
        })
    }

    #[must_use]
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }
}

impl Executor for ToyExecutor {
    async fn execute(&self, req: ExecRequest) -> Result<ExecResponse, ExecError> {
        let (head_tx, head_rx) = oneshot::channel();
        let (body_tx, body_rx) = mpsc::channel(16);
        let (done_tx, done_rx) = oneshot::channel();

        let pre = self.pre.clone();
        let engine = self.facade.engine().clone();
        let tick = self.facade.epoch_tick();
        let deadline = self.deadline;
        let head_json = JsonV1
            .encode_envelope(&req.head)
            .map_err(|err| ExecError::Protocol(err.to_string()))?;

        let mut body = req.body;
        tokio::task::spawn_blocking(move || {
            let result = run_guest(GuestJob {
                engine,
                pre,
                head_json,
                body: &mut body,
                head_tx,
                body_tx,
                tick,
                deadline,
            });
            let _ = done_tx.send(result);
        });

        match head_rx.await {
            Ok(head) => Ok(ExecResponse {
                head,
                body: body_rx,
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

struct GuestJob<'a> {
    engine: wasmtime::Engine,
    pre: InstancePre<HostState>,
    head_json: Vec<u8>,
    body: &'a mut (dyn tokio::io::AsyncRead + Send + Unpin),
    head_tx: oneshot::Sender<raddy_abi::ResponseHead>,
    body_tx: mpsc::Sender<bytes::Bytes>,
    tick: Duration,
    deadline: Duration,
}

fn run_guest(job: GuestJob<'_>) -> Result<(), ExecError> {
    let GuestJob {
        engine,
        pre,
        head_json,
        body,
        head_tx,
        body_tx,
        tick,
        deadline,
    } = job;
    let handle = tokio::runtime::Handle::current();
    let mut body_bytes = Vec::new();
    handle
        .block_on(body.read_to_end(&mut body_bytes))
        .map_err(|err| ExecError::Protocol(err.to_string()))?;

    let mut store = Store::new(
        &engine,
        HostState::new(head_json, body_bytes, head_tx, body_tx),
    );
    let ticks = deadline_ticks(deadline, tick);
    store.set_epoch_deadline(ticks);

    let instance = pre
        .instantiate(&mut store)
        .map_err(|err| ExecError::Artifact(err.to_string()))?;
    let exec = instance
        .get_typed_func::<(), i32>(&mut store, "raddy_execute")
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    let call = exec.call(&mut store, ());
    if let Some(fail) = store.data_mut().take_fail() {
        return Err(fail);
    }
    match call {
        Ok(_) => Ok(()),
        Err(err) => {
            if is_epoch_interrupt(&err) {
                Err(ExecError::DeadlinePreHead)
            } else {
                Err(ExecError::Trap(err.to_string()))
            }
        }
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
