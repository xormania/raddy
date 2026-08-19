/// True when `clang --version` text is the pinned wasi-sdk 33 toolchain.
#[must_use]
pub fn is_wasi_sdk_33(clang_version: &str) -> bool {
    clang_version.contains("wasi-sdk")
}

#[cfg(test)]
mod tests {
    use super::is_wasi_sdk_33;

    #[test]
    fn accepts_wasi_sdk_banner() {
        assert!(is_wasi_sdk_33(
            "clang version 22.1.0-wasi-sdk (https://github.com/llvm/llvm-project)"
        ));
    }

    #[test]
    fn rejects_host_clang() {
        assert!(!is_wasi_sdk_33("Ubuntu clang version 18.1.3"));
    }
}
