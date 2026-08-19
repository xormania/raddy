# 11. Response completion and bounded teardown

Date: 2026-08-19

## Status

Accepted

## Context

C2 fixes `ExecResponse.body` as `mpsc::Receiver<Bytes>` closed at
`raddy_resp_end`. C4 requires guest termination and instance destruction to
run after the response and bounds post-response guest work. Waiting for
`raddy_execute` before closing the channel made EOF include teardown. Timing
only a Rust `spawn_blocking` join did not interrupt a guest still executing.

## Decision

Keep C2's body type unchanged. Add a `response_complete` oneshot sender for
the server adapter to acknowledge that the closed body reached its consumer,
plus a `teardown` receiver for lifecycle observation. These are additive C2
fields; ADR 0005's `done` signal remains the post-head error channel.

The `raddy_resp_end` hostcall closes the body channel immediately and records
the post-response deadline. The store's epoch callback checks the nearer of
the request deadline and teardown window. An epoch interrupt after protocol
state `Ended` is a successful, bounded teardown kill; the spent instance is
then dropped only after response completion. C3 gains no new guest export.

## Consequences

HTTP consumers acknowledge completion on EOF or body drop. Direct executor
consumers that need to await teardown must send or drop `response_complete`.
Long requests revisit the epoch callback once per teardown window; normal
fast-path requests complete before that callback runs.
