# Workspace forbid blocks a stage-required unsafe

**Class:** a workspace lint is `forbid`, so a later stage that the plan requires (`unsafe` for `Module::deserialize_file`) cannot even `#[allow]` it.

**Where it occurs:** any workspace-wide `forbid`; here `unsafe_code`. Same class as forbidding `unwrap` then needing it in a test harness.

**What you see:** `unsafe is forbidden` on the one call the plan names, and `allow` does not compile.

**Covered by:** workspace lint is `deny`. The deserialize wrapper is the only `#[allow(unsafe_code)]`.

**Shape that fits the rest:** `forbid` is for things that must never appear. Stage-required operations are `deny` plus a local allow that names the invariant.
