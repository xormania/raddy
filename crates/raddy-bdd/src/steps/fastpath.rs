use std::time::Duration;

use cucumber::{given, then, when};
use raddy_executor::{EngineBuilder, RestoreStrategy, ToyExecutor, snap_guest_wizer};
use raddy_server::{ServerLimits, serve_tcp};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use super::http::exchange;
use crate::BddWorld;

fn snapshot_exec() -> ToyExecutor {
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade.load_wasm_bytes(snap_guest_wizer()).expect("module");
    ToyExecutor::with_strategy(
        facade,
        module,
        Duration::from_secs(2),
        RestoreStrategy::Snapshot,
    )
    .expect("executor")
}

#[given(regex = r#"^a snapshot server with pool_min (\d+) and pool_max (\d+)$"#)]
async fn start_pooled_server(world: &mut BddWorld, min: usize, max: usize) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    let exec = snapshot_exec().with_pool(min, max).expect("pool");
    world.snapshot_exec = Some(exec.clone());
    let (tx, rx) = oneshot::channel();
    let limits = ServerLimits {
        concurrency: 32,
        request_timeout: Duration::from_secs(5),
    };
    tokio::spawn(async move {
        let _ = serve_tcp(listener, exec, limits, async {
            let _ = rx.await;
        })
        .await;
    });
    world.server_addr = Some(addr);
    world.shutdown = Some(tx);
}

#[when(regex = r#"^I GET \"([^\"]+)\" (\d+) times$"#)]
async fn get_n_times(world: &mut BddWorld, path: String, n: usize) {
    let addr = world.server_addr.expect("server");
    let mut last = (0, http::HeaderMap::new(), Vec::new());
    for _ in 0..n {
        last = exchange(addr, "GET", &path, None).await;
    }
    world.last_status = Some(last.0);
    world.last_headers = Some(last.1);
    world.last_body = Some(String::from_utf8_lossy(&last.2).into_owned());
}

#[then(expr = "guest teardown completed after the response was fully sent")]
async fn teardown_after_body(world: &mut BddWorld) {
    let exec = world.snapshot_exec.clone().unwrap_or_else(snapshot_exec);
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if exec.pool().teardown_followed_response() {
            world.teardown_after_body = Some(true);
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("teardown did not follow response");
}

#[then(regex = r#"^the idle pool size returns to (\d+)$"#)]
async fn idle_returns(world: &mut BddWorld, want: usize) {
    let exec = world.snapshot_exec.as_ref().expect("pooled server");
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if exec.pool().idle_count() == want && exec.pool().in_use() == 0 {
            world.pool_idle = Some(want);
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!(
        "idle {} in_use {}, want idle {want}",
        exec.pool().idle_count(),
        exec.pool().in_use()
    );
}
