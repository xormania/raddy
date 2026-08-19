# Assumed crate API from memory

**Class:** a constructor or method name is taken from an older major of a crate, or from training data, instead of from the crate that `cargo add` just resolved.

**Where it occurs:** any "latest at init" dependency; here `ulid` 3.0.0 (`Ulid::generate`, not `Ulid::new`). Same class as figment/wasmtime/thiserror breaking changes.

**What you see:** `no associated function named X found` with a note listing the real names.

**Covered by:** `cargo add` then compile. Read the crate that is actually in the registry, not the API you remember.

**Shape that fits the rest:** the lockfile is the pin. The source in `~/.cargo/registry` is the API.
