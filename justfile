# Development recipes. `just check` is the commit gate.
# From Stage 1, guest C builds need WASI_SDK_PATH (wasi-sdk 33).

set shell := ["bash", "-euo", "pipefail", "-c"]

check:
    just guest-php
    cargo fmt --check
    cargo clippy --workspace --all-targets -- --deny warnings
    cargo test --workspace

fmt:
    cargo fmt

bdd:
    cargo test -p raddy-bdd --test bdd -- --nocapture

# Run one stage's Gherkin slice (`@stage-N`).
bdd-stage N:
    BDD_STAGE={{N}} cargo test -p raddy-bdd --test bdd -- --nocapture

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
    lock=guest/php/php-cgi.lock
    url=$(awk -F= '/^url=/{print $2}' "$lock")
    want=$(awk -F= '/^sha256=/{print $2}' "$lock")
    name=$(awk -F= '/^filename=/{print $2}' "$lock")
    out=guest/php/out
    mkdir -p "$out"
    dest="$out/$name"
    if [[ -f "$dest" ]]; then
        got=$(sha256sum "$dest" | awk '{print $1}')
        if [[ "$got" == "$want" ]]; then
            echo "$dest already verified"
            exit 0
        fi
        rm -f "$dest"
    fi
    curl -fsSL "$url" -o "$dest.part"
    got=$(sha256sum "$dest.part" | awk '{print $1}')
    if [[ "$got" != "$want" ]]; then
        echo "sha256 mismatch: got $got want $want" >&2
        rm -f "$dest.part"
        exit 1
    fi
    mv "$dest.part" "$dest"
    echo "fetched $dest"

artifact NAME:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "TODO: artifact builder lands in stage 4 (asked for {{NAME}})" >&2
    exit 1

run *ARGS:
    cargo run -p raddy -- {{ARGS}}
