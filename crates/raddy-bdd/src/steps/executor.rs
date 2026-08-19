use std::time::Duration;

use cucumber::{given, then, when};
use raddy_abi::{BodyMeta, Envelope, RequestId};
use raddy_executor::{
    EngineBuilder, ExecError, ExecRequest, Executor, MemoryBody, ToyExecutor, toy_guest_wasm,
};

use crate::BddWorld;

#[given(regex = r#"^the toy "([^"]+)" guest is loaded$"#)]
async fn load_toy(world: &mut BddWorld, name: String) {
    let wasm = toy_guest_wasm(&name).unwrap_or_else(|| panic!("unknown toy guest {name}"));
    let facade = EngineBuilder::new()
        .epoch_tick(Duration::from_millis(10))
        .build()
        .expect("engine");
    let module = facade.load_wasm_bytes(wasm).expect("module");
    world.executor = Some(ToyExecutor::new(facade, module, Duration::from_secs(2)).expect("exec"));
}

#[given(regex = r#"^the executor deadline is (\d+) ms$"#)]
async fn set_deadline(world: &mut BddWorld, ms: u64) {
    let exec = world
        .executor
        .take()
        .expect("a toy guest must be loaded before setting a deadline");
    world.executor = Some(exec.with_deadline(Duration::from_millis(ms)));
}

#[when(regex = r#"^I execute a GET "([^"]+)" with body "([^"]*)"$"#)]
async fn execute_get(world: &mut BddWorld, target: String, body: String) {
    let exec = world
        .executor
        .as_ref()
        .expect("a toy guest must be loaded before execute");
    let req = ExecRequest {
        head: Envelope {
            v: raddy_abi::ABI_VERSION,
            request_id: RequestId::from_u128(1),
            method: "GET".into(),
            target,
            scheme: "http".into(),
            authority: "localhost:8080".into(),
            headers: Vec::new(),
            remote_addr: "127.0.0.1:1".parse().expect("addr"),
            body: BodyMeta {
                len: Some(body.len() as u64),
            },
            deadline_ms: 30_000,
        },
        body: Box::new(MemoryBody::new(body.into_bytes())),
    };
    match exec.execute(req).await {
        Ok(mut resp) => {
            let mut collected = Vec::new();
            while let Some(chunk) = resp.body.recv().await {
                collected.extend_from_slice(&chunk);
            }
            world.last_status = Some(resp.head.status);
            world.last_body = Some(String::from_utf8_lossy(&collected).into_owned());
            world.last_error = None;
        }
        Err(ExecError::DeadlinePreHead) => {
            world.last_error = Some("DeadlinePreHead".into());
            world.last_status = None;
            world.last_body = None;
        }
        Err(ExecError::Trap(_)) => {
            world.last_error = Some("Trap".into());
            world.last_status = None;
            world.last_body = None;
        }
        Err(err) => panic!("unexpected execute error: {err}"),
    }
}

#[then(expr = "the execute result is a response")]
async fn is_response(world: &mut BddWorld) {
    assert!(
        world.last_status.is_some(),
        "expected a response, got error {:?}",
        world.last_error
    );
}

#[then(regex = r#"^the response status is (\d+)$"#)]
async fn status_is(world: &mut BddWorld, status: u16) {
    assert_eq!(world.last_status, Some(status));
}

#[then(regex = r#"^the response body contains "([^"]+)"$"#)]
async fn body_contains(world: &mut BddWorld, needle: String) {
    let body = world.last_body.as_deref().expect("response body");
    assert!(
        body.contains(&needle),
        "body {body:?} does not contain {needle:?}"
    );
}

#[then(expr = "the execute result is DeadlinePreHead")]
async fn is_deadline(world: &mut BddWorld) {
    assert_eq!(world.last_error.as_deref(), Some("DeadlinePreHead"));
}

#[then(expr = "the execute result is a trap")]
async fn is_trap(world: &mut BddWorld) {
    assert_eq!(world.last_error.as_deref(), Some("Trap"));
}
