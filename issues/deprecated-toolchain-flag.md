# Deprecated toolchain flag still accepted

**Class:** a vendor flag still works but warns, and copying the warning into a tracked recipe freezes a deprecation.

**Where it occurs:** wasi-sdk 33 `clang --target=wasm32-wasi` (use `wasm32-wasip1`); any SDK that renamed a triple or option.

**What you see:** `argument '--target=wasm32-wasi' is deprecated`. The wasm still builds.

**Covered by:** `guest/toy` and `build.rs` use `wasm32-wasip1`.

**Shape that fits the rest:** the compiler's current flag is the contract. Do not copy a working-but-deprecated invocation into the tree.
