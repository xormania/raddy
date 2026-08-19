use std::time::Duration;

use raddy_abi::{HeadCodec, JsonV1};
use tokio::sync::{mpsc, oneshot};
use wasmtime::Module;

use crate::engine::EngineFacade;
use crate::limits::MAX_REQ_BODY_BYTES;
use crate::pool::InstancePool;
use crate::slot::InstanceSlot;
use crate::{ExecError, ExecRequest, ExecResponse, Executor, RestoreStrategy, Warm};

/// Runs a compiled toy (or any ABI-compatible) wasm module.
#[derive(Clone)]
pub struct ToyExecutor {
    warm: InstanceSlot<Warm>,
    pool: InstancePool,
    deadline: Duration,
    teardown_deadline: Duration,
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
        "slow_echo" => Some(include_bytes!(concat!(env!("OUT_DIR"), "/slow_echo.wasm"))),
        "teardown_spin" => Some(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/teardown_spin.wasm"
        ))),
        _ => None,
    }
}

/// Pre-Wizer hello-symfony guest (has `wizer.initialize`).
#[must_use]
pub fn snap_guest_raw() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/hello_symfony_raw.wasm"))
}

/// Post-Wizer hello-symfony guest. Instantiation is resume.
#[must_use]
pub fn snap_guest_wizer() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/hello_symfony_wizer.wasm"))
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
        let pool = InstancePool::new(warm.facade().clone(), warm.pre().clone(), 0, workers)?;
        Ok(Self {
            warm,
            pool,
            deadline,
            teardown_deadline: Duration::from_secs(2),
        })
    }

    pub fn with_pool(self, min: usize, max: usize) -> Result<Self, ExecError> {
        let pool = InstancePool::new(
            self.warm.facade().clone(),
            self.warm.pre().clone(),
            min,
            max,
        )?;
        Ok(Self { pool, ..self })
    }

    #[must_use]
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    #[must_use]
    pub fn with_teardown_deadline(mut self, deadline: Duration) -> Self {
        self.teardown_deadline = deadline;
        self
    }

    pub fn with_workers(self, n: usize) -> Result<Self, ExecError> {
        self.with_pool(0, n.max(1))
    }

    #[must_use]
    pub fn pool(&self) -> &InstancePool {
        &self.pool
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

        let stolen = self.pool.steal()?;
        let (head_tx, head_rx) = oneshot::channel();
        let (body_tx, body_rx) = mpsc::channel(16);
        let (done_tx, done_rx) = oneshot::channel();
        let (tear_tx, tear_rx) = oneshot::channel();
        let (response_tx, response_rx) = oneshot::channel();

        let deadline = self.deadline;
        let teardown_deadline = self.teardown_deadline;
        let tick = self.warm.facade().epoch_tick();
        let head_json = JsonV1
            .encode_envelope(&req.head)
            .map_err(|err| ExecError::Protocol(err.to_string()))?;
        let body = req.body;
        let runtime = tokio::runtime::Handle::current();
        let cleanup_runtime = runtime.clone();

        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let (result, mut stolen) = run_pooled(PooledJob {
                stolen,
                head_json,
                body,
                runtime,
                head_tx,
                body_tx,
                deadline,
                teardown_deadline,
                tick,
            });
            stolen.slot_mut().store.data_mut().finish_response();
            cleanup_runtime.spawn(async move {
                let _ = response_rx.await;
                pool.note_response_done();
                let teardown = tokio::task::spawn_blocking(move || {
                    drop(stolen);
                    pool.note_teardown();
                    let _ = tear_tx.send(());
                });
                let _ = teardown.await;
            });
            let _ = done_tx.send(result);
        });

        match head_rx.await {
            Ok(head) => Ok(ExecResponse {
                head,
                body: body_rx,
                done: done_rx,
                response_complete: response_tx,
                teardown: tear_rx,
            }),
            Err(_) => {
                drop(response_tx);
                let result = match done_rx.await {
                    Ok(Err(err)) => Err(err),
                    Ok(Ok(())) => Err(ExecError::Protocol(
                        "guest returned without emitting resp_head".into(),
                    )),
                    Err(_) => Err(ExecError::Trap("guest worker dropped".into())),
                };
                let _ = tear_rx.await;
                result
            }
        }
    }
}

struct PooledJob {
    stolen: crate::pool::Stolen,
    head_json: Vec<u8>,
    body: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
    runtime: tokio::runtime::Handle,
    head_tx: oneshot::Sender<raddy_abi::ResponseHead>,
    body_tx: mpsc::Sender<bytes::Bytes>,
    deadline: Duration,
    teardown_deadline: Duration,
    tick: Duration,
}

fn run_pooled(job: PooledJob) -> (Result<(), ExecError>, crate::pool::Stolen) {
    use crate::slot::deadline_ticks;

    let PooledJob {
        mut stolen,
        head_json,
        body,
        runtime,
        head_tx,
        body_tx,
        deadline,
        teardown_deadline,
        tick,
    } = job;
    stolen.slot_mut().store.data_mut().rebind(
        head_json,
        body,
        runtime,
        head_tx,
        body_tx,
        deadline,
        teardown_deadline,
    );
    stolen
        .slot_mut()
        .store
        .set_epoch_deadline(deadline_ticks(deadline.min(teardown_deadline), tick));
    stolen
        .slot_mut()
        .store
        .epoch_deadline_callback(move |store| {
            let window = store.data().epoch_window();
            if window.is_zero() {
                Ok(wasmtime::UpdateDeadline::Interrupt)
            } else {
                Ok(wasmtime::UpdateDeadline::Continue(deadline_ticks(
                    window, tick,
                )))
            }
        });
    let instance = stolen.slot_mut().instance;
    let call =
        match instance.get_typed_func::<(), i32>(&mut stolen.slot_mut().store, "raddy_execute") {
            Ok(func) => func.call(&mut stolen.slot_mut().store, ()),
            Err(err) => {
                stolen.mark_spent();
                return (Err(ExecError::Artifact(err.to_string())), stolen);
            }
        };
    let fail = stolen.slot_mut().store.data_mut().take_fail();
    let proto = stolen.slot_mut().store.data().protocol();
    stolen.mark_spent();
    if let Some(fail) = fail {
        return (Err(fail), stolen);
    }
    let result = match call {
        Ok(0) if proto == crate::Protocol::Ended => Ok(()),
        Ok(0) => Err(ExecError::Protocol("missing resp_end".into())),
        Ok(code) => Err(ExecError::Protocol(format!(
            "raddy_execute returned {code}"
        ))),
        Err(err) => {
            if is_epoch_interrupt(&err) {
                if proto == crate::Protocol::Ended {
                    Ok(())
                } else {
                    Err(ExecError::DeadlinePreHead)
                }
            } else {
                Err(ExecError::Trap(err.to_string()))
            }
        }
    };
    (result, stolen)
}

fn is_epoch_interrupt(err: &wasmtime::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<wasmtime::Trap>()
            .is_some_and(|trap| *trap == wasmtime::Trap::Interrupt)
    })
}
