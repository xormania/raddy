/// Failures while encoding or decoding a head document.
#[derive(Debug, thiserror::Error)]
pub enum AbiError {
    #[error("abi decode failed: {0}")]
    Decode(String),
    #[error("abi encode failed: {0}")]
    Encode(String),
}
