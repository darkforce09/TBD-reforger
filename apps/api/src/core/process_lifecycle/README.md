# Process lifecycle

The process-wide shutdown signal of the [API](/documentation/glossary/a_to_f.md#api): the one
flag the `api` binary raises when it is asked to stop, so the
[SSE](/documentation/glossary/n_to_z.md#sse) streams that would otherwise hold a graceful
shutdown open end with it.

## Contents

```text
apps/api/src/core/process_lifecycle/
├── mod.rs  `ShutdownSignal`, a one-way flag that wakes every waiter, and `process_shutdown`
└── tests/  unit tests for beginning, waking, late waiters, idempotence and a dropped signal
```

## How it works

A `ShutdownSignal` holds a `tokio::sync::watch` channel that reads `false` until `begin` stores
`true`. `begun` hands out a future that owns its own receiver and resolves once the signal has
begun, at once when it already has, so a long-lived stream can race it without borrowing the
signal. `has_begun` reads the flag. Beginning twice is the same as beginning once, and a begun
signal never returns to not begun.

`process_shutdown` is the one instance the process shares, kept in a static. The `api` binary
begins it when SIGINT or SIGTERM arrives, as `axum::serve` stops accepting connections and starts
its graceful drain. `authorize_event_stream` in `crate::core::middleware` races every open event
stream against it and closes the body when it begins, with no event of its own, so the drain
waits only for ordinary requests and the process exits. A client sees the plain end of the
stream and reconnects the way it does after any end; the audit log feed resumes from its
`Last-Event-ID`.

```text
SIGINT / SIGTERM ─▶ bin/api.rs begins process_shutdown ─▶ axum::serve stops accepting, drains
                                   └─▶ authorize_event_stream closes every open SSE body
                                        ─▶ the drain completes ─▶ the process exits 0
```

## Boundaries

- Depends on: `tokio::sync::watch`.
- Used by: `apps/api/src/bin/api.rs`, which begins `process_shutdown`;
  `crate::core::middleware::authorized_event_stream`, which waits on it for the audit log feed of
  `administration` and the server status stream of `server_infrastructure`;
  `apps/api/tests/audit_replay_shutdown.rs`, which begins it in its own process.
- Rules: only the `api` binary begins `process_shutdown`; the library's unit tests race signals of
  their own, because a begun process shutdown ends every event stream opened afterwards in the same
  process, which is also why `tests/audit_replay_shutdown.rs` is a binary with one case.
