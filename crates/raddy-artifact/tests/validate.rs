use raddy_artifact::{parse_manifest, sha256_hex, validate_module};

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
