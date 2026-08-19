use std::time::Duration;

use raddy_abi::{BodyMeta, Envelope, RequestId};
use raddy_executor::{EngineBuilder, ExecError, ExecRequest, Executor, MemoryBody, ToyExecutor};

fn sample_envelope() -> Envelope {
    Envelope {
        v: raddy_abi::ABI_VERSION,
        request_id: RequestId::from_u128(1),
        method: "GET".into(),
        target: "/echo".into(),
        scheme: "http".into(),
        authority: "localhost:8080".into(),
        headers: vec![("accept".into(), "*/*".into())],
        remote_addr: "127.0.0.1:1".parse().expect("addr"),
        body: BodyMeta { len: Some(5) },
        deadline_ms: 30_000,
    }
}

fn load(name: &str) -> ToyExecutor {
    let wasm = match name {
        "echo" => include_bytes!(concat!(env!("OUT_DIR"), "/echo.wasm")).as_slice(),
        "spin" => include_bytes!(concat!(env!("OUT_DIR"), "/spin.wasm")).as_slice(),
        "trap" => include_bytes!(concat!(env!("OUT_DIR"), "/trap.wasm")).as_slice(),
        other => panic!("unknown toy guest {other}"),
    };
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade.load_wasm_bytes(wasm).expect("module");
    ToyExecutor::new(facade, module, Duration::from_secs(2)).expect("executor")
}

#[tokio::test]
async fn echo_guest_returns_head_and_body() {
    let exec = load("echo");
    let req = ExecRequest {
        head: sample_envelope(),
        body: Box::new(MemoryBody::new(b"hello".to_vec())),
    };
    let mut resp = exec.execute(req).await.expect("execute");
    assert_eq!(resp.head.status, 200);
    let mut body = Vec::new();
    while let Some(chunk) = resp.body.recv().await {
        body.extend_from_slice(&chunk);
    }
    let text = String::from_utf8(body).expect("utf8");
    assert!(
        text.contains("/echo") && text.contains("hello"),
        "echo should contain the request target and body, got {text:?}"
    );
}

#[tokio::test]
async fn spin_guest_hits_epoch_deadline_before_head() {
    let exec = load("spin").with_deadline(Duration::from_millis(50));
    let req = ExecRequest {
        head: sample_envelope(),
        body: Box::new(MemoryBody::new(Vec::new())),
    };
    let err = exec.execute(req).await.expect_err("spin must time out");
    assert!(
        matches!(err, ExecError::DeadlinePreHead),
        "expected DeadlinePreHead, got {err:?}"
    );
}

#[tokio::test]
async fn trap_guest_is_reported_as_trap() {
    let exec = load("trap");
    let req = ExecRequest {
        head: sample_envelope(),
        body: Box::new(MemoryBody::new(Vec::new())),
    };
    let err = exec.execute(req).await.expect_err("trap must fail");
    assert!(
        matches!(err, ExecError::Trap(_)),
        "expected Trap, got {err:?}"
    );
}
