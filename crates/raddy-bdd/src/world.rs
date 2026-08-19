use std::collections::BTreeMap;
use std::net::SocketAddr;

use raddy_config::Config;
use tokio::sync::oneshot;

/// Per-scenario state. Bind at crate ports; do not spawn the binary.
#[derive(Debug, Default, cucumber::World)]
pub struct BddWorld {
    pub env: BTreeMap<String, String>,
    pub printed: Option<String>,
    pub parsed: Option<Config>,
    pub executor: Option<raddy_executor::ToyExecutor>,
    pub last_status: Option<u16>,
    pub last_body: Option<String>,
    pub last_error: Option<String>,
    pub server_addr: Option<SocketAddr>,
    pub server_concurrency: Option<usize>,
    pub shutdown: Option<oneshot::Sender<()>>,
    pub last_headers: Option<http::HeaderMap>,
    pub first_byte: Option<u8>,
    pub more_after_first: Option<Vec<u8>>,
    pub prior_body: Option<String>,
    pub teardown_after_body: Option<bool>,
    pub pool_idle: Option<usize>,
    pub snapshot_exec: Option<raddy_executor::ToyExecutor>,
}
