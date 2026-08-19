use std::time::Duration;

use wasmtime::{Instance, InstancePre, Module, Store};

use crate::engine::EngineFacade;
use crate::host::HostState;
use crate::{ExecError, RestoreStrategy};

/// Empty typestate: module loaded, not linked.
#[derive(Debug)]
pub struct Cold {
    facade: EngineFacade,
    module: Module,
    strategy: RestoreStrategy,
}

/// `InstancePre` ready. Not executing a request.
#[derive(Clone)]
pub struct Warm {
    facade: EngineFacade,
    pre: InstancePre<HostState>,
}

impl std::fmt::Debug for Warm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Warm").finish_non_exhaustive()
    }
}

/// One request is in flight. `finish` is the only exit.
pub struct Executing {
    store: Store<HostState>,
    instance: Instance,
}

impl std::fmt::Debug for Executing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Executing").finish_non_exhaustive()
    }
}

/// Lifecycle token. Illegal transitions do not compile.
#[derive(Clone)]
pub struct InstanceSlot<S> {
    state: S,
}

impl<S: std::fmt::Debug> std::fmt::Debug for InstanceSlot<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceSlot")
            .field("state", &self.state)
            .finish()
    }
}

impl InstanceSlot<Cold> {
    #[must_use]
    pub fn cold(facade: EngineFacade, module: Module, strategy: RestoreStrategy) -> Self {
        Self {
            state: Cold {
                facade,
                module,
                strategy,
            },
        }
    }

    pub fn instantiate(self) -> Result<InstanceSlot<Warm>, ExecError> {
        match self.state.strategy {
            RestoreStrategy::Fresh => {
                let mut linker = wasmtime::Linker::new(self.state.facade.engine());
                crate::host::register(&mut linker)?;
                let pre = linker
                    .instantiate_pre(&self.state.module)
                    .map_err(|err| ExecError::Artifact(err.to_string()))?;
                Ok(InstanceSlot {
                    state: Warm {
                        facade: self.state.facade,
                        pre,
                    },
                })
            }
        }
    }
}

impl InstanceSlot<Warm> {
    #[must_use]
    pub fn facade(&self) -> &EngineFacade {
        &self.state.facade
    }

    pub(crate) fn begin(
        &self,
        host: HostState,
        deadline: Duration,
    ) -> Result<InstanceSlot<Executing>, ExecError> {
        let mut store = Store::new(self.state.facade.engine(), host);
        let ticks = deadline_ticks(deadline, self.state.facade.epoch_tick());
        store.set_epoch_deadline(ticks);
        let instance = self
            .state
            .pre
            .instantiate(&mut store)
            .map_err(|err| ExecError::Artifact(err.to_string()))?;
        Ok(InstanceSlot {
            state: Executing { store, instance },
        })
    }
}

impl InstanceSlot<Executing> {
    pub fn call_execute(&mut self) -> Result<i32, wasmtime::Error> {
        let func = self
            .state
            .instance
            .get_typed_func::<(), i32>(&mut self.state.store, "raddy_execute")?;
        func.call(&mut self.state.store, ())
    }

    pub fn take_fail(&mut self) -> Option<ExecError> {
        self.state.store.data_mut().take_fail()
    }

    pub fn protocol(&self) -> crate::Protocol {
        self.state.store.data().protocol()
    }

    /// Consume the slot. An instance is never reused after execute returns.
    pub fn finish(self) {}
}

fn deadline_ticks(deadline: Duration, tick: Duration) -> u64 {
    let ticks = deadline.as_nanos() / tick.as_nanos().max(1);
    ticks.max(1) as u64
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::sync::{mpsc, oneshot};

    use super::InstanceSlot;
    use crate::RestoreStrategy;
    use crate::engine::EngineBuilder;
    use crate::host::HostState;

    #[tokio::test]
    async fn typestate_cold_warm_executing_finishes() {
        let facade = EngineBuilder::new().build().expect("engine");
        let wasm = include_bytes!(concat!(env!("OUT_DIR"), "/echo.wasm"));
        let module = facade.load_wasm_bytes(wasm).expect("module");
        let warm = InstanceSlot::cold(facade, module, RestoreStrategy::Fresh)
            .instantiate()
            .expect("Cold -> Warm via Fresh");

        let (head_tx, _head_rx) = oneshot::channel();
        let (body_tx, _body_rx) = mpsc::channel(4);
        let host = HostState::new(
            Vec::new(),
            Box::new(crate::MemoryBody::new(Vec::new())),
            tokio::runtime::Handle::current(),
            head_tx,
            body_tx,
            Duration::from_secs(1),
        );
        let exec = warm
            .begin(host, Duration::from_secs(1))
            .expect("Warm -> Executing");
        exec.finish();
    }
}
