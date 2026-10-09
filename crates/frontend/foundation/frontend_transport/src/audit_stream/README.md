# Audit stream transport

The browser half of the live [audit logs](/documentation/glossary/a_to_f.md#audit-logs) stream:
the connection the audit logs page holds open to `GET /api/v1/admin/audit-logs/stream`, and the
loop that reconnects it. The protocol it applies (what each event means, the resume cursor, the
reconnect wait, what an answer status means) is the parent file
`crates/frontend/foundation/frontend_transport/src/audit_stream.rs`, which also compiles into the native
test build and is tested there.

## Contents

```text
crates/frontend/foundation/frontend_transport/src/audit_stream/
└── transport.rs  the page-owned handle, the reconnect loop, one connection and its reader; wasm32 only
```

## How it works

```text
open_audit_stream ─▶ spawn run_stream ─▶ AuditStreamHandle (page keeps it, aborts on cleanup)
run_stream: wait for the session restore
  loop: tracker.begin_connection() ─▶ Last-Event-ID
        connect ─▶ 2xx ─▶ pump: SseParser ─▶ tracker.observe ─▶ callbacks
                ─▶ 401 ─▶ revalidate once (GET /me through the client), retry at once
                ─▶ 403, or 401 after revalidating ─▶ Offline, stop
                ─▶ 400 ─▶ forget the cursor, back off
                ─▶ anything else, or the body ends ─▶ Reconnecting, back off
        authorization_expired ─▶ revalidate, reconnect at once
```

`open_audit_stream` returns an `AuditStreamHandle` and spawns the loop. Each connection builds its
own request (`stream_request`: the bearer token, `Accept: text/event-stream`, `Last-Event-ID` when
resuming) with its own abort controller, which it parks in the handle before the fetch, so
`AuditStreamHandle::abort` always reaches the connection in flight. `pump` reads the body chunk by
chunk into an `SseParser`, gives each item to the tracker, and calls the page's
`AuditStreamCallbacks`: `on_state` (`Live` on `ready`, `Reconnecting` before each wait, `Offline`
when it stops), `on_ready` with whether the history must be (re)loaded, `on_row`, `on_reset` and
`on_rejected` for an event that did not decode, which is also logged to the console. A missing
access token is treated as a `401`: a cold start restores only the refresh token, and the
revalidation mints the access token. The task checks the stop flag after every await and before
every callback.

## Boundaries

- Depends on: the parent module's `AuditStreamTracker`, `AuditStreamStep`, `connect_verdict` and
  `AUDIT_STREAM_PATH`; `crate::sse_frames` (`SseParser`);
  `crate::client` (`api_get`, `API_BASE`); `frontend_api_dtos`
  (`administration`, `MeResponse`); `crate::token_provider`
  (`TokenProvider`, the session the stream reads its token from); `web-sys`, `js-sys`,
  `wasm-bindgen-futures` and `gloo-timers`.
- Used by: `crates/frontend/pages/administration_pages/src/audit_logs/page.rs`, which opens the
  stream and aborts it on cleanup.
- Rules: every connection resumes from the tracker's cursor and carries its own abort signal, and
  the handle's abort stops the loop and the connection, with no global slot holding the handle
  (`audit_stream_transport_resumes_from_the_tracker_and_aborts_through_the_handle` in
  `crates/frontend/foundation/frontend_transport/src/tests/audit_stream.rs`).

## Related documentation

- [Audit logs page](/documentation/crates/frontend/pages/administration_pages/audit_logs/audit_logs_page.md)
  — what the page does with the stream.
- [Audit replay and reset](/documentation/crates/api/api_server/verification_evidence/administration_and_content.md#audit-replay-and-reset)
  — the server side of the protocol.
