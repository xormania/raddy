# 2. Integrate on `dev` through pull requests

Date: 2026-08-18

## Status

Accepted

## Context

`proj/plan.md` §6 said the repo is local-only, `main` is the only branch, and
there are no remotes. The work now lives at https://github.com/xormania/raddy
with `master` as the published line and a `dev` integration branch. Agents must
not land on `dev` by direct push.

## Decision

- Agents start from `origin/dev`, work on a temporary branch, and open a draft PR
  into `dev` through a GitHub write path whose resulting visible author is
  **xormania**. They verify that author and stop if no available path produces
  xormania attribution.
- Credentials and transport authentication are separate from attribution.
  Agents preserve working credentials rather than changing them to match the
  attribution label.
- Only **xormania** approves and merges.
- Agents do not request reviewers, do not `@`-mention for review, do not
  push to `dev` or `master`, do not approve, and do not merge.
- No force-push and no history rewrite.

This supersedes plan §6 for branching and remotes. Conventional commits, the
`just check` gate, and the xormania author identity stay.

## Consequences

`dev` is the only integration target. A repository ruleset on `dev` should
require a pull request, one approving review from the code owner, and should
forbid force-push and deletion. xormania applies that ruleset.
