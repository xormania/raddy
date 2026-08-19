# Error Display omits the cause that classifies it

**Class:** a wrapper's `Display` is a backtrace or context string, and the actual discriminant (`Trap::Interrupt`) lives in the source chain.

**Where it occurs:** wasmtime epoch traps (`"interrupt"` / `Trap::Interrupt`); any `anyhow`/`thiserror` wrap of a typed trap; matching `err.to_string().contains("epoch")` against a message that never said "epoch".

**What you see:** `Trap("error while executing at wasm backtrace: ... raddy_execute")` for a deadline. The guest was interrupted; the string lied.

**Covered by:** walk `Error::chain` and downcast to `wasmtime::Trap`. Do not classify traps by Display.

**Shape that fits the rest:** classify on the typed cause. Display is for humans.
