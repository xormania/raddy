# Offline cargo before the first fetch

**Class:** a command is told not to use the network while the thing it needs has never been retrieved.

**Where it occurs:** `cargo test --offline`, `cargo build --offline`, any tool with an offline flag, on a lockfile whose crates are not in the local cache.

**What you see:** `attempting to make an HTTP request, but --offline was specified` while downloading a crate.

**Covered by:** first fetch is online (`cargo test` / `cargo fetch`). `--offline` is only valid after that. Not a product test.

**Shape that fits the rest:** network is a precondition of the first resolve, not an optimization to strip by default.
