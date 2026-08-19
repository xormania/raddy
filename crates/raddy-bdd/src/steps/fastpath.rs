use std::sync::Arc;
use std::time::Duration;

use cucumber::{given, then, when};
use raddy_executor::{
    EngineBuilder, RestoreStrategy, ToyExecutor, snap_guest_wizer, toy_guest_wasm,
};
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

#[given(regex = r#"^a slow pooled server with pool_min (\d+) and pool_max (\d+)$"#)]
async fn start_pooled_server(world: &mut BddWorld, min: usize, max: usize) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade
        .load_wasm_bytes(toy_guest_wasm("slow_echo").expect("slow_echo guest"))
        .expect("module");
    let exec = ToyExecutor::new(facade, module, Duration::from_secs(2))
        .expect("executor")
        .with_pool(min, max)
        .expect("pool");
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
    let start = Arc::new(tokio::sync::Barrier::new(n));
    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..n {
        let path = path.clone();
        let start = Arc::clone(&start);
        requests.spawn(async move {
            start.wait().await;
            exchange(addr, "GET", &path, None).await
        });
    }
    let mut responses = Vec::with_capacity(n);
    while let Some(result) = requests.join_next().await {
        responses.push(result.expect("burst request task"));
    }
    assert_eq!(responses.len(), n, "one response per burst request");
    world.burst_statuses = responses.iter().map(|response| response.0).collect();
    let last = responses
        .pop()
        .expect("burst contains at least one request");
    world.last_status = Some(last.0);
    world.last_headers = Some(last.1);
    world.last_body = Some(String::from_utf8_lossy(&last.2).into_owned());
}

#[then(regex = r#"^all (\d+) HTTP statuses are (\d+)$"#)]
async fn all_http_statuses(world: &mut BddWorld, count: usize, status: u16) {
    assert_eq!(world.burst_statuses.len(), count);
    assert!(
        world.burst_statuses.iter().all(|got| *got == status),
        "burst statuses {:?}, want every response to be {status}",
        world.burst_statuses
    );
}

#[then(expr = "the peak pool use exceeded pool_min")]
async fn peak_exceeded_min(world: &mut BddWorld) {
    let pool = world.snapshot_exec.as_ref().expect("pooled server").pool();
    assert!(
        pool.peak_in_use() > pool.min(),
        "peak pool use {} did not exceed pool_min {}",
        pool.peak_in_use(),
        pool.min()
    );
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
