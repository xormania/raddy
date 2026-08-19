# Changelog

## Unreleased

Stage 5 fast path: warm pool, detached teardown, cwasm serialize/deserialize,
mimalloc on the binary, `just bench` / `just bench-check`. `@stage-5`.

Stage 4 snapshot resume: Wizer 11 over an ABI guest, `RestoreStrategy::Snapshot`,
`raddy-artifact` manifests. `@stage-4` hello / entropy / REQUEST_TIME.
Symfony-from-snapshot waits on the raddy SAPI (ADR 0009).

Stage 3 PHP cold: `CgiExecutor` runs stock WLR `php-cgi-8.2.6.wasm`
(`just guest-php`, ADR 0008). `@stage-3` hello, POST echo, Set-Cookie.
Custom raddy SAPI is not this stage.

Stage 2 HTTP ingress: `raddy-server` on hyper 1.11, envelope mapping, 503
on saturation, streamed TTFB. `@stage-2` scenarios. No GitHub tag.

Audit of `origin/dev` (`proj/crawl/audit.md`): typestate is on the real
executor path, guest lengths are bounded before allocate, request bodies
stream, missing `resp_end` / bad heads / non-zero exits are Protocol,
unknown `RADDY_*` keys fail, wasi-sdk 33 is identified, `toml` and
`ExecResponse.done` have ADRs. Do not treat the GitHub `stage-1` tag as
acceptance (ADR 0003).

## stage-1 — Executor on a toy guest

`raddy-abi` JSON v1 codecs, `raddy-executor` on wasmtime 47.0.3 (pooling, CoW,
epoch ticker, `InstancePre`), and C toy guests (`echo`, `spin`, `trap`) compiled
with wasi-sdk 33. `@stage-1` scenarios run at the executor port. wasmtime 48 LTS
was not published; the pin stays 47.0.3.

## stage-0 — Bedrock

Workspace, `just check`, `raddy-config` layering (defaults → TOML → `RADDY_*` → CLI),
`raddy --print-config`, cucumber harness, and the `@stage-0` env-override scenario.
Unknown keys are hard errors. `env:NAME` strings resolve through a secret lookup.
