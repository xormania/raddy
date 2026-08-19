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
    cargo run -p raddy-executor --release --example measure

bench-check:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${WASI_SDK_PATH:?WASI_SDK_PATH must point at a wasi-sdk 33 root}"
    tmp=$(mktemp)
    trap 'rm -f "$tmp"' EXIT
    cargo run -p raddy-executor --release --example measure --quiet > "$tmp"
    cat "$tmp"
    python3 scripts/bench-check.py bench/baselines.json "$tmp"

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
    : "${WASI_SDK_PATH:?WASI_SDK_PATH must point at a wasi-sdk 33 root}"
    name={{NAME}}
    if [[ "$name" != "hello-symfony" ]]; then
        echo "unknown artifact $name" >&2
        exit 1
    fi
    wizer_bin=${WIZER:-wizer}
    out=artifacts/hello-symfony
    mkdir -p "$out/app.fs"
    raw=$(mktemp)
    trap 'rm -f "$raw"' EXIT
    "$WASI_SDK_PATH/bin/clang" --target=wasm32-wasip1 -nostdlib \
        -Wl,--no-entry -Wl,--export=raddy_execute -Wl,--export-memory \
        -Wl,--allow-undefined -fno-builtin -O2 \
        -o "$raw" guest/apps/hello-symfony/guest.c
    "$wizer_bin" -f wizer.initialize -o "$out/guest.wasm" "$raw"
    sum=$(sha256sum "$out/guest.wasm" | awk '{print $1}')
    wasmtime_bin=${WASMTIME:-wasmtime}
    cwasm="$out/guest.$(uname -m)-unknown-linux-gnu.cwasm"
    if command -v "$wasmtime_bin" >/dev/null; then
        "$wasmtime_bin" compile -o "$cwasm" "$out/guest.wasm"
        csum=$(sha256sum "$cwasm" | awk '{print $1}')
        triple=$(uname -m)-unknown-linux-gnu
    else
        csum=
        triple=
    fi
    {
        echo '[artifact]'
        echo 'name = "hello-symfony"'
        echo 'version = "0.1.0"'
        echo 'abi = 1'
        echo
        echo '[module]'
        echo 'wasm = "guest.wasm"'
        echo "sha256 = \"$sum\""
        echo
        echo '[app]'
        echo 'fs = "app.fs/"'
        echo
        echo '[limits]'
        echo 'memory_max_mib = 64'
        echo 'deadline_ms = 30000'
        if [[ -n "$csum" ]]; then
            echo
            echo "[precompiled.$triple]"
            echo "cwasm = \"$(basename "$cwasm")\""
            echo 'wasmtime = "47.0.3"'
            echo "sha256 = \"$csum\""
        fi
    } > "$out/raddy.artifact.toml"
    cp guest/apps/hello-symfony/public/index.php "$out/app.fs/index.php"
    echo "wrote $out (sha256 $sum)"

run *ARGS:
    cargo run -p raddy -- {{ARGS}}
