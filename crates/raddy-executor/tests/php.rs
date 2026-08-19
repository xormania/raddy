use std::time::Duration;

use raddy_abi::{BodyMeta, Envelope, RequestId};
use raddy_executor::{ExecRequest, Executor, MemoryBody, PhpExecutor};

fn envelope(method: &str, target: &str, body_len: Option<u64>) -> Envelope {
    Envelope {
        v: raddy_abi::ABI_VERSION,
        request_id: RequestId::from_u128(1),
        method: method.into(),
        target: target.into(),
        scheme: "http".into(),
        authority: "localhost:8080".into(),
        headers: Vec::new(),
        remote_addr: "127.0.0.1:1".parse().expect("addr"),
        body: BodyMeta { len: body_len },
        deadline_ms: 30_000,
    }
}

fn load() -> PhpExecutor {
    PhpExecutor::discover()
        .unwrap_or_else(|err| panic!("php-cgi wasm missing; run `just guest-php`: {err}"))
        .with_deadline(Duration::from_secs(10))
}

async fn collect(exec: &PhpExecutor, method: &str, target: &str, body: Vec<u8>) -> (u16, Vec<u8>) {
    let len = body.len() as u64;
    let mut resp = exec
        .execute(ExecRequest {
            head: envelope(method, target, Some(len)),
            body: Box::new(MemoryBody::new(body)),
        })
        .await
        .expect("execute");
    let status = resp.head.status;
    let mut out = Vec::new();
    while let Some(chunk) = resp.body.recv().await {
        out.extend_from_slice(&chunk);
    }
    resp.done.await.expect("done").expect("clean end");
    (status, out)
}

#[tokio::test]
async fn php_hello_is_plain_text() {
    let (status, body) = collect(&load(), "GET", "/hello", Vec::new()).await;
    assert_eq!(status, 200);
    assert_eq!(body, b"Hello");
}

#[tokio::test]
async fn php_echo_is_bytes_exact() {
    let payload = b"hello\x00world".to_vec();
    let (status, body) = collect(&load(), "POST", "/echo", payload.clone()).await;
    assert_eq!(status, 200);
    assert_eq!(body, payload);
}

#[tokio::test]
async fn php_cookies_preserve_order() {
    let exec = load();
    let mut resp = exec
        .execute(ExecRequest {
            head: envelope("GET", "/cookies", Some(0)),
            body: Box::new(MemoryBody::new(Vec::new())),
        })
        .await
        .expect("execute");
    let cookies: Vec<_> = resp
        .head
        .headers
        .iter()
        .filter(|(n, _)| n == "set-cookie")
        .map(|(_, v)| v.as_str())
        .collect();
    assert_eq!(cookies, ["a=1", "b=2"]);
    while resp.body.recv().await.is_some() {}
    resp.done.await.expect("done").expect("clean end");
}
