use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// ULID minted at ingress and threaded through spans, logs, and teardown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(ulid::Ulid);

impl RequestId {
    #[must_use]
    pub fn new() -> Self {
        Self(ulid::Ulid::generate())
    }

    #[must_use]
    pub const fn from_ulid(id: ulid::Ulid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn from_u128(bits: u128) -> Self {
        Self(ulid::Ulid(bits))
    }

    #[must_use]
    pub const fn as_ulid(self) -> ulid::Ulid {
        self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for RequestId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(ulid::Ulid::from_string(s)?))
    }
}
