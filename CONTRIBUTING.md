# Contributing

Code enters `dev` only through pull requests. `master` is the published line.
Work starts from `origin/dev` on a temporary branch. Only **xormania** approves
and merges. Agents open draft PRs through a GitHub write path whose resulting
visible author is **xormania**, verify that author, and stop. Credentials and
transport authentication are separate from attribution and need not share its
label. If no available path produces xormania attribution, stop. Do not request
reviewers.

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

The body is how the change is reviewed. It is a self-contained explanation for
a reader who never saw the chat, not a commit diary or a prose copy of the diff.

There is no fixed set of narrative headings. Follow the shape of the change:

- lead with the concrete problem or decision and why it matters;
- explain the mechanism, meaningful boundaries, and what remains unchanged or
  out of scope;
- use compact tables where they make comparisons or contracts easier to judge;
- finish with `## Verification`, naming the exact commands or hosted evidence
  and the results that matter.

Every figure must be produced at the time of writing by a named command or a
linked CI run. Do not rely on recalled numbers. For a docs-only change with
nothing quantitative to measure, say that plainly in `Verification`.

Use the repository pull-request template. Its checklist is the only fixed
section. PRs start as drafts; complete the checklist before marking one ready.
