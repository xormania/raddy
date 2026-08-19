# Clone drop stops a shared worker

**Class:** a `Clone` handle around `Arc<Inner>` implements `Drop` that stops
a background thread, so dropping one clone kills the worker for every handle.

**Where it occurs:** any cloneable owner of a refill/ticker thread. Here,
`InstancePool`.

**What you see:** idle count stays 0 after spent slots; refill never runs.

**Covered by:** the refill worker holds only a `Weak<PoolInner>`; the unit test
proves the final owner can disappear while that worker is running.

**Shape that fits the rest:** cloneable handles keep the pool alive; the worker
exits once the last real owner is gone.
