use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::ABI_VERSION;
use crate::request_id::RequestId;

/// Request head. Canonical encoding is JSON, `v = 1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub request_id: RequestId,
    pub method: String,
    pub target: String,
    pub scheme: String,
    pub authority: String,
    /// Lowercase names; repeated headers are repeated pairs, order preserved.
    pub headers: Vec<(String, String)>,
    pub remote_addr: SocketAddr,
    pub body: BodyMeta,
    pub deadline_ms: u64,
}

/// Known body length, or `null` when the request is chunked / unknown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyMeta {
    pub len: Option<u64>,
}

/// Response head. Mirrors [`Envelope`] down to the header-pair rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseHead {
    pub v: u32,
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

impl ResponseHead {
    #[must_use]
    pub fn new(status: u16, headers: Vec<(String, String)>) -> Self {
        Self {
            v: ABI_VERSION,
            status,
            headers,
        }
    }
}
