# A failing test writes a side file

**Class:** observing red is not a read-only act; the test framework persists the failure.

**Where it occurs:** proptest `FileFailurePersistence` writes `*.proptest-regressions` next to the test; snapshot testers write `.snap.new`; cucumber junit writers; anything that records the first failure.

**What you see:** after a deliberate red run, an untracked regressions file appears. Committing it freezes the stub's failure as if it were a real input.

**Covered by:** delete the file after the implementation is green, before commit. Do not treat the file as part of the spec.

**Shape that fits the rest:** a red run must not leave durable product state. Persistence belongs to green cases you chose to keep.
