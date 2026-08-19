/// Maximum ResponseHead JSON the guest may emit in one `raddy_resp_head`.
pub const MAX_RESP_HEAD_BYTES: usize = 16 * 1024;

/// Maximum body chunk for one `raddy_resp_write`.
pub const MAX_RESP_CHUNK_BYTES: usize = 64 * 1024;

/// Maximum request body the host will copy, known-length or streamed.
pub const MAX_REQ_BODY_BYTES: usize = 1024 * 1024;
