use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use raddy_config::Config;
use raddy_executor::{EngineBuilder, snap_guest_wizer};

struct FixtureDir(PathBuf);

impl FixtureDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "raddy-runtime-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).expect("fixture directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove fixture directory");
    }
}

#[test]
fn production_builder_uses_verified_cwasm_and_configured_pool() {
    let fixture = FixtureDir::new();
    let facade = EngineBuilder::new().build().expect("fixture engine");
    let module = facade
        .load_wasm_bytes(snap_guest_wizer())
        .expect("fixture module");
    let cwasm = facade.serialize_module(&module).expect("serialize cwasm");
    let invalid_wasm = b"this is deliberately not a Wasm module";
    std::fs::write(fixture.path().join("guest.wasm"), invalid_wasm).expect("wasm fixture");
    std::fs::write(fixture.path().join("guest.cwasm"), &cwasm).expect("cwasm fixture");
    std::fs::write(
        fixture.path().join("raddy.artifact.toml"),
        format!(
            r#"[artifact]
name = "hello"
version = "0.1.0"
abi = 1

[module]
wasm = "guest.wasm"
sha256 = "{}"

[app]
fs = "app.fs/"

[limits]
memory_max_mib = 64
deadline_ms = 30000

[precompiled.{}]
cwasm = "guest.cwasm"
wasmtime = "{}"
sha256 = "{}"
"#,
            raddy_artifact::sha256_hex(invalid_wasm),
            raddy_executor::host_target(),
            raddy_executor::WASMTIME_VERSION,
            raddy_artifact::sha256_hex(&cwasm)
        ),
    )
    .expect("manifest fixture");

    let mut cfg = Config::default();
    cfg.executor.artifact = fixture.path().to_path_buf();
    cfg.executor.pool_min = 2;
    cfg.executor.pool_max = 3;
    let exec = raddy::build_executor(&cfg).expect("production executor");

    assert_eq!(exec.pool().min(), 2);
    assert_eq!(exec.pool().max(), 3);
    assert_eq!(exec.pool().idle_count(), 2);
}
