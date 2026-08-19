# 10. Stage 5 allocator and bench gate

Date: 2026-08-19

## Status

Accepted

## Context

§3.10 names `mimalloc` as the host global allocator. The pin table does not
list it. Stage 5 also wants criterion benches and `just bench-check` against
`bench/baselines.json`. Criterion is not in the pin table. Absolute
nanoseconds are forbidden as test assertions (§5.6).

`deserialize_file` is required as the default artifact load when a cwasm pin
matches this wasmtime version. That is `unsafe` and already allowed at the
engine facade.

## Decision

- `mimalloc` is the process allocator in the `raddy` binary (`cargo add`).
- Measurements live in `raddy-executor`'s `measure` example (resume, warm
  e2e hello, TTFB). `just bench` prints JSON; `just bench-check` compares
  p50 ratios to `bench/baselines.json` and flags a >10% regression.
- `just check` does not run benches. A flagged `bench-check` is explained in
  the commit or this ADR; it does not fail the compile gate.
- Artifact `just artifact` emits a cwasm via `wasmtime compile` with the host
  engine's epoch, pooling, and CoW settings and records `[precompiled]` in the
  manifest. The production loader verifies the raw module hash, then verifies
  the selected cwasm's version and hash on an open file before Wasmtime
  deserializes that same file.
- The original `resume_p50_ns = 57` record only popped and returned an already
  idle slot. It never instantiated a snapshot, so it did not measure resume.
  The corrected loop starts with an empty pool and marks every minted slot
  spent. Its first 21-iteration median record is 1,142 ns. The accompanying
  response-lifecycle correction changed the first recorded warm e2e/TTFB from
  51,502/51,153 ns to 113,121/78,057 ns. These are an explicit one-time
  methodology/ownership baseline reset, not evidence that the old path stayed
  within 10%.

## Consequences

`mimalloc` 0.1.x is a host-only dependency. Bench numbers are machine-local;
future comparisons use the corrected committed record. The invalid 57 ns
resume number remains here so the reset is auditable instead of erased.
