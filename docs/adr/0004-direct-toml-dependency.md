# 4. Direct `toml` 1.1.4 dependency

Date: 2026-08-18

## Status

Accepted

## Context

The plan's §9 pin table lists figment 0.10.19 (with its `toml` feature) but not
a direct `toml` crate. `--print-config` and the BDD "parses as TOML" step emit
and parse typed `Config` with `serde`. Figment's extract path does not honor
`deny_unknown_fields` (see `issues/figment-skips-unknown-fields.md`), so the
file overlay is also parsed with `toml` + serde.

`toml = "1.1.4"` was added with `cargo add` at Stage 0 ("latest at init") and
locked in `Cargo.lock`.

## Decision

Keep a direct `toml` 1.1.4 dependency in the workspace. Version changes go
through `cargo add` and an ADR, same as any pin outside §9.

## Consequences

Print/parse and unknown-key rejection stay on one crate. Figment remains the
layering engine only.
