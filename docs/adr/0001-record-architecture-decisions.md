# 1. Record architecture decisions

Date: 2026-08-18

## Status

Accepted

## Context

The staged plan is the spec. Implementations will sometimes have to disagree with it — a different crate, a changed contract, a reordered stage. Those disagreements need a durable record next to the code, not a comment in a chat.

## Decision

Architecture Decision Records live in `docs/adr/` as `NNNN-title.md`, numbered from 0001. This file is the first. A change that disagrees with `proj/plan.md` is incomplete until an ADR ships with it.

## Consequences

Deviations are reviewable and dated. Training-data priors lose to the plan; the plan loses to an ADR plus the code that implements it.
