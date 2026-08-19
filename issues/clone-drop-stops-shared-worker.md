# Clone drop stops a shared worker

**Class:** a `Clone` handle around `Arc<Inner>` implements `Drop` that stops
a background thread, so dropping one clone kills the worker for every handle.

**Where it occurs:** any cloneable owner of a refill/ticker thread. Here,
`InstancePool`.

**What you see:** idle count stays 0 after spent slots; refill never runs.

**Covered by:** `Drop` only signals stop when `Arc::strong_count == 1`.

**Shape that fits the rest:** cloneable handles do not shut down shared
workers until the last handle is gone.
