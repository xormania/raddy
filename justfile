# Development recipes. `just check` is the commit gate.

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
