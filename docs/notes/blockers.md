# Blockers

## Response-gated teardown on pre-head failures

Two test runs deadlocked while moving instance destruction behind response-body
completion. A guest that fails before emitting a head has no response body to
drive the completion signal: first the retained guest state kept the head
sender alive, then the executor retained the response-completion sender while
awaiting teardown. The next attempt must close both senders before awaiting the
detached teardown task on the pre-head error path.
