# Workspace test run hides later crates

**Class:** a runner stops at the first failing package, so the rest of the suite is never observed.

**Where it occurs:** `cargo test --workspace` when one harness (`raddy-bdd`) exits non-zero; CI matrices that fail-fast; any `&&` chain of package tests.

**What you see:** BDD is red and `raddy-config` tests never print. The later failures are still there.

**Covered by:** when a workspace run fails, re-run the other packages before concluding what is red. `just check` still fail-fast, which is correct for the gate.

**Shape that fits the rest:** a failed run is a pointer, not a complete list. Confirm the class of failures, not the first name printed.
