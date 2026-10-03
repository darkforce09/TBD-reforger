# API layer

Everything the app talks to the [API](/documentation/glossary/a_to_f.md#api) through: the HTTP client,
the typed endpoint calls, the wire types they carry, and the two
[SSE](/documentation/glossary/n_to_z.md#sse) streams: one server's live status and the live audit
log. A page imports one path to fetch and one path to name what comes back.

## Contents

```text
apps/frontend/src/foundation/transport/
├── audit_stream.rs  the live audit stream's protocol: steps, resume cursor, backoff, connect verdict
├── audit_stream/    the live audit stream's browser transport, with its page-owned abort handle
├── client/          the request verbs with their single retry, the failure types and the refresh policy
├── dto/             the wire types, one Rust shape per JSON body, re-exported flat
├── endpoints/       one typed call per route for event access, registration, reviews and the fleet
├── mod.rs           the module tree
├── sse.rs           the live server status stream the server intel page opens
├── sse_frames.rs    the incremental event-stream parser the audit stream reads with
├── tests/           unit tests for the parser, the audit stream and the status stream's teardown
└── token_provider.rs  `TokenProvider`: what a request needs from the session that holds the tokens
```

## How it works

```text
page ─▶ endpoints/ typed call ─┐
page ──────────────────────────┴─▶ client/ verb ─▶ /api/v1/<path> ─▶ dto/ type
server intel page ─▶ sse.rs ─▶ /api/v1/servers/{id}/status/stream ─▶ dto/ frame decoder
audit logs page ─▶ audit_stream.rs ─▶ /api/v1/admin/audit-logs/stream ─▶ sse_frames.rs ─▶ dto/ types
```

A caller passes `client/` a path relative to `/api/v1` and names the answer with a type from
`dto/`; for the routes that have one, it calls the typed function in `endpoints/`, which builds
the path and the body and calls the same verbs. Every authenticated call takes the session as a
`TokenProvider` (the session store implements it), so the transport sits below the session and
never names it. The client injects the provider's bearer token, refreshes a 401 once through the
provider's single-flight cell and locked refresh and retries once, and reports a failure as a
`(status, message)` pair, or as an `ApiRefusal` that keeps the reason for the callers that branch
on it. Every `/api/v1` request the app makes goes through `client/`, with three exceptions: the
status stream in `sse.rs`, the audit stream in `audit_stream.rs`, and the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s measured fetch of a
[mission](/documentation/glossary/g_to_m.md#mission), which reads a 2xx answer itself for its progress
bar and hands anything else to `api_get`.

`sse.rs` holds the one long-lived connection. `stream_server_status` opens
`GET /api/v1/servers/{id}/status/stream` as a bearer-authenticated fetch over a readable stream,
because the browser's own event source cannot carry an authorization header. It splits the bytes
on blank lines, decodes each frame with `dto::decode_server_status_frame`, and writes the caller's
status, connected and error signals; a frame it cannot read is logged to the console and raises
the error signal while the stream stays open. Opening a stream aborts the previous one, and
`abort_server_status_stream` (a function that captures nothing, so it can be a page's cleanup
callback) aborts it on route-leave.

`audit_stream.rs` and its `audit_stream/transport.rs` hold the audit page's connection.
`open_audit_stream` opens
`GET /api/v1/admin/audit-logs/stream` the same way, with the bearer token and, on every reconnect,
the last event id as `Last-Event-ID`, and returns an `AuditStreamHandle` the page owns and aborts on
cleanup; no global slot holds it. The bytes go through `sse_frames::SseParser`, which follows the
HTML standard's event-stream rules: UTF-8 split anywhere across reads, LF, CRLF or CR line breaks,
multi-line `data`, `id` carried across frames, `retry`, comments. `AuditStreamTracker` turns each
parsed item into an `AuditStreamStep` and keeps the resume cursor: `ready` records its id and
says whether the history must be (re)loaded (the connection opened without a cursor), each audit
line (`AuditLogEntry`) and `reset` record theirs, and `authorization_expired` revalidates the
session through the client (a `GET /me`, whose `401` spends the shared refresh) and reconnects at
once. After a drop the wait doubles from 1 s (or a longer server `retry`) to 30 s, plus up to a
quarter of itself as jitter, and resets once a connection delivers anything after `ready`
(`reconnect_delay_ms`). A `401` on connect revalidates once and retries at once; a second `401` or
a `403` stops the stream as `Offline`, and a `400` forgets the cursor (`connect_verdict`).

The transport half of the client, the endpoint calls and both streams compile for `wasm32` only;
the refresh policy, the endpoint paths, the wire types, the event-stream parser and the audit
stream's bookkeeping also compile into the native test build, so the native `cargo test` covers
them.

## Public surface

- `client`: the verbs `api_get`, `api_post`, `api_put`, `api_patch`, `api_delete`, `api_post_ok`,
  `api_post_raw` and `api_upload_file`; the refresh policy (`send_with_refresh`,
  `refresh_cross_tab`, `peer_rotation_supersedes`) and the `SingleFlight` cell type; the failure
  types `ApiErr` and `ApiRefusal`, and the readers `api_error_message`,
  `error_body_message` and `split_error_lines`. The four JSON `_keeping_refusal` verbs serve
  `endpoints/` alone; `api_post_form_keeping_refusal` sends a multipart form for the ballistics
  catalog upload page. `Fetched` and `ApiFailure` have no user outside `client/`.
- `dto`: every wire type, flat, and `dto::role`: the account tier ladder `Role` with
  `has_min_role` and `has_min_role_authed`.
- `token_provider::TokenProvider`: the session seam every authenticated call is generic over.
- `endpoints::<file>`: each route's path builder and typed call, and `encode_path_segment`, which
  pages use for the paths they build themselves.
- `sse::stream_server_status` and `sse::abort_server_status_stream`, for the server intel page.
- `audit_stream::open_audit_stream`, `AuditStreamCallbacks`, `AuditStreamHandle`,
  `AuditStreamState` and `OfflineReason`, for the audit logs page; `AuditStreamTracker`,
  `AuditStreamStep`, `connect_verdict`, `reconnect_delay_ms` and `parse_cursor` are its pure half.
- `sse_frames::SseParser`, `SseItem` and `SseMessage`: the event-stream parser.

## Boundaries

- Depends on: nothing above it in the foundation; `crate::foundation::test_support` in tests;
  `map_engine::data`, in `dto/`; `serde`, `serde_json`
  and `futures`; `gloo-net`, `gloo-timers`, `web-sys`, `js-sys` and `wasm-bindgen-futures` in the
  browser build.
- Used by:
  - `crate::foundation::auth`: the session is built from `dto::User`, `dto::role` and
    `dto::RefreshResponse`, the store implements `TokenProvider` and reads `dto::MeResponse`, and
    the session refresh sends through the client and applies its refresh policy;
  - `crate::foundation::route_table`, for `dto::role`; `crate::foundation::offline`;
  - the pages under `apps/frontend/src/pages/`, the server intel page's stream in
    `apps/frontend/src/pages/command_center/server_intel/page.rs` and the audit stream
    in `apps/frontend/src/pages/administration/audit_logs/page.rs` among them;
  - the Mission Creator under `apps/frontend/src/workspaces/editor/`.
- Rules: the wire types follow the API's models, and the API wins a disagreement (the golden round
  trips in `dto/tests/`); one page holds one live stream, torn down on leave
  (`class_r_sse_abort_teardown_exists` in `tests/sse.rs`,
  `audit_stream_transport_resumes_from_the_tracker_and_aborts_through_the_handle` in
  `tests/audit_stream.rs`); a frame that fails to decode is reported, never dropped in silence
  (`a_bad_payload_is_rejected_with_its_reason_not_silently_dropped` in
  `dto/tests/r_api_servers.rs`, `audit_stream_an_unreadable_row_is_rejected_and_skipped_by_the_cursor`);
  the parser matches the event-stream rules for every split of its input (`sse_*` in
  `tests/sse_frames.rs`).

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
- [Server intel page](/documentation/apps/frontend/pages/command_center/server_intel/server_intel_page.md)
  — the page that reads the live status stream.
- [Audit logs page](/documentation/apps/frontend/pages/administration/audit_logs/audit_logs_page.md)
  — the page that reads the live audit stream.
- [Frontend documentation](/documentation/apps/frontend/README.md#shared-foundations) — the shared foundations
  among the routes, pages and workspaces of the app.
