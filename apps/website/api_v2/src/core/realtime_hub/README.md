# Realtime hub

The in-process publish-subscribe hub that fans messages out to
[SSE](/documentation_v2/glossary.md#sse) clients by topic; the live server status streams run on
it.

## Contents

```text
apps/website/api_v2/src/core/realtime_hub/
├── mod.rs  `Hub`: one bounded broadcast channel per topic, created on first subscribe
└── tests/  unit tests for delivery, topic isolation and unsubscribe
```

## How it works

`AppState` holds one `Hub`. `Hub::subscribe` returns a receiver for a topic, and dropping the
receiver unsubscribes. `Hub::publish` never blocks: each topic buffers 16 messages, a subscriber
too slow to drain them lags and loses the oldest, and a topic whose last subscriber has gone is
removed on the next publish. Messages are bytes; the publisher chooses their encoding.

## Boundaries

- Depends on: `tokio::sync::broadcast`.
- Used by: `crate::core::application_state`, which creates the hub; in `server_infrastructure`,
  the status broadcast service, which publishes each server's status on its `server:{id}` topic,
  and the server status stream handler, which subscribes; the
  [game runtime](/documentation_v2/glossary.md#game-runtime) heartbeat of `match_telemetry` and
  the `server_status_publisher` and `runtime_session_expiry`
  [background workers](/documentation_v2/glossary.md#background-workers), which publish through
  that service.
- Rules: the hub lives in one process, so a subscriber sees only what its own process
  publishes; the audit log feed does not use it.
