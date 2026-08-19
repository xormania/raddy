# Raddy

> [!WARNING]
> **Pre-alpha:** Raddy is an experimental prototype. It is not ready for
> production use, it does not yet run Symfony from a prebooted snapshot, and
> its interfaces may change without notice.

[![CI](https://github.com/xormania/raddy/actions/workflows/check.yml/badge.svg?branch=dev)](https://github.com/xormania/raddy/actions/workflows/check.yml)

Raddy is a Rust HTTP runtime being built for PHP and Symfony on WebAssembly.
The design boots an application once, snapshots it with Wizer, and resumes a
pristine guest for every request. The host owns HTTP, concurrency, deadlines,
and reusable infrastructure; request-local guest state is discarded.

The goal is a narrow runtime with the isolation of per-request execution and a
fast path designed for long-running Symfony applications.

## Project status

The runtime foundation is working, but the PHP/Symfony execution path is not
complete.

| Area | Current state |
|---|---|
| HTTP | Hyper-based ingress, bounded concurrency, and streamed request and response bodies |
| Wasm execution | Wasmtime pooling, copy-on-write initialization, epoch deadlines, and a fresh ABI guest per request |
| Artifacts | Wizer snapshots, precompiled cwasm selection, manifests, and SHA-256 verification |
| PHP | A locked legacy PHP 8.2.6 CGI executor is implemented and tested, but is not selected by the server binary |
| Symfony snapshots | Not implemented; the current snapshot demo is a C ABI guest, not a booted Symfony kernel |
| Stability | No compatibility, performance, or security guarantees yet |

The reason for the CGI executor is recorded in
[ADR 0008](docs/adr/0008-cgi-executor-fallback.md). The later
[ADR 0009](docs/adr/0009-snapshot-abi-guest.md) records the current C snapshot
boundary; the server binary now follows that snapshot path.
Production support is intended to follow maintained PHP and Symfony release
lines, with LTS compatibility defined explicitly.

## Request path

```text
client -> raddy-server -> admission limits -> raddy-executor -> fresh Wasm guest
                                                        |
                                                        +-> streamed response
                                                        +-> bounded teardown, then discard
```

Raddy keeps the public ABI small. The guest reads a request envelope and body
through host calls, writes the response head and body, and signals completion.
The host validates artifacts before loading them and controls the resources a
guest can reach.

## Requirements

- Rust 1.97.1, selected by [`rust-toolchain.toml`](rust-toolchain.toml)
- [just](https://github.com/casey/just)
- wasi-sdk 33, exposed through `WASI_SDK_PATH`
- `curl` and GNU `sha256sum` for fetching the locked PHP guest
- Python 3 for `just bench-check`
- Wizer 11.0.3 and the Wasmtime 47.0.3 CLI for building a runnable artifact

The current artifact recipe and documented run path target glibc Linux. Other
host targets are not supported yet.

Cargo dependencies are pinned in [`Cargo.lock`](Cargo.lock). The first check
downloads the locked PHP CGI Wasm module and verifies its digest.

## Build and test

Set `WASI_SDK_PATH` to the wasi-sdk 33 installation directory, then run the
core code-quality gate used by CI:

```bash
export WASI_SDK_PATH=/path/to/wasi-sdk-33
just check
```

Useful development commands:

```bash
just bdd             # all behavior scenarios
just bdd-stage 5     # one tagged scenario slice
just bench           # local executor measurements
just bench-check     # compare measurements with the checked-in baseline
```

`just bench-check` compares local measurements with a checked-in baseline. Its
result is advisory unless it runs in the environment that produced that
baseline. Performance changes require before-and-after measurements from the
same machine and build mode; absolute millisecond thresholds are not portable.

Hosted CI also builds an artifact, smoke-tests the running server, and audits
Rust dependencies.

## Run the current snapshot demo

With wasi-sdk, Wizer, and the Wasmtime CLI available:

```bash
just artifact hello-symfony
just run
```

In another shell:

```bash
curl 'http://127.0.0.1:8080/hello?name=Raddy'
# Hello Raddy
```

Despite the artifact name, this currently exercises the snapshot runtime with
the C ABI guest described above. It does not yet execute Symfony.

Use `Ctrl-C` to stop the server. To inspect the resolved defaults, create a
configuration file from them, and run with that file:

```bash
cargo run -p raddy -- --print-config
cargo run -p raddy -- --print-config > /tmp/raddy.toml
cargo run -p raddy -- --config /tmp/raddy.toml
```

Configuration is layered from defaults, TOML, `RADDY_*` environment variables,
and command-line overrides. Unknown keys are rejected.

## Workspace

| Path | Responsibility |
|---|---|
| `crates/raddy` | process entry point and runtime assembly |
| `crates/raddy-server` | HTTP ingress, admission, and streaming |
| `crates/raddy-executor` | Wasmtime engine, guest lifecycle, and pooling |
| `crates/raddy-artifact` | artifact manifests, hashes, and loading |
| `crates/raddy-abi` | host/guest request protocol |
| `crates/raddy-config` | typed layered configuration |
| `crates/raddy-bdd` and `features/` | executable behavior specifications |
| `guest/` | PHP inputs, application fixtures, and ABI test guests |

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development workflow, naming
conventions, and pull-request expectations. Pull requests target `dev` and
begin as drafts.

## License

Licensed under the [Apache License 2.0](LICENSE).
