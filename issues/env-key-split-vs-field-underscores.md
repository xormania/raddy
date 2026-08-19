# Env key split vs underscored field names

**Class:** a nested-key decoder splits on the same character the leaves are allowed to contain.

**Where it occurs:** `RADDY_*` overlay (`request_timeout_ms`, `pool_min`, `log_format`, `db.<name>.pool_max`); any `PREFIX_SECTION_FIELD` scheme; figment `Env::split("_")`.

**What you see:** `RADDY_SERVER_REQUEST_TIMEOUT_MS` becomes `server.request.timeout.ms` instead of `server.request_timeout_ms`. The load either ignores the value or fails extract.

**Covered by:** `parse_raddy_key` in `crates/raddy-config/src/load.rs` matches a known section prefix, then the remainder as the field name. The layering proptest uses `RADDY_SERVER_LISTEN` and `RADDY_SERVER_CONCURRENCY`.

**Shape that fits the rest:** the schema owns the key grammar. Do not split blindly.
