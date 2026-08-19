use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "src/guest_sdk.rs"]
mod guest_sdk;

const GUESTS: &[&str] = &[
    "echo",
    "spin",
    "trap",
    "no_end",
    "nonzero",
    "bad_head",
    "huge_len",
    "oob_write",
    "no_body_read",
    "two_reads",
    "flood_write",
    "slow_echo",
];

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let src_dir = manifest.join("../../guest/toy");
    println!("cargo:rerun-if-changed={}", src_dir.display());
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");

    let sdk = env::var("WASI_SDK_PATH")
        .expect("WASI_SDK_PATH must point at a wasi-sdk 33 root (do not hard-code a home path)");
    let clang = Path::new(&sdk).join("bin/clang");
    let version = Command::new(&clang)
        .arg("--version")
        .output()
        .unwrap_or_else(|err| panic!("failed to spawn {}: {err}", clang.display()));
    let banner = String::from_utf8_lossy(&version.stdout);
    assert!(
        guest_sdk::is_wasi_sdk_33(&banner),
        "WASI_SDK_PATH must be wasi-sdk 33; clang --version was:\n{banner}"
    );

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    for name in GUESTS {
        let src = src_dir.join(format!("{name}.c"));
        let dest = out.join(format!("{name}.wasm"));
        let status = Command::new(&clang)
            .args([
                "--target=wasm32-wasip1",
                "-nostdlib",
                "-Wl,--no-entry",
                "-Wl,--export=raddy_execute",
                "-Wl,--export-memory",
                "-Wl,--allow-undefined",
                "-O2",
                "-o",
            ])
            .arg(&dest)
            .arg(&src)
            .status()
            .unwrap_or_else(|err| panic!("failed to spawn {}: {err}", clang.display()));
        assert!(
            status.success(),
            "clang failed building {} from {}",
            dest.display(),
            src.display()
        );
    }
    build_hello_symfony(&clang, &out, &manifest);
}

fn build_hello_symfony(clang: &Path, out: &Path, manifest: &Path) {
    let src = manifest.join("../../guest/apps/hello-symfony/guest.c");
    println!("cargo:rerun-if-changed={}", src.display());
    println!("cargo:rerun-if-env-changed=WIZER");
    let raw = out.join("hello_symfony_raw.wasm");
    let status = Command::new(clang)
        .args([
            "--target=wasm32-wasip1",
            "-nostdlib",
            "-Wl,--no-entry",
            "-Wl,--export=raddy_execute",
            "-Wl,--export-memory",
            "-Wl,--allow-undefined",
            "-fno-builtin",
            "-O2",
            "-o",
        ])
        .arg(&raw)
        .arg(&src)
        .status()
        .unwrap_or_else(|err| panic!("clang snap guest: {err}"));
    assert!(status.success(), "clang failed building {}", raw.display());

    let wizer = env::var("WIZER").unwrap_or_else(|_| "wizer".into());
    let snap = out.join("hello_symfony_wizer.wasm");
    let status = Command::new(&wizer)
        .args(["-f", "wizer.initialize", "-o"])
        .arg(&snap)
        .arg(&raw)
        .status()
        .unwrap_or_else(|err| panic!("WIZER must be the Wizer 11 CLI (set WIZER or PATH): {err}"));
    assert!(
        status.success(),
        "wizer failed on {} (need Wizer 11.0.3)",
        raw.display()
    );
}
