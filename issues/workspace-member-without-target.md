# Workspace member without a target

**Class:** a package manifest is added to a workspace before it has a `src` target.

**Where it occurs:** any `cargo add`, `cargo metadata`, or workspace load against a new crate; not only the first crate in a workspace.

**What you see:** `no targets specified in the manifest` / `either src/lib.rs, src/main.rs, a [lib] section, or [[bin]] section must be present`.

**Covered by:** create `src/lib.rs` or `src/main.rs` before the crate is listed as a member and before `cargo add` runs. This file is the record; there is no automatic check.

**Shape that fits the rest:** a crate becomes a workspace member only after it has a target file on disk.
