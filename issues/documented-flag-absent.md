# Documented invocation flag the binary does not implement

**Class:** a version or help string is treated as a stable CLI contract when the binary never defined that flag.

**Where it occurs:** any "run `--version` and assert" check; the plan names `wizer --version`. Same class as assuming `tool -V` exists.

**What you see:** `error: unexpected argument '--version' found` / `unexpected argument '-V' found`. The tool still works.

**Covered by:** nothing automatic. Probe `--help` before treating a flag as a gate. Record the real invocation in the stage that needs the tool.

**Shape that fits the rest:** the binary is the contract. A sentence in a plan is a claim, not an interface.
