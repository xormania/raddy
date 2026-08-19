use std::sync::OnceLock;
use std::time::Duration;

use bytes::Bytes;
use cucumber::{given, then, when};
use http_body_util::{BodyExt, Empty, Full};
use hyper::Request;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use raddy_executor::{
    EngineBuilder, RestoreStrategy, ToyExecutor, snap_guest_wizer, toy_guest_wasm,
};
use raddy_server::{ServerLimits, serve_tcp};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use crate::BddWorld;

fn toy(name: &str) -> ToyExecutor {
    let wasm = toy_guest_wasm(name).unwrap_or_else(|| panic!("unknown toy guest {name}"));
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade.load_wasm_bytes(wasm).expect("module");
    ToyExecutor::new(facade, module, Duration::from_secs(2)).expect("executor")
}

fn php() -> raddy_executor::PhpExecutor {
    static EXECUTOR: OnceLock<raddy_executor::PhpExecutor> = OnceLock::new();
    EXECUTOR
        .get_or_init(|| {
            raddy_executor::PhpExecutor::discover()
                .unwrap_or_else(|err| panic!("php-cgi wasm missing; run `just guest-php`: {err}"))
                .with_deadline(Duration::from_secs(10))
        })
        .clone()
}

#[given(regex = r#"^the server concurrency is (\d+)$"#)]
async fn set_concurrency(world: &mut BddWorld, n: usize) {
    world.server_concurrency = Some(n);
}

#[given(regex = r#"^a server on an ephemeral port serving the "([^"]+)" guest$"#)]
async fn start_server(world: &mut BddWorld, guest: String) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    let exec = toy(&guest);
    let (tx, rx) = oneshot::channel();
    let limits = ServerLimits {
        concurrency: world.server_concurrency.unwrap_or(32),
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

#[when(regex = r#"^I POST "([^"]+)" with body "([^"]*)"$"#)]
async fn post(world: &mut BddWorld, path: String, body: String) {
    let (status, headers, bytes) = exchange(
        world.addr(),
        "POST",
        &path,
        Some(Bytes::from(body.into_bytes())),
    )
    .await;
    world.last_status = Some(status);
    world.last_headers = Some(headers);
    world.last_body = Some(String::from_utf8_lossy(&bytes).into_owned());
}

#[when(regex = r#"^I GET "([^"]+)"$"#)]
async fn get(world: &mut BddWorld, path: String) {
    let (status, headers, bytes) = exchange(world.addr(), "GET", &path, None).await;
    world.last_status = Some(status);
    world.last_headers = Some(headers);
    world.last_body = Some(String::from_utf8_lossy(&bytes).into_owned());
}

#[when(regex = r#"^I GET "([^"]+)" without waiting for the response$"#)]
async fn get_inflight(world: &mut BddWorld, path: String) {
    let addr = world.addr();
    tokio::spawn(async move {
        let _ = exchange(addr, "GET", &path, None).await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
}

#[when(regex = r#"^I GET "([^"]+)" and read the first body byte$"#)]
async fn get_first_byte(world: &mut BddWorld, path: String) {
    let addr = world.addr();
    let stream = TcpStream::connect(addr).await.expect("connect");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .expect("handshake");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .body(Empty::<Bytes>::new())
        .expect("request");
    let res = sender.send_request(req).await.expect("send");
    world.last_status = Some(res.status().as_u16());
    let mut body = res.into_body();
    let first = next_byte(&mut body).await.expect("first byte");
    world.first_byte = Some(first);
    let mut rest = Vec::new();
    while let Some(b) = next_byte(&mut body).await {
        rest.push(b);
    }
    world.more_after_first = Some(rest);
}

#[then(regex = r#"^the HTTP status is (\d+)$"#)]
async fn http_status(world: &mut BddWorld, status: u16) {
    assert_eq!(world.last_status, Some(status));
}

#[then(regex = r#"^the HTTP body contains "([^"]+)"$"#)]
async fn http_body_contains(world: &mut BddWorld, needle: String) {
    let body = world.last_body.as_deref().expect("body");
    assert!(body.contains(&needle), "body {body:?} missing {needle:?}");
}

#[then(regex = r#"^the Retry-After header is "([^"]+)"$"#)]
async fn retry_after(world: &mut BddWorld, expected: String) {
    let headers = world.last_headers.as_ref().expect("headers");
    let got = headers
        .get(http::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .expect("Retry-After");
    assert_eq!(got, expected);
}

#[then(expr = "at least one more body byte arrives after the first")]
async fn more_bytes(world: &mut BddWorld) {
    let first = world.first_byte.expect("first byte");
    let rest = world.more_after_first.as_ref().expect("rest");
    assert!(!rest.is_empty(), "only got first byte {first}");
}

#[given(expr = "a server on an ephemeral port serving the PHP app")]
async fn start_php_server(world: &mut BddWorld) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    let exec = php();
    let (tx, rx) = oneshot::channel();
    let limits = ServerLimits {
        concurrency: world.server_concurrency.unwrap_or(32),
        request_timeout: Duration::from_secs(10),
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

#[then(regex = r#"^the HTTP body is exactly \"([^\"]*)\"$"#)]
async fn http_body_exact(world: &mut BddWorld, expected: String) {
    let body = world.last_body.as_deref().expect("body");
    assert_eq!(body, expected);
}

#[then(regex = r#"^the Set-Cookie headers are \"([^\"]+)\" then \"([^\"]+)\"$"#)]
async fn set_cookie_order(world: &mut BddWorld, first: String, second: String) {
    let headers = world.last_headers.as_ref().expect("headers");
    let got: Vec<_> = headers
        .get_all(http::header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().expect("set-cookie utf8"))
        .collect();
    assert_eq!(got, [first.as_str(), second.as_str()]);
}

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

#[given(expr = "a server on an ephemeral port serving the hello-symfony artifact")]
async fn start_snapshot_server(world: &mut BddWorld) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    let exec = snapshot_exec();
    let (tx, rx) = oneshot::channel();
    let limits = ServerLimits {
        concurrency: world.server_concurrency.unwrap_or(32),
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

#[when(regex = r#"^I GET \"([^\"]+)\" twice$"#)]
async fn get_twice(world: &mut BddWorld, path: String) {
    let (status, headers, first) = exchange(world.addr(), "GET", &path, None).await;
    let (_, _, second) = exchange(world.addr(), "GET", &path, None).await;
    world.last_status = Some(status);
    world.last_headers = Some(headers);
    world.prior_body = Some(String::from_utf8_lossy(&first).into_owned());
    world.last_body = Some(String::from_utf8_lossy(&second).into_owned());
}

#[then(expr = "the request was served by a resumed instance")]
async fn resumed_instance(world: &mut BddWorld) {
    let headers = world.last_headers.as_ref().expect("headers");
    let got = headers
        .get("x-raddy-restore")
        .and_then(|v| v.to_str().ok())
        .expect("x-raddy-restore");
    assert_eq!(got, "snapshot");
}

#[then(expr = "the two response bodies differ")]
async fn bodies_differ(world: &mut BddWorld) {
    let a = world.prior_body.as_deref().expect("first body");
    let b = world.last_body.as_deref().expect("second body");
    assert_ne!(a, b, "bodies must differ, both {a:?}");
}

#[then(expr = "REQUEST_TIME is within 5 seconds of the host clock")]
async fn request_time_near_host(world: &mut BddWorld) {
    let headers = world.last_headers.as_ref().expect("headers");
    let got: u64 = headers
        .get("x-raddy-request-time")
        .and_then(|v| v.to_str().ok())
        .expect("x-raddy-request-time")
        .parse()
        .expect("unix seconds");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let delta = now.abs_diff(got);
    assert!(delta <= 5, "REQUEST_TIME {got} is {delta}s from host {now}");
}

impl BddWorld {
    fn addr(&self) -> std::net::SocketAddr {
        self.server_addr.expect("server must be started")
    }
}

async fn exchange(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: Option<Bytes>,
) -> (u16, http::HeaderMap, Vec<u8>) {
    let stream = TcpStream::connect(addr).await.expect("connect");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .expect("handshake");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header(http::header::HOST, addr.to_string())
        .body(Full::new(body.unwrap_or_default()))
        .expect("request");
    let res = sender.send_request(req).await.expect("send");
    let status = res.status().as_u16();
    let headers = res.headers().clone();
    let bytes = res
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, headers, bytes)
}

async fn next_byte(body: &mut Incoming) -> Option<u8> {
    loop {
        match body.frame().await? {
            Ok(frame) => {
                if let Ok(data) = frame.into_data()
                    && let Some(b) = data.first().copied()
                {
                    return Some(b);
                }
            }
            Err(_) => return None,
        }
    }
}
