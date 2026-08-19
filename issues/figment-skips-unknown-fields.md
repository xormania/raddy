# Figment extract skips unknown fields

**Class:** a layered decoder looks up known keys and never walks leftovers, so `deny_unknown_fields` is a no-op.

**Where it occurs:** figment `extract` into a serde struct; any "merge then deserialize" stack that does not iterate unused keys. The file layer is the tested instance.

**What you see:** a TOML key such as `server.nope` loads as success. The plan requires unknown keys to be hard errors.

**Covered by:** `reject_unknown_toml` parses the raw file with `toml` + `#[serde(deny_unknown_fields)]` before figment merges it. Unit test `unknown_key_is_rejected`.

**Shape that fits the rest:** validate each overlay against the schema, then layer values. Do not trust the merger to reject extras.
