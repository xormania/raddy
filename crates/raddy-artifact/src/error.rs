/// Failures from manifest parse, hash checks, or the Wizer CLI.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("invalid artifact manifest: {0}")]
    Manifest(String),
    #[error("artifact module hash mismatch")]
    HashMismatch,
    #[error("artifact has no precompiled module for target {0}")]
    MissingPrecompiled(String),
    #[error("artifact cwasm uses wasmtime {got}, host requires {want}")]
    WasmtimeVersion { got: String, want: String },
    #[error("unsupported artifact abi {0}")]
    Abi(u32),
    #[error("wizer failed: {0}")]
    Wizer(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
