# Large foreign error in Result

**Class:** a third-party error type is stored inline in a public `Result`, and clippy `-D warnings` rejects the size on every function that returns it.

**Where it occurs:** `figment::Error` inside `ConfigError::Extract`; any `#[from]` of a fat error (figment, sqlx, wasmtime) on a crate-wide error enum.

**What you see:** `clippy::result_large_err` on every `Result<_, ConfigError>` in the crate, not just the extract site.

**Covered by:** box the fat variant. The lint is the check.

**Shape that fits the rest:** foreign errors that we do not construct field-by-field go in a `Box`. Our own small variants stay inline.
