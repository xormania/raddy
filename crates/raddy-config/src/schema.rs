use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Milliseconds. Newtype so timeout fields cannot be confused with other integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Millis(u64);

impl Millis {
    #[must_use]
    pub const fn new(ms: u64) -> Self {
        Self(ms)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Fully-resolved runtime configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub admin: AdminConfig,
    pub executor: ExecutorConfig,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub db: BTreeMap<String, DbConfig>,
    pub observability: ObservabilityConfig,
}

impl Config {
    /// Emit the resolved config as pretty TOML.
    pub fn to_toml(&self) -> Result<String, super::ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub concurrency: u32,
    pub request_timeout_ms: Millis,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8080"
                .parse()
                .expect("compiled default server.listen is a valid socket address"),
            concurrency: 256,
            request_timeout_ms: Millis::new(30_000),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdminConfig {
    pub listen: SocketAddr,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:9090"
                .parse()
                .expect("compiled default admin.listen is a valid socket address"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExecutorConfig {
    pub artifact: PathBuf,
    pub pool_min: u32,
    pub pool_max: u32,
    pub deadline_ms: Millis,
    pub epoch_tick_ms: Millis,
    pub teardown_deadline_ms: Millis,
    pub debug_fuel: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            artifact: PathBuf::from("artifacts/hello"),
            pool_min: 8,
            pool_max: 64,
            deadline_ms: Millis::new(30_000),
            epoch_tick_ms: Millis::new(10),
            teardown_deadline_ms: Millis::new(2_000),
            debug_fuel: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DbConfig {
    pub driver: DbDriver,
    pub url: String,
    pub pool_max: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbDriver {
    Postgres,
    Sqlite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ObservabilityConfig {
    pub log_format: LogFormat,
    pub trace_level: TraceLevel,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Json,
            trace_level: TraceLevel::Info,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
