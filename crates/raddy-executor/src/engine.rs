use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use raddy_artifact::LoadedArtifact;
use wasmtime::{Config, Engine, InstanceAllocationStrategy, Module};

use crate::ExecError;

pub const WASMTIME_VERSION: &str = "47.0.3";

#[must_use]
pub fn host_target() -> String {
    match std::env::consts::OS {
        "linux" => {
            let environment = if cfg!(target_env = "musl") {
                "musl"
            } else {
                "gnu"
            };
            format!("{}-unknown-linux-{environment}", std::env::consts::ARCH)
        }
        "macos" => format!("{}-apple-darwin", std::env::consts::ARCH),
        "windows" => format!("{}-pc-windows-msvc", std::env::consts::ARCH),
        other => format!("{}-unknown-{other}", std::env::consts::ARCH),
    }
}

/// Builder for the wasmtime engine. All knobs are validated here.
#[derive(Clone, Debug)]
pub struct EngineBuilder {
    pooling: bool,
    memory_init_cow: bool,
    epoch_tick: Duration,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            pooling: true,
            memory_init_cow: true,
            epoch_tick: Duration::from_millis(10),
        }
    }
}

impl EngineBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn pooling(mut self, enabled: bool) -> Self {
        self.pooling = enabled;
        self
    }

    #[must_use]
    pub fn memory_init_cow(mut self, enabled: bool) -> Self {
        self.memory_init_cow = enabled;
        self
    }

    #[must_use]
    pub fn epoch_tick(mut self, tick: Duration) -> Self {
        self.epoch_tick = tick;
        self
    }

    pub fn build(self) -> Result<EngineFacade, ExecError> {
        if self.epoch_tick.is_zero() {
            return Err(ExecError::Artifact(
                "epoch_tick must be greater than zero".into(),
            ));
        }
        let mut config = Config::new();
        if self.pooling {
            config.allocation_strategy(InstanceAllocationStrategy::pooling());
        }
        config.memory_init_cow(self.memory_init_cow);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|err| ExecError::Artifact(err.to_string()))?;
        let ticker = Ticker::spawn(engine.clone(), self.epoch_tick);
        Ok(EngineFacade {
            engine,
            epoch_tick: self.epoch_tick,
            pooling: self.pooling,
            memory_init_cow: self.memory_init_cow,
            _ticker: Arc::new(ticker),
        })
    }
}

/// Facade over wasmtime `Engine`. The 47 → 48 LTS bump should touch this file.
#[derive(Clone, Debug)]
pub struct EngineFacade {
    engine: Engine,
    epoch_tick: Duration,
    pooling: bool,
    memory_init_cow: bool,
    _ticker: Arc<Ticker>,
}

impl EngineFacade {
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    #[must_use]
    pub fn epoch_tick(&self) -> Duration {
        self.epoch_tick
    }

    #[must_use]
    pub fn pooling_enabled(&self) -> bool {
        self.pooling
    }

    #[must_use]
    pub fn memory_init_cow_enabled(&self) -> bool {
        self.memory_init_cow
    }

    pub fn load_wasm_file(&self, path: &Path) -> Result<Module, ExecError> {
        Module::from_file(&self.engine, path).map_err(|err| ExecError::Artifact(err.to_string()))
    }

    pub fn load_wasm_bytes(&self, bytes: &[u8]) -> Result<Module, ExecError> {
        Module::from_binary(&self.engine, bytes).map_err(|err| ExecError::Artifact(err.to_string()))
    }

    pub fn serialize_module(&self, module: &Module) -> Result<Vec<u8>, ExecError> {
        module
            .serialize()
            .map_err(|err| ExecError::Artifact(err.to_string()))
    }

    /// Load bytes that were hash- and version-verified by `raddy-artifact`.
    #[allow(unsafe_code)]
    pub fn load_verified_artifact(&self, artifact: LoadedArtifact) -> Result<Module, ExecError> {
        // SAFETY: `LoadedArtifact` can only be constructed by the loader,
        // which verifies the open file's hash and Wasmtime version.
        unsafe { Module::deserialize_open_file(&self.engine, artifact.into_precompiled()) }
            .map_err(|err| ExecError::Artifact(err.to_string()))
    }

    /// Deserialize a cwasm produced by [`Self::serialize_module`] on this engine.
    ///
    /// # Safety
    /// `bytes` must be unmodified output of this wasmtime version.
    #[allow(unsafe_code)]
    pub unsafe fn load_cwasm_bytes(&self, bytes: &[u8]) -> Result<Module, ExecError> {
        // SAFETY: caller upholds the Wasmtime deserialize contract above.
        unsafe { Module::deserialize(&self.engine, bytes) }
            .map_err(|err| ExecError::Artifact(err.to_string()))
    }

    /// Deserialize a cwasm file produced by [`Self::serialize_module`].
    ///
    /// # Safety
    /// `path` must be unmodified bytes from this wasmtime version, and the file
    /// must remain unchanged for the lifetime of the returned `Module`.
    #[allow(unsafe_code)]
    pub unsafe fn load_cwasm_file(&self, path: &Path) -> Result<Module, ExecError> {
        // SAFETY: caller upholds the Wasmtime deserialize_file contract above.
        unsafe { Module::deserialize_file(&self.engine, path) }
            .map_err(|err| ExecError::Artifact(err.to_string()))
    }
}

#[derive(Debug)]
struct Ticker {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Ticker {
    fn spawn(engine: Engine, tick: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let join = thread::Builder::new()
            .name("raddy-epoch".into())
            .spawn(move || {
                while !flag.load(Ordering::Relaxed) {
                    thread::sleep(tick);
                    engine.increment_epoch();
                }
            })
            .expect("epoch ticker thread starts");
        Self {
            stop,
            join: Some(join),
        }
    }
}

impl Drop for Ticker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
