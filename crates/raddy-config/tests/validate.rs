use std::collections::BTreeMap;

use raddy_config::{FileSource, LoadRequest, load};

#[test]
fn unknown_key_is_rejected() {
    let planted = "nope";
    let body = format!("[server]\nlisten = \"127.0.0.1:1\"\n{planted} = true\n");
    assert!(
        body.contains("nope = true"),
        "plant must land in the fixture body"
    );

    let err = load(LoadRequest {
        file: FileSource::Toml(body),
        ..LoadRequest::default()
    })
    .expect_err("an unknown key must be a hard error");
    let msg = err.to_string();
    assert!(
        msg.contains(planted),
        "error must name the unknown key, got {msg}"
    );
}

#[test]
fn env_indirection_resolves_string_values() {
    let var = "RADDY_TEST_DB_URL";
    let resolved = "sqlite://fixture.db";
    let body = format!("[db.main]\ndriver = \"sqlite\"\nurl = \"env:{var}\"\npool_max = 4\n");
    assert!(
        body.contains(&format!("env:{var}")),
        "plant must land in the fixture body"
    );

    let mut secrets = BTreeMap::new();
    secrets.insert(var.to_string(), resolved.to_string());
    assert_eq!(
        secrets.get(var).map(String::as_str),
        Some(resolved),
        "secret lookup plant must be present before load"
    );

    let cfg = load(LoadRequest {
        file: FileSource::Toml(body),
        secrets,
        ..LoadRequest::default()
    })
    .expect("env: indirection with a present secret must load");
    assert_eq!(cfg.db["main"].url, resolved);
}
