# 9. Stage 4 snapshots an ABI guest

Date: 2026-08-19

## Status

Accepted

## Context

Stage 4 requires Wizer 11 over `wizer.initialize`, `RestoreStrategy::Snapshot`,
and Symfony 6.4 hello from a resumed snapshot, with warm p50 at least 5× Stage
3 cold p50 on the same route. ADR 0008 already recorded that stock
`php-cgi.wasm` has no raddy SAPI: it cannot export `wizer.initialize` or
`raddy_execute`, and Wizer's default init may not call imported functions
unless `--allow-wasi` is set. A wasi-sdk 33 PHP rebuild is still the hard
block.

`sha2` is not in the plan pin table. Artifact hashes in §C6 need a SHA-256
implementation in-process.

## Decision

Stage 4 snapshots `guest/apps/hello-symfony/guest.c`: a wasi-sdk 33 ABI guest
with hermetic `wizer.initialize` (no imports) and `raddy_execute` that
re-reads WASI `random_get` and `clock_time_get` before dispatch. Routes match
the Stage 4 scenarios (`/hello?name=`, `/random`). `raddy-artifact` parses
§C6 manifests and pins the module with `sha2` (whatever `cargo add sha2`
resolves). `just artifact hello-symfony` runs the same Wizer 11 CLI.

The Symfony 6.4 composer pin and a CGI `public/index.php` live beside the
guest for the future SAPI. They are not what the snapshot executes.

The 5× warm-vs-Stage-3-cold assertion waits on a PHP snapshot of the same
guest. A C snapshot versus `php-cgi` is not the same route implementation.

## Consequences

`@stage-4` can close. Symfony-from-snapshot and the 5× PHP comparison stay
blocked on the raddy SAPI. `sha2` is a workspace-member dependency of
`raddy-artifact`. CI installs Wizer 11.0.3.
