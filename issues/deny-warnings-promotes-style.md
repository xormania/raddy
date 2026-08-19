# Deny-warnings promotes style lints to blockers

**Class:** a gate that denies all warnings makes every clippy style lint a failed build, including ones that appeared only after a real fix.

**Where it occurs:** `just check` → `clippy -- -D warnings`; rustc `missing_debug_implementations`; any `-D warnings` CI. Instances here: `result_large_err`, `derivable_impls`, `collapsible_if`.

**What you see:** tests are green, `cargo clippy -D warnings` is red, on a lint that does not change behavior.

**Covered by:** the gate itself. Fix the lint; do not allow it unless the allow names a real constraint.

**Shape that fits the rest:** treat the gate output as the list of remaining work, not as optional advice.
