use raddy_artifact::{load_artifact, parse_manifest, sha256_hex, validate_module};

const GOOD: &str = r#"
[artifact]
name = "hello"
version = "0.1.0"
abi = 1

[module]
wasm = "guest.wasm"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[app]
fs = "app.fs/"

[capabilities]
grants = ["log"]

[limits]
memory_max_mib = 512
deadline_ms = 30000
"#;

#[test]
fn good_manifest_accepts_matching_bytes() {
    let m = parse_manifest(GOOD).expect("parse");
    validate_module(&m, b"").expect("empty blob matches the well-known hash");
}

#[test]
fn wrong_hash_refuses_load() {
    let m = parse_manifest(GOOD).expect("parse");
    let err = validate_module(&m, b"not empty").expect_err("hash must refuse");
    assert!(
        err.to_string().contains("hash"),
        "expected hash mismatch, got {err}"
    );
}

#[test]
fn wrong_abi_refuses_parse() {
    let text = GOOD.replace("abi = 1", "abi = 99");
    let err = parse_manifest(&text).expect_err("abi");
    assert!(
        err.to_string().contains("abi"),
        "expected abi refusal, got {err}"
    );
}

#[test]
fn unknown_key_is_a_hard_error() {
    let text = format!("{GOOD}\nextra = true\n");
    let err = parse_manifest(&text).expect_err("unknown");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown") || msg.contains("extra"),
        "expected unknown-key error, got {msg}"
    );
}

#[test]
fn hash_helper_matches_pin_shape() {
    let hex = sha256_hex(b"raddy");
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn matching_precompiled_module_is_selected_and_hash_checked() {
    let root = std::env::temp_dir().join(format!(
        "raddy-artifact-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("fixture directory");
    let wasm = b"fallback";
    let cwasm = b"precompiled";
    std::fs::write(root.join("guest.wasm"), wasm).expect("wasm fixture");
    std::fs::write(root.join("guest.cwasm"), cwasm).expect("cwasm fixture");
    std::fs::write(
        root.join("raddy.artifact.toml"),
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
memory_max_mib = 512
deadline_ms = 30000

[precompiled.x86_64-unknown-linux-gnu]
cwasm = "guest.cwasm"
wasmtime = "47.0.3"
sha256 = "{}"
"#,
            sha256_hex(wasm),
            sha256_hex(cwasm)
        ),
    )
    .expect("manifest fixture");

    let loaded =
        load_artifact(&root, "x86_64-unknown-linux-gnu", "47.0.3").expect("matching cwasm");
    assert_eq!(loaded.manifest().artifact.name, "hello");

    std::fs::write(root.join("guest.wasm"), b"corrupt").expect("corrupt wasm fixture");
    let err = load_artifact(&root, "x86_64-unknown-linux-gnu", "47.0.3")
        .expect_err("raw wasm hash mismatch");
    assert!(err.to_string().contains("hash"), "unexpected error: {err}");

    std::fs::write(root.join("guest.wasm"), wasm).expect("restore wasm fixture");
    std::fs::write(root.join("guest.cwasm"), b"corrupt").expect("corrupt fixture");
    let err = load_artifact(&root, "x86_64-unknown-linux-gnu", "47.0.3")
        .expect_err("cwasm hash mismatch");
    assert!(err.to_string().contains("hash"), "unexpected error: {err}");
    std::fs::remove_dir_all(root).expect("remove fixture");
}
