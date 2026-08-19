# raddy

Rust HTTP server that runs Symfony (PHP) compiled to WebAssembly: one pristine
guest instance per request, resumed from a Wizer snapshot.

## Toolchain

Pinned in `rust-toolchain.toml` and recorded here from `rustc --version` at
Stage 0:

- Rust **1.97.1** (`8bab26f4f`, 2026-07-14)

The rest of the pin table (wasmtime, Wizer, wasi-sdk, PHP, Symfony, crates)
lives in `proj/plan.md`. Anything marked "latest at init" is whatever
`Cargo.lock` resolved during Stage 0.

## Check

```bash
just check
just bdd-stage 0
```
