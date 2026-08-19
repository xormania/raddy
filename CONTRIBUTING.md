# Contributing

Code enters `dev` only through pull requests. `master` is the published line.
Work starts from `origin/dev` on a temporary branch. Only **xormania** approves
and merges. Agents open PRs as **xor-machine** and stop.

`just check` is the gate. Green before every commit that is not docs-only.
Never commit red. Never force-push. Never rewrite history. Do not create
GitHub tags.

Author and committer stay
`xormania <127287135+xormania@users.noreply.github.com>`. Do not change git
config. No tool names in branches, commits, PRs, or docs. No `Co-Authored-By`.
No generated-with footers.

The product brief is `AGENTS.md`. This file is the house style for names and
PR bodies.

## Names

### Commits

```
type(scope): summary
```

| Piece | Rule |
|---|---|
| `type` | `feat` `fix` `test` `perf` `refactor` `build` `docs` `chore` `ci` |
| `scope` | crate or area: `executor`, `abi`, `guest`, `bdd`, `repo`, `config`, … |
| `summary` | imperative, lowercase after the colon, no trailing period, ≤72 characters |

Commit body: the decision and why, when that is not obvious from the subject.
Not a diary of files touched.

Branch: `type/short-slug` from `origin/dev`. Same types as commits. No tool names.

### Pull request titles

```
Label: summary
```

The label is Title Case (acronyms stay caps). It is not the commit `type(scope)`.

| Label | Use for |
|---|---|
| `Feat:` | new behavior |
| `Fix:` | a defect |
| `Test:` | tests only |
| `Perf:` | a measured speed change |
| `Refactor:` | same behavior, different shape |
| `Build:` | toolchain, guest build, lockfile |
| `Docs:` | prose, ADRs, this file |
| `Chore:` | process that is none of the above |
| `CI:` | workflows and the check gate |

Summary: imperative, lowercase after the colon, no trailing period. The title
names the whole change, not the last commit.

Examples: `Docs: house rules for commits and pull requests`,
`CI: run just check on pull requests`.

## PR body

The body is how the change is reviewed. Fill every section. Delete a section
only if it cannot apply, and say so in one line.

### Why

The problem or decision that made this change necessary. Write it so a reader
who never saw the chat still knows why this exists. Not a restatement of the
title.

### What

What changed, in observable terms: behavior, files that matter, what did not
change. Not a file list (`git diff --stat` already is).

### Background

Context a reviewer needs and will not get from the diff: the prior state, a
constraint, a rejected alternative. Keep it short.

### Data

Evidence. Every figure is produced by a command at the time of writing, not
recalled. Paste the command and the line that matters, or link to CI.

If there is nothing to measure, write `none` and why (for example: docs-only,
rename with no behavior change).

Do not put a number in Why/What/Background unless Data has the command that
produced it.

## Checks

State what you ran. For Rust, that is `just check` unless the change is
docs-only. Guest C still needs `WASI_SDK_PATH`. CI on the PR is the same
recipe; it does not replace saying what you ran locally.

Use the repository pull-request template. Leave the headings in place.
