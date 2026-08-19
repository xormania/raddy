# 7. tokio `net` and `signal` features

Date: 2026-08-19

## Status

Accepted

## Context

Stage 2 binds a TCP listener and drains on SIGTERM. The plan names tokio
1.53.1 but not those features. Std cannot accept HTTP/1.1 on tokio without
`net`, and graceful drain needs `signal`.

## Decision

Enable tokio features `net` and `signal`.

## Consequences

Workspace tokio features are `macros`, `rt-multi-thread`, `io-util`, `sync`,
`time`, `net`, `signal`.
