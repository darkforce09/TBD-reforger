# Frontend transport

The `frontend_transport` crate: everything the single-page app talks to the
[API](/documentation/glossary/a_to_f.md#api) through. It holds the request verbs with their
refresh-and-retry policy and the crate's one error, one typed call per route that has one, the live
server status stream and the audit log stream over [SSE](/documentation/glossary/n_to_z.md#sse),
and the `TokenProvider` seam the session implements. The wire types it carries are
`frontend_api_dtos`'s.

## Contents

```text
crates/frontend/foundation/frontend_transport/
├── Cargo.toml  the package: `frontend_api_dtos`, `leptos`, the browser crates for wasm32, layout tier 8
└── src/        the client, the endpoint calls, the two streams, the frame parser and the crate error
```

## How it works

A page passes a request verb a path relative to `/api/v1` and names the answer with a
`frontend_api_dtos` type, or calls the typed function in `endpoints` for the routes that have one.
Every authenticated call takes the session as a `TokenProvider`, so the transport sits below the
session and never names it: the verb injects the provider's bearer token, refreshes a `401` once
through the provider's single-flight cell and retries once, and reports a failure as the crate's
one `Error`: the session is over, the API refused (with its status, its sentence and, from the
refusal-keeping verbs, the reason its body names), the request never reached the API, the answer
could not be used, or the read was cancelled. `Fetched` turns a settled request into data or that
named reason, never an empty state. The [source tree README](src/README.md) describes each module and
both streams.

Only code that calls the browser directly is gated to the wasm32 build: the request verbs, the
public reads and the browser halves of both streams. The error, the refresh policy, the
endpoint paths, the event-stream parser and the audit stream's bookkeeping compile on every
target.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_transport   # the refresh policy, the endpoint paths, the parser, the streams' bookkeeping
```

## Configuration

None: no feature, no environment variable. Every request goes to the same-origin `/api/v1`
(`client::API_BASE`); the development server proxies it to the API.

## Public surface

- `client`: the request verbs (wasm32), `API_BASE`, `MAX_ERROR_DETAILS`, the refresh policy,
  `SingleFlight`, `Fetched` and the error body readers.
- `Error` and `Result` (`error`): why a request produced no data; every request verb, endpoint
  call, public read and stream setup returns `Result`. A caller branches on its variants or on
  `status`, `code` and `detail`, and words it with `message_or`.
- `endpoints`: each route's path builder and typed call, and `encode_path_segment`.
- `token_provider::TokenProvider`: the session seam.
- `sse` and `audit_stream`: the two live streams; `sse_frames`: the event-stream parser.
- `prelude`: `Fetched`, `Error` and `TokenProvider`.

## Boundaries

- Depends on: `frontend_api_dtos`, `leptos`, `futures`, `serde`, `serde_json`, `thiserror`;
  `gloo-net`, `gloo-timers`, `web-sys`, `js-sys`, `wasm-bindgen` and `wasm-bindgen-futures` in
  the wasm32 build only.
- Used by: the single-page app (`apps/frontend`): the session, whose store implements
  `TokenProvider` and whose refresh sends through the client; the offline pack; the pages; the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator); the debug benches' public
  reads.
- Rules: exactly one refresh-and-retry per request; one page holds one live stream, torn down on
  leave; a frame that fails to decode is reported, never dropped in silence; the crate depends on
  no frontend crate above `frontend_api_dtos` (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
- [Frontend documentation](/documentation/apps/frontend/README.md#shared-foundations) — the shared
  foundations among the routes, pages and workspaces of the app.
