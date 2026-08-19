//! Layered configuration: compiled defaults → TOML → `RADDY_*` → CLI.

mod error;
mod load;
mod schema;

pub use error::ConfigError;
pub use load::{CliOverrides, EnvSource, FileSource, LoadRequest, load};
pub use schema::{
    AdminConfig, Config, DbConfig, DbDriver, ExecutorConfig, LogFormat, Millis,
    ObservabilityConfig, ServerConfig, TraceLevel,
};
