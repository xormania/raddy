use std::time::{Duration, SystemTime, UNIX_EPOCH};

use raddy_abi::{BodyMeta, Envelope, RequestId};
use raddy_artifact::{parse_manifest, sha256_hex, validate_module, wizer_file};
use raddy_executor::{
    ExecRequest, Executor, MemoryBody, RestoreStrategy, ToyExecutor, snap_guest_raw,
    snap_guest_wizer,
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

fn load_snapshot() -> ToyExecutor {
    let facade = raddy_executor::EngineBuilder::new()
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

async fn collect(exec: &ToyExecutor, target: &str) -> (u16, Vec<(String, String)>, Vec<u8>) {
    let mut resp = exec
        .execute(ExecRequest {
            head: envelope(target),
            body: Box::new(MemoryBody::new(Vec::new())),
        })
        .await
        .expect("execute");
    let status = resp.head.status;
    let headers = resp.head.headers.clone();
    let mut body = Vec::new();
    while let Some(chunk) = resp.body.recv().await {
        body.extend_from_slice(&chunk);
    }
    resp.done.await.expect("done").expect("clean end");
    (status, headers, body)
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> &'a str {
    headers
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
        .unwrap_or("")
}

#[tokio::test]
async fn snapshot_hello_names_the_query() {
    let (status, headers, body) = collect(&load_snapshot(), "/hello?name=xor").await;
    assert_eq!(status, 200);
    assert_eq!(body, b"Hello xor");
    assert_eq!(header(&headers, "x-raddy-restore"), "snapshot");
}

#[tokio::test]
async fn snapshot_entropy_differs_across_requests() {
    let exec = load_snapshot();
    let (_, _, a) = collect(&exec, "/random").await;
    let (_, _, b) = collect(&exec, "/random").await;
    assert_ne!(a, b, "WASI random_get must be re-read per request");
}

#[tokio::test]
async fn snapshot_request_time_tracks_host() {
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let (_, headers, _) = collect(&load_snapshot(), "/hello?name=xor").await;
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let got: u64 = header(&headers, "x-raddy-request-time")
        .parse()
        .expect("unix seconds");
    assert!(
        got >= before.saturating_sub(5) && got <= after + 5,
        "REQUEST_TIME {got} not in {before}..={after}"
    );
}

#[test]
fn two_wizer_builds_have_identical_hash() {
    let dir = std::env::temp_dir().join(format!("raddy-wizer-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmpdir");
    let raw = dir.join("raw.wasm");
    std::fs::write(&raw, snap_guest_raw()).expect("write raw");
    let a = dir.join("a.wasm");
    let b = dir.join("b.wasm");
    wizer_file(&raw, &a).expect("wizer a");
    wizer_file(&raw, &b).expect("wizer b");
    let ha = sha256_hex(&std::fs::read(&a).expect("read a"));
    let hb = sha256_hex(&std::fs::read(&b).expect("read b"));
    assert_eq!(ha, hb, "hermetic initialize must be deterministic");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn manifest_refuses_tampered_module() {
    let hash = sha256_hex(snap_guest_wizer());
    let toml = format!(
        r#"
[artifact]
name = "hello-symfony"
version = "0.1.0"
abi = 1

[module]
wasm = "guest.wasm"
sha256 = "{hash}"

[app]
fs = "app.fs/"

[limits]
memory_max_mib = 64
deadline_ms = 30000
"#
    );
    let m = parse_manifest(&toml).expect("parse");
    validate_module(&m, snap_guest_wizer()).expect("pin matches");
    let err = validate_module(&m, snap_guest_raw()).expect_err("raw is not the pin");
    assert!(err.to_string().contains("hash"));
}
