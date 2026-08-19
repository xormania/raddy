# Changelog

## stage-1 — Executor on a toy guest

`raddy-abi` JSON v1 codecs, `raddy-executor` on wasmtime 47.0.3 (pooling, CoW,
epoch ticker, `InstancePre`), and C toy guests (`echo`, `spin`, `trap`) compiled
with wasi-sdk 33. `@stage-1` scenarios run at the executor port. wasmtime 48 LTS
was not published; the pin stays 47.0.3.

## stage-0 — Bedrock

Workspace, `just check`, `raddy-config` layering (defaults → TOML → `RADDY_*` → CLI),
`raddy --print-config`, cucumber harness, and the `@stage-0` env-override scenario.
Unknown keys are hard errors. `env:NAME` strings resolve through a secret lookup.
