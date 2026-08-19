# 6. tokio `time` feature

Date: 2026-08-18

## Status

Accepted

## Context

The plan says no tokio features beyond those it names. Hostcall deadlines
(audit RADDY-003) need `tokio::time::timeout` so `resp_write` / `body_read`
cannot pin a worker after the epoch deadline.

## Decision

Enable tokio's `time` feature. It is required by the C2/C3 deadline contract,
not an extra runtime luxury.

## Consequences

Workspace tokio features are `macros`, `rt-multi-thread`, `io-util`, `sync`,
`time`.
