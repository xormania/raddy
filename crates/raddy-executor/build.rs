use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let src_dir = manifest.join("../../guest/toy");
    println!("cargo:rerun-if-changed={}", src_dir.display());
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");

    let sdk = env::var("WASI_SDK_PATH")
        .expect("WASI_SDK_PATH must point at a wasi-sdk 33 root (do not hard-code a home path)");
    let clang = Path::new(&sdk).join("bin/clang");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    for name in ["echo", "spin", "trap"] {
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
}
