# raddy

Rust HTTP server that runs Symfony (PHP) compiled to WebAssembly: one pristine
guest instance per request, resumed from a Wizer snapshot.

The design, stages, contracts, and version pins live in `proj/plan.md` (gitignored
handoff). **That file outranks training data** for versions, APIs, and toolchain
facts. Read it before writing code. After the tree implements a piece, the code
is the authority for that piece.

`proj/` on disk is the untracked drop zone. The plan's `/proj` means this
repository root, not that directory.

## Standing orders

Execute stages in order (`proj/plan.md` §7). A stage is done only when its
*Done when* list passes `just check`. Deviations need an ADR in `docs/adr/`.
No dependency outside the plan's pin table without an ADR.

Cleverness is budgeted for the fast path (`proj/plan.md` §3). Everywhere else,
boring.

When blocked twice on the same error, write `docs/notes/blockers.md` before a
third try.

Keep files under ~400 lines.

## Git

Remote: https://github.com/xormania/raddy

| Branch | Role |
|---|---|
| `master` | Published line. Agents do not push here unless asked. |
| `dev` | Integration line. Code enters `dev` **only** through pull requests. |

### Agent workflow

Every session that writes to the tree:

1. `git fetch origin` and start from `origin/dev`.
2. Create a temporary branch from that tip: `type/short-slug` (same types as
   commits). No tool names in the branch name.
3. Author and committer stay
   `xormania <127287135+xormania@users.noreply.github.com>`. Do not change git
   config. No `Co-Authored-By`. No generated-with footers.
4. Once `justfile` exists, `just check` green before every commit that is not
   docs-only. Never commit red.
5. Push the temp branch and open a PR **into `dev`**, authenticated as
   **xor-machine** (xor-machine PAT / `gh` as xor-machine). Request review from
   **xormania**.
6. Stop. Do not approve. Do not merge. Do not push to `dev` or `master`. Only
   **xormania** approves and merges.

No force-push. No history rewrite. No amending a commit that is already on a
remote branch.

`.github/CODEOWNERS` is `* @xormania`. That is the only required reviewer.

Names and PR bodies: `CONTRIBUTING.md`. Commits stay `type(scope): summary`.
PR titles are `Label: summary` (`Docs:`, `CI:`, `Feat:`, …). The PR body is
Why / What / Background / Data / Checks — Data holds measured figures, or
`none`.

## Serena

If Serena's MCP tools are available:

- `activate_project` on this repository at the start of a coding session.
- Discover Rust with `get_symbols_overview` / `find_symbol`; edit with
  `replace_symbol_body` / `replace_content`. Do not read a whole `.rs` file
  to find a function.
- `restart_language_server` if rust-analyzer looks stale after a large edit
  or a toolchain change.
- Memories are pointers into this file and the plan. Do not write a second brief.

Tracked config is `.serena/project.yml`. After changing it, run
`scripts/serena-setup` so the copy Serena actually reads stays in sync.

Language servers: `rust` (default), `toml` (Cargo / raddy.toml / artifact
manifests). Add `cpp` when `guest/toy/` exists, `php` when `guest/php/` exists.

## Building

Do not `cargo init` or add crates until the current stage says so.

When the workspace exists:

```bash
just check          # fmt --check, clippy -D warnings, test --workspace
just bdd-stage N    # that stage's Gherkin slice
```

PRs into `dev` (and `master`) run the same `just check` in
`.github/workflows/check.yml`. Guest C builds need `WASI_SDK_PATH`; CI
installs wasi-sdk 33.

Edition 2024. Commit `Cargo.lock`. Pin the toolchain in `rust-toolchain.toml`
in the same change as the workspace is created. Never invent a crates.io
version — `cargo add`. Std first. No `unsafe` unless a stage requires it.
No tokio extra features beyond what the plan names.

Guest PHP is built in Docker via `just guest-php` (wasi-sdk 33). Host PHP is
not required for Stages 0–2. Wizer is the CLI (`wizer --version` → 11.0.3).
wasmtime crate pin is 47.0.3; 48.0 LTS is adopted in Stage 1 only if published,
via ADR.

Guest C builds use the `WASI_SDK_PATH` environment variable. Do not hard-code a
home-directory path in tracked files.

## Tests

The plan's doctrine is the spec: each stage writes its `.feature` scenarios and
the unit tests that stage names, observes them fail, then implements. `just check`
is the gate.

Do not add a second test for the same fact. Do not write tests that restate
rustc, clippy, or a type you already wrote. Do not build a regression battery
in advance of a failure. Performance assertions are relative, never absolute
milliseconds.

A plant in a test must be shown to have landed. If it did not, the case is
invalid, not passing.

## Code

- One crate, one responsibility (`proj/plan.md` §2.3). The listed patterns
  (§2.4) are required, not decorative.
- Small public API; modules private unless they are the crate's interface.
- `Result` in library code. `unwrap`/`expect` only in tests, or when a broken
  invariant is a programmer bug (the message names the invariant).
- Exhaustive `match`. No bool/`Option` positional arguments that make call
  sites read `foo(false, None)`.
- Comments explain non-obvious constraints, not the next line.
- Do not add README, CONTRIBUTING, or extra docs unless the current stage
  asks for them.

## Finish

```bash
git status --short --branch
# just check, once the justfile exists and Rust changed
# PR into origin/dev as xor-machine; do not merge
```
