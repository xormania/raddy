# Development recipes. `just check` is the commit gate.
# From Stage 1, guest C builds need WASI_SDK_PATH (wasi-sdk 33).

set shell := ["bash", "-euo", "pipefail", "-c"]

check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- --deny warnings
    cargo test --workspace

fmt:
    cargo fmt

bdd:
    cargo test -p raddy-bdd --test bdd -- --nocapture

# Run one stage's Gherkin slice (`@stage-N`).
bdd-stage N:
    RADDY_BDD_STAGE={{N}} cargo test -p raddy-bdd --test bdd -- --nocapture

bench:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TODO: criterion benches land in stage 5" >&2
    exit 1

bench-check:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TODO: bench-check lands in stage 5" >&2
    exit 1

guest-toy:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${WASI_SDK_PATH:?WASI_SDK_PATH must point at a wasi-sdk 33 root}"
    out=guest/toy/out
    mkdir -p "$out"
    clang="$WASI_SDK_PATH/bin/clang"
    for name in echo spin trap; do
        "$clang" --target=wasm32-wasip1 -nostdlib \
            -Wl,--no-entry -Wl,--export=raddy_execute -Wl,--export-memory \
            -Wl,--allow-undefined -O2 \
            -o "$out/$name.wasm" "guest/toy/$name.c"
    done

guest-php:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TODO: just guest-php is implemented in stage 3" >&2
    exit 1

artifact NAME:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TODO: artifact builder lands in stage 4 (asked for {{NAME}})" >&2
    exit 1

run *ARGS:
    cargo run -p raddy -- {{ARGS}}
