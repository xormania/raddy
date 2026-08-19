use std::collections::BTreeMap;

use raddy_config::Config;

/// Per-scenario state. Bind at crate ports; do not spawn the binary.
#[derive(Debug, Default, cucumber::World)]
pub struct BddWorld {
    pub env: BTreeMap<String, String>,
    pub printed: Option<String>,
    pub parsed: Option<Config>,
}
