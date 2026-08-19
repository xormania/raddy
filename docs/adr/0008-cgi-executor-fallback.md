# 8. CgiExecutor fallback for Stage 3

Date: 2026-08-19

## Status

Accepted

## Context

Stage 3 wants a custom PHP 8.2.6 wasi guest: vendored WLR patches, Dockerized
wasi-sdk 33, NTS, no opcache, a `raddy` C extension, and SAPI exports
`wizer.initialize` / `raddy_execute`. The WLR 8.2.6 recipe (`php/v8.2.6` at
`6e7674cf`) only builds `MAKE_TARGETS=cgi` (CLI only for the wasmedge flavor),
links wasi-sdk **20** flavor libraries, and configures `--host=wasm32-wasi`.
That is not a raddy SAPI and it is not wasi-sdk 33. Rebuilding PHP, porting
the patches, and adding a new SAPI is the hard block the plan time-boxes.

`wasmtime-wasi` 47.0.3 is the WASI implementation that matches the wasmtime
47.0.3 pin. The pin table names the engine, not this crate; they ship together.

## Decision

Stage 3 ships `CgiExecutor` (and `PhpExecutor` as a thin wrapper) driving the
stock WLR `php-cgi-8.2.6.wasm` (`just guest-php` downloads it and checks
`guest/php/php-cgi.lock`). Apps in `guest/apps/plain/` use CGI headers and
`php://input`. The WLR patch lineage is vendored under `guest/php/patches/`
for the custom SAPI work. `Raddy\*` phpt cases wait on that SAPI.

`just run` serves the PHP app, not the toy echo guest.

## Consequences

`@stage-3` hello / echo / Set-Cookie can close without a custom PHP build.
Stage 4 still needs `wizer.initialize` on a raddy SAPI; CgiExecutor cannot
snapshot a booted kernel. `wasmtime-wasi` 47.0.3 is a workspace pin.
