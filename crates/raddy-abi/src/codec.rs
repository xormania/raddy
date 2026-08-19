use crate::envelope::{Envelope, ResponseHead};
use crate::{ABI_VERSION, AbiError};

/// Strategy for head documents. JSON v1 is the only v0.1 codec.
pub trait HeadCodec: Send + Sync {
    fn encode_envelope(&self, env: &Envelope) -> Result<Vec<u8>, AbiError>;
    fn decode_envelope(&self, bytes: &[u8]) -> Result<Envelope, AbiError>;
    fn encode_response(&self, head: &ResponseHead) -> Result<Vec<u8>, AbiError>;
    fn decode_response(&self, bytes: &[u8]) -> Result<ResponseHead, AbiError>;
}

/// JSON encoding, `abi = 1`.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonV1;

impl HeadCodec for JsonV1 {
    fn encode_envelope(&self, env: &Envelope) -> Result<Vec<u8>, AbiError> {
        serde_json::to_vec(env).map_err(|err| AbiError::Encode(err.to_string()))
    }

    fn decode_envelope(&self, bytes: &[u8]) -> Result<Envelope, AbiError> {
        let env: Envelope =
            serde_json::from_slice(bytes).map_err(|err| AbiError::Decode(err.to_string()))?;
        if env.v != ABI_VERSION {
            return Err(AbiError::Decode(format!(
                "unsupported abi version {}",
                env.v
            )));
        }
        Ok(env)
    }

    fn encode_response(&self, head: &ResponseHead) -> Result<Vec<u8>, AbiError> {
        serde_json::to_vec(head).map_err(|err| AbiError::Encode(err.to_string()))
    }

    fn decode_response(&self, bytes: &[u8]) -> Result<ResponseHead, AbiError> {
        let head: ResponseHead =
            serde_json::from_slice(bytes).map_err(|err| AbiError::Decode(err.to_string()))?;
        if head.v != ABI_VERSION {
            return Err(AbiError::Decode(format!(
                "unsupported abi version {}",
                head.v
            )));
        }
        Ok(head)
    }
}
