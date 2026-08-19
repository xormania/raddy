# 3. GitHub tags are not acceptance, and agents do not create them

Date: 2026-08-18

## Status

Accepted

## Context

The plan tags `stage-N` on the commit that completes a stage. `stage-1` was
pushed to GitHub at `62ebb4a`. An audit of `origin/dev` (`proj/crawl/audit.md`)
showed that tag sits on an unused typestate stub and does not meet Stage 1
acceptance. History and published tags must not be rewritten.

## Decision

- Do not create GitHub tags (`stage-*`, `v*`, or otherwise). Agents never
  `git tag` or `git push --tags`.
- Existing `stage-0` and `stage-1` tags are historical pointers only. They are
  not acceptance authority.
- Stage completion is recorded in `CHANGELOG.md` and an ADR after `just check`
  is green on the real wasi-sdk 33 gate. A later human release tag is out of
  band for agents.

## Consequences

Stage 2 does not start until the audit's Stage 1 acceptance evidence is green.
Nobody should treat `stage-1` as "done."
