use std::sync::Arc;
use std::time::Duration;

use raddy_abi::{BodyMeta, Envelope, RequestId};
use raddy_executor::{
    EngineBuilder, ExecRequest, Executor, MemoryBody, RestoreStrategy, ToyExecutor,
    snap_guest_wizer, toy_guest_wasm,
};

fn envelope(target: &str) -> Envelope {
    Envelope {
        v: raddy_abi::ABI_VERSION,
        request_id: RequestId::from_u128(1),
        method: "GET".into(),
        target: target.into(),
        scheme: "http".into(),
        authority: "localhost:8080".into(),
        headers: Vec::new(),
        remote_addr: "127.0.0.1:1".parse().expect("addr"),
        body: BodyMeta { len: Some(0) },
        deadline_ms: 30_000,
    }
}

fn snap_exec(min: usize, max: usize) -> ToyExecutor {
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
    .expect("exec")
    .with_pool(min, max)
    .expect("pool")
}

#[test]
fn fill_starts_at_min() {
    let exec = snap_exec(2, 4);
    assert_eq!(exec.pool().idle_count(), 2);
    assert_eq!(exec.pool().in_use(), 0);
}

#[test]
fn unused_steal_returns_to_idle() {
    let exec = snap_exec(2, 4);
    let stolen = exec.pool().steal().expect("steal");
    assert_eq!(exec.pool().in_use(), 1);
    assert_eq!(exec.pool().idle_count(), 1);
    drop(stolen);
    assert_eq!(exec.pool().in_use(), 0);
    assert_eq!(exec.pool().idle_count(), 2);
}

#[test]
fn spent_slot_is_not_returned() {
    let exec = snap_exec(2, 4);
    let mut stolen = exec.pool().steal().expect("steal");
    stolen.mark_spent();
    drop(stolen);
    assert_eq!(exec.pool().in_use(), 0);
    assert_eq!(exec.pool().idle_count(), 1);
}

#[test]
fn concurrent_steal_return_preserves_accounting() {
    const THREADS: usize = 24;
    let exec = Arc::new(snap_exec(0, 2));
    let start = Arc::new(std::sync::Barrier::new(THREADS));
    let admitted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            let e = Arc::clone(&exec);
            let start = Arc::clone(&start);
            let admitted = Arc::clone(&admitted);
            scope.spawn(move || {
                start.wait();
                if let Ok(stolen) = e.pool().steal() {
                    admitted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(50));
                    drop(stolen);
                }
            });
        }
    });
    assert_eq!(admitted.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(exec.pool().peak_in_use(), 2);
    assert_eq!(exec.pool().in_use(), 0);
    assert_eq!(exec.pool().idle_count(), 2);
}

#[tokio::test]
async fn trap_does_not_return_the_slot() {
    let wasm = toy_guest_wasm("trap").expect("trap guest");
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade.load_wasm_bytes(wasm).expect("module");
    let exec = ToyExecutor::with_strategy(
        facade,
        module,
        Duration::from_secs(2),
        RestoreStrategy::Fresh,
    )
    .expect("exec")
    .with_pool(2, 4)
    .expect("pool");
    assert_eq!(exec.pool().idle_count(), 2);
    let err = exec
        .execute(ExecRequest {
            head: envelope("/trap"),
            body: Box::new(MemoryBody::new(Vec::new())),
        })
        .await
        .expect_err("trap");
    assert!(matches!(err, raddy_executor::ExecError::Trap(_)));
    assert_eq!(exec.pool().in_use(), 0);
    // Spent slots are dropped; refill may restore idle. RAII is in_use == 0.
}

#[tokio::test]
async fn teardown_is_detached_at_resp_end_and_bounded() {
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade
        .load_wasm_bytes(toy_guest_wasm("teardown_spin").expect("teardown guest"))
        .expect("module");
    let exec = ToyExecutor::new(facade, module, Duration::from_secs(2))
        .expect("exec")
        .with_pool(1, 2)
        .expect("pool")
        .with_teardown_deadline(Duration::from_millis(500));
    let mut response = exec
        .execute(ExecRequest {
            head: envelope("/teardown"),
            body: Box::new(MemoryBody::new(Vec::new())),
        })
        .await
        .expect("response head");

    let end = tokio::time::timeout(Duration::from_millis(100), response.body.recv())
        .await
        .expect("resp_end must close the body before teardown finishes");
    assert!(end.is_none(), "teardown fixture emits no body chunks");
    response
        .response_complete
        .send(())
        .expect("response completion receiver");
    tokio::time::timeout(Duration::from_secs(3), response.done)
        .await
        .expect("bounded post-response guest work")
        .expect("worker done")
        .expect("clean end");
    tokio::time::timeout(Duration::from_secs(1), response.teardown)
        .await
        .expect("spent instance drop")
        .expect("teardown signal");
    assert_eq!(exec.pool().in_use(), 0);
    assert!(exec.pool().teardown_followed_response());
}
