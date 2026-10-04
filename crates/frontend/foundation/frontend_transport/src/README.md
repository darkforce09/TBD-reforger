# Transport source tree

The modules of `frontend_transport`, everything the app talks to the
[API](/documentation/glossary/a_to_f.md#api) through: the HTTP client, the typed endpoint calls,
and the two [SSE](/documentation/glossary/n_to_z.md#sse) streams: one server's live status and the
live audit log. The wire types they carry are `frontend_api_dtos`'s. A page imports one path to
fetch and one path to name what comes back.

## Contents

```text
crates/frontend/foundation/frontend_transport/src/
├── audit_stream.rs  the live audit stream's protocol: steps, resume cursor, backoff, connect verdict
├── audit_stream/    the live audit stream's browser transport, with its page-owned abort handle
├── client/          the request verbs with their single retry, the error body readers and the refresh policy
├── endpoints/       one typed call per route for event access, registration, reviews and the fleet
├── error.rs         `Error`: why a request produced no data — session over, refused, never sent
├── lib.rs           the module tree; re-exports `Error` and `Result`
├── prelude.rs       `Error`, `Fetched` and `TokenProvider`
├── sse.rs           the live server status stream the server intel page opens
├── sse_frames.rs    the incremental event-stream parser the audit stream reads with
├── tests/           unit tests for the parser, the audit stream and the stream teardown; source pins
└── token_provider.rs  `TokenProvider`: what a request needs from the session that holds the tokens
```

## How it works

```text
page ─▶ endpoints/ typed call ─┐
page ──────────────────────────┴─▶ client/ verb ─▶ /api/v1/<path> ─▶ frontend_api_dtos type
server intel page ─▶ sse.rs ─▶ /api/v1/servers/{id}/status/stream ─▶ frontend_api_dtos frame decoder
audit logs page ─▶ audit_stream.rs ─▶ /api/v1/admin/audit-logs/stream ─▶ sse_frames.rs ─▶ frontend_api_dtos types
```

A caller passes `client/` a path relative to `/api/v1` and names the answer with a type from
`frontend_api_dtos`; for the routes that have one, it calls the typed function in `endpoints/`, which builds
the path and the body and calls the same verbs. Every authenticated call takes the session as a
`TokenProvider` (the session store implements it), so the transport sits below the session and
never names it. The client injects the provider's bearer token, refreshes a 401 once through the
provider's single-flight cell and locked refresh and retries once, and reports a failure as the
crate's `Error` (`error.rs`), which the refusal-keeping verbs fill with the reason the refusal
body names for the callers that branch on it. Every `/api/v1` request the app makes goes through `client/`, with three exceptions: the
status stream in `sse.rs`, the audit stream in `audit_stream.rs`, and the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s measured fetch of a
[mission](/documentation/glossary/g_to_m.md#mission), which reads a 2xx answer itself for its progress
bar and hands anything else to `api_get`.

`sse.rs` holds the one long-lived connection. `stream_server_status` opens
`GET /api/v1/servers/{id}/status/stream` as a bearer-authenticated fetch over a readable stream,
because the browser's own event source cannot carry an authorization header. It splits the bytes
on blank lines, decodes each frame with `frontend_api_dtos::decode_server_status_frame`, and writes the caller's
status, connected and error signals; a frame it cannot read is logged to the console and raises
the error signal while the stream stays open. Opening a stream aborts the previous one, and
`abort_server_status_stream` (a function that captures nothing, so it can be a page's cleanup
callback) aborts it on route-leave. A browser that cannot build the abort controller fails the
call with an `Error` and opens nothing; a step that fails later writes its reason to the error
signal.

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

The request verbs, the endpoint calls and the browser halves of both streams compile for
`wasm32` only, because they call the browser; the error, the refresh policy, the endpoint
paths, the event-stream parser and the audit stream's bookkeeping compile on every target, so
`cargo test -p frontend_transport` covers them natively.

## Public surface

- `client`: the verbs `api_get`, `api_post`, `api_put`, `api_patch`, `api_delete`, `api_post_ok`,
  `api_post_raw` and `api_upload_file`; the refresh policy (`send_with_refresh`,
  `refresh_cross_tab`, `peer_rotation_supersedes`) and the `SingleFlight` cell type; the error
  body readers `error_body_message` and `split_error_lines`, and the refusal decoder
  `decode_answer`; the anonymous `public_reads::public_get` and the generic
  `rate_limit_retry`. The four JSON `_keeping_refusal` verbs serve
  `endpoints/` alone; `api_post_form_keeping_refusal` sends a multipart form for the ballistics
  catalog upload page. `Fetched` wraps a settled request as data or an `Error`.
- `Error` and `Result` (`error.rs`): the one failure type of every verb, endpoint call, public
  read and stream setup — `SessionExpired`, `Http` (status, sentence, `details`), `Transport`,
  `Failed`, `Cancelled`, `NotASitePath` and `PublicRefusal` — read through `status`, `message`,
  `code`, `detail` and `message_or`; each variant's text is the sentence the interface shows.
- `token_provider::TokenProvider`: the session seam every authenticated call is generic over.
- `endpoints::<file>`: each route's path builder and typed call, and `encode_path_segment`, which
  pages use for the paths they build themselves.
- `sse::stream_server_status` and `sse::abort_server_status_stream`, for the server intel page.
- `audit_stream::open_audit_stream`, `AuditStreamCallbacks`, `AuditStreamHandle`,
  `AuditStreamState` and `OfflineReason`, for the audit logs page; `AuditStreamTracker`,
  `AuditStreamStep`, `connect_verdict`, `reconnect_delay_ms` and `parse_cursor` are its pure half.
- `sse_frames::SseParser`, `SseItem` and `SseMessage`: the event-stream parser.

## Boundaries

- Depends on: `frontend_api_dtos`, for the wire types; `frontend_test_support` in tests; `leptos`,
  `serde`, `serde_json`, `futures` and `thiserror`; `gloo-net`, `gloo-timers`, `web-sys`,
  `js-sys`, `wasm-bindgen` and `wasm-bindgen-futures` in the browser build.
- Used by:
  - the session in `crates/frontend/foundation/frontend_session/src/`: the store implements `TokenProvider`,
    and the session refresh sends through the client and applies its refresh policy;
  - the offline pack in `crates/frontend/foundation/frontend_offline/src/`;
  - the page crates under `crates/frontend/pages/`, the server intel page's stream in
    `crates/frontend/pages/command_center_pages/src/server_intel/page.rs` and the audit stream
    in `crates/frontend/pages/administration_pages/src/audit_logs/page.rs` among them;
  - the Mission Creator under `crates/frontend/workspaces/mission_creator_workspace/src/`.
- Rules: one page holds one live stream, torn down on leave
  (`class_r_sse_abort_teardown_exists` in `tests/sse.rs`,
  `audit_stream_transport_resumes_from_the_tracker_and_aborts_through_the_handle` in
  `tests/audit_stream.rs`); a frame that fails to decode is reported, never dropped in silence
  (`a_bad_payload_is_rejected_with_its_reason_not_silently_dropped` in `frontend_api_dtos`,
  `audit_stream_an_unreadable_row_is_rejected_and_skipped_by_the_cursor`);
  the parser matches the event-stream rules for every split of its input (`sse_*` in
  `tests/sse_frames.rs`).

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
- [Server intel page](/documentation/crates/frontend/pages/command_center_pages/server_intel/server_intel_page.md)
  — the page that reads the live status stream.
- [Audit logs page](/documentation/crates/frontend/pages/administration_pages/audit_logs/audit_logs_page.md)
  — the page that reads the live audit stream.
- [Frontend transport](/crates/frontend/foundation/frontend_transport/README.md) — the crate, its
  dependencies and how to run its tests.
