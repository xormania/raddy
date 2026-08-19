use std::path::PathBuf;

/// Failures while loading or emitting configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid configuration: {0}")]
    Extract(Box<figment::Error>),
    #[error("invalid configuration file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to emit config as TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("unknown configuration key: {0}")]
    UnknownKey(String),
    #[error("missing environment variable `{0}` referenced as env:{0}")]
    MissingEnv(String),
    #[error("invalid env: reference `{0}`")]
    BadEnvRef(String),
    #[error("executor.pool_min ({min}) exceeds executor.pool_max ({max})")]
    PoolBounds { min: u32, max: u32 },
    #[error("server.concurrency must be at least 1")]
    Concurrency,
}
impl From<figment::Error> for ConfigError {
    fn from(err: figment::Error) -> Self {
        Self::Extract(Box::new(err))
    }
}
