# Shared prefix on non-config environment keys

**Class:** a product prefix is reused by harness or ops variables, then "unknown key" treats every match as config.

**Where it occurs:** `RADDY_*` config overlay vs `RADDY_BDD_STAGE`; any `APP_*` scheme that later grows test/CI keys.

**What you see:** a legitimate harness variable fails `raddy --print-config` with `unknown configuration key`.

**Covered by:** `parse_raddy_key` only accepts keys under known sections (`server_`, `admin_`, `executor_`, `observability_`, `db_`). Other `RADDY_*` names are ignored.

**Shape that fits the rest:** a prefix is not a schema. Unknown-key rejection applies to config-shaped keys, not to the entire prefix.
