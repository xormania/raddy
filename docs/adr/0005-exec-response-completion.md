# 5. `ExecResponse.done` observes post-head failure

Date: 2026-08-18

## Status

Accepted

## Context

C2's `ExecResponse` is a head plus a body channel. C3 requires a missing
`resp_end` to be `ExecError::Protocol`. Once `execute()` has resolved at the
response head (§3.6), a later protocol failure cannot change that `Result`.
Closing the body channel looks the same as a clean `resp_end`.

## Decision

Add `done: oneshot::Receiver<Result<(), ExecError>>` to `ExecResponse`. It
resolves when the guest worker finishes: `Ok(())` only if `raddy_execute`
returned 0 and the protocol is `Ended`. Truncation after the head is visible
there without waiting on a display-string trap.

## Consequences

Ingress (Stage 2) must watch `done` as well as the body channel when mapping
C7 mid-body failures. Existing echo tests may ignore `done`.
