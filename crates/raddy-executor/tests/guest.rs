use std::time::{Duration, Instant};

use raddy_abi::{BodyMeta, Envelope, RequestId};
use raddy_executor::{
    EngineBuilder, ExecError, ExecRequest, Executor, MAX_REQ_BODY_BYTES, MemoryBody, OpenBody,
    ToyExecutor, toy_guest_wasm,
};

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
    let wasm = toy_guest_wasm(name).unwrap_or_else(|| panic!("unknown toy guest {name}"));
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade.load_wasm_bytes(wasm).expect("module");
    ToyExecutor::new(facade, module, Duration::from_secs(2)).expect("executor")
}

fn req(body: impl tokio::io::AsyncRead + Send + Unpin + 'static) -> ExecRequest {
    ExecRequest {
        head: sample_envelope(),
        body: Box::new(body),
    }
}

#[tokio::test]
async fn echo_guest_returns_head_and_body() {
    let mut resp = load("echo")
        .execute(req(MemoryBody::new(b"hello".to_vec())))
        .await
        .expect("execute");
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
    resp.done.await.expect("done").expect("clean end");
}

#[tokio::test]
async fn spin_guest_hits_epoch_deadline_before_head() {
    let err = load("spin")
        .with_deadline(Duration::from_millis(50))
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect_err("spin must time out");
    assert!(
        matches!(err, ExecError::DeadlinePreHead),
        "expected DeadlinePreHead, got {err:?}"
    );
}

#[tokio::test]
async fn trap_guest_is_reported_as_trap() {
    let err = load("trap")
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect_err("trap must fail");
    assert!(
        matches!(err, ExecError::Trap(_)),
        "expected Trap, got {err:?}"
    );
}

#[tokio::test]
async fn missing_end_is_protocol() {
    let resp = load("no_end")
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect("head still arrives");
    let done = resp.done.await.expect("worker finished");
    assert!(
        matches!(done, Err(ExecError::Protocol(ref k)) if k.contains("missing resp_end")),
        "expected missing resp_end, got {done:?}"
    );
}

#[tokio::test]
async fn nonzero_execute_is_protocol() {
    let resp = load("nonzero")
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect("head still arrives");
    let done = resp.done.await.expect("worker finished");
    assert!(
        matches!(done, Err(ExecError::Protocol(ref k)) if k.contains("returned 7")),
        "expected non-zero return, got {done:?}"
    );
}

#[tokio::test]
async fn malformed_head_is_protocol_not_trap() {
    let err = load("bad_head")
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect_err("bad head");
    assert!(
        matches!(err, ExecError::Protocol(ref k) if k.contains("invalid response head")),
        "expected Protocol, got {err:?}"
    );
}

#[tokio::test]
async fn huge_declared_length_does_not_kill_host() {
    let started = Instant::now();
    let err = load("huge_len")
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect_err("huge length");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "must not allocate i32::MAX"
    );
    assert!(
        matches!(err, ExecError::Protocol(_)),
        "expected Protocol, got {err:?}"
    );
    load("echo")
        .execute(req(MemoryBody::new(b"hello".to_vec())))
        .await
        .expect("host still serves after hostile length");
}

#[tokio::test]
async fn oob_write_after_head_is_protocol() {
    let resp = load("oob_write")
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect("head");
    let done = resp.done.await.expect("worker finished");
    assert!(
        matches!(done, Err(ExecError::Protocol(_))),
        "expected Protocol, got {done:?}"
    );
}

#[tokio::test]
async fn guest_emits_head_without_reading_or_waiting_for_body_eof() {
    let started = Instant::now();
    let resp = tokio::time::timeout(
        Duration::from_millis(500),
        load("no_body_read").execute(req(OpenBody)),
    )
    .await
    .expect("must not wait on body EOF")
    .expect("head");
    assert_eq!(resp.head.status, 200);
    assert!(
        started.elapsed() < Duration::from_millis(400),
        "TTFB waited on an open body: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn two_body_reads_see_separate_chunks() {
    let mut resp = load("two_reads")
        .execute(req(MemoryBody::new(b"ab".to_vec())))
        .await
        .expect("execute");
    let mut body = Vec::new();
    while let Some(chunk) = resp.body.recv().await {
        body.extend_from_slice(&chunk);
    }
    assert_eq!(body, b"ab");
}

#[tokio::test]
async fn backpressured_writes_release_within_deadline() {
    let resp = load("flood_write")
        .with_deadline(Duration::from_millis(80))
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect("head");
    let done = tokio::time::timeout(Duration::from_millis(400), resp.done)
        .await
        .expect("worker must not pin past the deadline");
    assert!(done.expect("oneshot").is_err());
}

#[tokio::test]
async fn known_oversize_body_is_rejected_without_buffering() {
    let mut head = sample_envelope();
    head.body.len = Some((MAX_REQ_BODY_BYTES as u64) + 1);
    let err = load("echo")
        .execute(ExecRequest {
            head,
            body: Box::new(MemoryBody::new(vec![0; MAX_REQ_BODY_BYTES + 1])),
        })
        .await
        .expect_err("oversize");
    assert!(
        matches!(err, ExecError::Protocol(ref k) if k.contains("body exceeds")),
        "got {err:?}"
    );
}

#[tokio::test]
async fn saturated_admission_rejects_without_queueing() {
    let exec = load("spin")
        .with_workers(1)
        .expect("workers")
        .with_deadline(Duration::from_secs(2));
    let busy = exec.clone();
    let handle = tokio::spawn(async move { busy.execute(req(MemoryBody::new(Vec::new()))).await });
    tokio::time::sleep(Duration::from_millis(30)).await;
    let err = exec
        .execute(req(MemoryBody::new(Vec::new())))
        .await
        .expect_err("second spin");
    assert!(
        matches!(err, ExecError::Saturated),
        "expected Saturated, got {err:?}"
    );
    handle.abort();
}
