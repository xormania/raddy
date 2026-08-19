# Contributing to Raddy

Raddy is pre-alpha, so focused bug fixes, tests, documentation, and measured
runtime improvements are especially useful. Discuss large changes to the ABI,
architecture, or dependency set in a
[GitHub issue](https://github.com/xormania/raddy/issues) before investing in an
implementation. Existing decisions are recorded in [`docs/adr/`](docs/adr/).

## Workflow

`dev` is the integration branch; `master` is the published line. All changes
enter `dev` through pull requests.

1. Fetch `origin` and start a `type/short-slug` branch from its current `dev`.
2. Keep the change focused and add the smallest tests that prove its behavior.
3. Run `just check` and any relevant focused checks.
4. Push the topic branch and open a draft pull request into `dev`.
5. Complete the pull-request template and mark the PR ready when the change and
   hosted checks are ready for review.

Maintainers perform final review and merge. Do not push directly to `dev` or
`master`, force-push shared history, or create repository tags as part of a
contribution.

## Development checks

The full local gate is:

```bash
just check
```

It verifies formatting, runs Clippy with warnings denied, and tests the entire
workspace. See the [README](README.md#requirements) for required local tools.

Run focused checks while developing when they make the feedback loop shorter:

```bash
just bdd
just bdd-stage 5
just bench
just bench-check
```

Behavior changes should have a test that fails for the intended reason before
the implementation makes it pass. Deliberately introduce the failure and
observe the expected test failure; a mutation that never reaches the exercised
path is not evidence. Do not duplicate an existing test for the same fact.

Performance assertions must be relative measurements from a comparable
machine and build mode, not absolute elapsed-time limits. Security claims need
an adversarial case that crosses the boundary being claimed.

Documentation-only changes may omit local code checks when they cannot affect
the build, but the PR must say so and hosted CI must still pass.

## Names

### Commits

```text
type(scope): summary
```

| Piece | Rule |
|---|---|
| `type` | `feat` `fix` `test` `perf` `refactor` `build` `docs` `chore` `ci` |
| `scope` | crate or area, such as `executor`, `abi`, `guest`, `bdd`, `repo`, or `config` |
| `summary` | imperative, lowercase after the colon, no trailing period, at most 72 characters |

Use a commit body when the decision or rationale is not clear from the subject.

Branches use `type/short-slug` and the same types as commits. Create them from
the latest `origin/dev`.

### Pull-request titles

```text
Label: summary
```

The label is title case; acronyms remain uppercase. It is not the commit
`type(scope)`.

| Label | Use for |
|---|---|
| `Feat:` | new behavior |
| `Fix:` | a defect |
| `Test:` | tests only |
| `Perf:` | a measured speed change |
| `Refactor:` | unchanged behavior with a different implementation shape |
| `Build:` | toolchain, guest build, or lockfile changes |
| `Docs:` | prose and ADRs |
| `Chore:` | maintenance that fits none of the other labels |
| `CI:` | workflows and the check gate |

Use an imperative summary, lowercase after the colon, with no trailing period.
The title describes the whole change, not only the last commit.

Examples:

- `Docs: clarify contribution workflow`
- `CI: cache the Rust build`
- `Fix: reject an invalid artifact hash`

## Pull-request body

The body is the review narrative. It must be self-contained and understandable
without external discussion; it is neither a commit diary nor a prose copy of
the diff.

There is no fixed set of narrative headings. Follow the shape of the change:

- lead with the concrete problem or decision and why it matters;
- explain the mechanism, meaningful boundaries, and what remains unchanged or
  out of scope;
- use compact tables when they make comparisons or contracts easier to judge;
- finish with `## Verification`, naming the exact commands or hosted evidence
  and the results that matter.

Support every quantitative claim with the command that produced it or a linked
CI run from the current change. For documentation-only work with no measured
result, state that plainly in `Verification`.

Use the repository pull-request template. Its checklist is the only fixed
section. Pull requests start as drafts; complete the checklist before marking a
PR ready for review.
