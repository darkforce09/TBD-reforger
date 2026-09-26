# API layer

Everything the app talks to the [API](/documentation_v2/glossary/a_to_f.md#api) through: the HTTP client,
the typed endpoint calls, the wire types they carry, and the
[SSE](/documentation_v2/glossary/n_to_z.md#sse) stream of one server's live status. A page imports one
path to fetch and one path to name what comes back.

## Contents

```text
apps/website/frontend/src/v2/core/api/
├── client/     the request verbs with their single retry, the failure types and the refresh policy
├── dto/        the wire types, one Rust shape per JSON body, re-exported flat
├── endpoints/  one typed call per route for event access, registration, reviews and the fleet
├── mod.rs      the module tree
├── sse.rs      the live server status stream the server intel page opens
└── tests/      unit tests for the stream's teardown
```

## How it works

```text
page ─▶ endpoints/ typed call ─┐
page ──────────────────────────┴─▶ client/ verb ─▶ /api/v1/<path> ─▶ dto/ type
server intel page ─▶ sse.rs ─▶ /api/v1/servers/{id}/status/stream ─▶ dto/ frame decoder
```

A caller passes `client/` a path relative to `/api/v1` and names the answer with a type from
`dto/`; for the routes that have one, it calls the typed function in `endpoints/`, which builds
the path and the body and calls the same verbs. The client injects the bearer token, refreshes a
401 once through the shared single flight and retries once, and reports a failure as a
`(status, message)` pair, or as an `ApiRefusal` that keeps the reason for the callers that branch
on it. Every `/api/v1` request the app makes goes through `client/`, with two exceptions: the
status stream in `sse.rs`, and the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s measured fetch of a
[mission](/documentation_v2/glossary/g_to_m.md#mission), which reads a 2xx answer itself for its progress
bar and hands anything else to `api_get`.

`sse.rs` holds the one long-lived connection. `stream_server_status` opens
`GET /api/v1/servers/{id}/status/stream` as a bearer-authenticated fetch over a readable stream,
because the browser's own event source cannot carry an authorization header. It splits the bytes
on blank lines, decodes each frame with `dto::decode_server_status_frame`, and writes the caller's
status, connected and error signals; a frame it cannot read is logged to the console and raises
the error signal while the stream stays open. Opening a stream aborts the previous one, and
`abort_server_status_stream` (a function that captures nothing, so it can be a page's cleanup
callback) aborts it on route-leave.

The transport half of the client, the endpoint calls and the stream compile for `wasm32` only;
the refresh policy, the endpoint paths and the wire types compile natively, so the native
`cargo test` covers them.

## Public surface

- `client`: the verbs `api_get`, `api_post`, `api_put`, `api_patch`, `api_delete`, `api_post_ok`,
  `api_post_raw` and `api_upload_file`; `bootstrap`, which the app layout spawns at startup; the
  failure types `ApiErr` and `ApiRefusal`, and the readers `api_error_message`,
  `error_body_message` and `split_error_lines`. The four `_keeping_refusal` verbs serve
  `endpoints/` alone, and `Fetched` and `ApiFailure` have no user outside `client/`.
- `dto`: every wire type, flat.
- `endpoints::<file>`: each route's path builder and typed call, and `encode_path_segment`, which
  pages use for the paths they build themselves.
- `sse::stream_server_status` and `sse::abort_server_status_stream`, for the server intel page.

## Boundaries

- Depends on: `crate::v2::core::auth` (the `AuthStore` signals, the stored session, the single
  flight, the refresh transaction); `website_map_engine::data`, in `dto/`; `serde`, `serde_json`
  and `futures`; `gloo-net`, `gloo-timers`, `web-sys`, `js-sys` and `wasm-bindgen-futures` in the
  browser build.
- Used by:
  - `crate::v2::core::auth`: the store reads `dto::MeResponse`, and the session takes the
    client's refresh lock;
  - the pages under `apps/website/frontend/src/v2/pages/`, the app layout's `bootstrap` in
    `apps/website/frontend/src/v2/pages/navigation/layout.rs` and the server intel page's stream
    in `apps/website/frontend/src/v2/pages/command_center/server_intel/page.rs` among them;
  - the Mission Creator under `apps/website/frontend/src/v2/apps/editor/`.
- Rules: the wire types follow the API's models, and the API wins a disagreement (the golden round
  trips in `dto/tests/`); one page holds one live stream, torn down on leave
  (`class_r_sse_abort_teardown_exists` in `tests/sse.rs`); a frame that fails to decode is
  reported, never dropped in silence
  (`a_bad_payload_is_rejected_with_its_reason_not_silently_dropped` in
  `dto/tests/r_api_servers.rs`).

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [Server intel page](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md)
  — the page that reads the live status stream.
- [Frontend documentation](/documentation_v2/website/frontend/README.md#shared-foundations) — the shared foundations
  among the routes, pages and workspaces of the app.
