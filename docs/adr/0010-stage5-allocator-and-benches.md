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
- Artifact `just artifact` emits a cwasm via `wasmtime compile` and records
  `[precompiled]` in the manifest. In-process tests use
  `serialize_module` / `load_cwasm_bytes`.

## Consequences

`mimalloc` 0.1.x is a host-only dependency. Bench numbers are machine-local;
the committed baseline is the ratio record from the first green Stage 5 run.
