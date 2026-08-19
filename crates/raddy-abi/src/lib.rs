//! Envelope and response-head codecs, plus the shared host/guest constants.

mod codec;
mod envelope;
mod error;
mod request_id;

pub use codec::{HeadCodec, JsonV1};
pub use envelope::{BodyMeta, Envelope, ResponseHead};
pub use error::AbiError;
pub use request_id::RequestId;

/// Canonical ABI version carried in every head (`v` field).
pub const ABI_VERSION: u32 = 1;

/// Wasm import namespace registered by the host.
pub const HOST_NAMESPACE: &str = "raddy";

/// Guest export invoked once per request.
pub const EXECUTE_EXPORT: &str = "raddy_execute";
