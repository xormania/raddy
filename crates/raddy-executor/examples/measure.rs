//! Stage 5 measurements: resume, e2e hello warm, TTFB. Relative, not wall-clock claims.

use std::time::{Duration, Instant};

use raddy_abi::{BodyMeta, Envelope, RequestId};
use raddy_executor::{
    EngineBuilder, ExecRequest, Executor, MemoryBody, RestoreStrategy, ToyExecutor,
    snap_guest_wizer,
};

fn envelope() -> Envelope {
    Envelope {
        v: raddy_abi::ABI_VERSION,
        request_id: RequestId::from_u128(1),
        method: "GET".into(),
        target: "/hello?name=xor".into(),
        scheme: "http".into(),
        authority: "localhost:8080".into(),
        headers: Vec::new(),
        remote_addr: "127.0.0.1:1".parse().expect("addr"),
        body: BodyMeta { len: Some(0) },
        deadline_ms: 30_000,
    }
}

fn exec() -> ToyExecutor {
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
    .with_pool(2, 4)
    .expect("pool")
}

fn median(mut xs: Vec<u128>) -> u128 {
    xs.sort_unstable();
    xs[xs.len() / 2]
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("rt");
    let n = 21_u32;
    let toy = exec();
    let resume_toy = exec().with_pool(0, 1).expect("resume pool");

    let mut resume = Vec::new();
    for _ in 0..n {
        let t0 = Instant::now();
        let mut resumed = resume_toy.pool().steal().expect("resume snapshot");
        resume.push(t0.elapsed().as_nanos());
        resumed.mark_spent();
    }

    let mut e2e = Vec::new();
    let mut ttfb = Vec::new();
    rt.block_on(async {
        for _ in 0..n {
            let t0 = Instant::now();
            let mut resp = toy
                .execute(ExecRequest {
                    head: envelope(),
                    body: Box::new(MemoryBody::new(Vec::new())),
                })
                .await
                .expect("execute");
            ttfb.push(t0.elapsed().as_nanos());
            while resp.body.recv().await.is_some() {}
            let _ = resp.response_complete.send(());
            let teardown = resp.teardown;
            resp.done.await.expect("done").expect("clean");
            e2e.push(t0.elapsed().as_nanos());
            teardown.await.expect("teardown");
        }
    });

    println!(
        "{{\n  \"resume_p50_ns\": {},\n  \"e2e_hello_warm_p50_ns\": {},\n  \"ttfb_p50_ns\": {}\n}}",
        median(resume),
        median(e2e),
        median(ttfb)
    );
}
