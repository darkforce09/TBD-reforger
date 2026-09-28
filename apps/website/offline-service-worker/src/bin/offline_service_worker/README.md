# Offline service worker binary

The service worker itself: the Rust handlers for `install`, `activate` and `fetch` that the
loader `apps/website/frontend/service_worker.js` calls.

## Contents

```text
apps/website/offline-service-worker/src/bin/offline_service_worker/
├── cached_range_response.rs  a cached full body answered as the full body, a 206 slice or a 416
├── fetch_handling.rs         `on_fetch`: cache-first, network-first and passthrough
├── lifecycle_events.rs       `on_install` precaches the shell; `on_activate` deletes stale caches
├── main.rs                   the binary root: wasm32-only modules and an empty `main`
└── worker_scope.rs           the global scope, this build's cache names, Cache Storage helpers
```

## How it works

The loader registers the three listeners synchronously and calls the exports once the module is
ready. `on_install` opens this build's shell cache, stores `/` and `/manifest.webmanifest` and
skips waiting. `on_activate` deletes every `tbd-offline-*` cache the current build does not own and
claims the open pages. `on_fetch` classifies the request and answers it:

```text
classify ─┬─ passthrough ─────────────▶ fetch(request)
          ├─ cache-first ─ hit ───────▶ cached response (map asset + Range ─▶ 206 / 416 slice)
          │               └ miss ─────▶ fetch; store a copy under waitUntil unless it was a Range
          └─ network-first ─ answered ─▶ response; store a copy under waitUntil
                            └ unreachable or 5xx ─▶ cached copy + x-served-from-offline-cache: 1,
                                                    or the 5xx / the network error when none
```

A network-first request (the catalog list, the shell document) falls back to its cached copy when
`network_fallback::prefers_saved_copy` says so: the network is unreachable or answers a gateway or
server failure (`502`, `503`, `504`, any `5xx`), as a proxy does while the API behind it is down. A
`4xx` is the server's real answer and reaches the page unchanged. The copy it answers with carries
every stored header plus `x-served-from-offline-cache: 1`, so the page can word it as the offline
copy; the cache entry itself is never marked.

Writes run under `event.waitUntil` so no response waits for a write, and a failed write (an
exhausted quota) is dropped. A `206` slices the cached `Blob`, so the 150 MB satellite mosaic is
never read into memory whole.

## Boundaries

- Depends on: the crate's library (`cache_names`, `request_classification`, `range_slicing`,
  `network_fallback`);
  `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys`.
- Used by: `apps/website/frontend/service_worker.js`, through the exports `on_install`,
  `on_activate` and `on_fetch`.
- Rules: only `200` responses (and opaque icon-font responses) are stored, the shell document only
  when it is HTML; stored and returned responses keep every header; the handlers hold no decision
  the library does not make.
