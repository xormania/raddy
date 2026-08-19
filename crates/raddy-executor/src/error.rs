/// Failures from [`crate::Executor::execute`].
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("deadline exceeded before response head")]
    DeadlinePreHead,
    #[error("guest trapped: {0}")]
    Trap(String),
    #[error("guest violated the ABI protocol: {0}")]
    Protocol(String),
    #[error("admission rejected: pool exhausted")]
    Saturated,
    #[error("artifact unavailable: {0}")]
    Artifact(String),
}
