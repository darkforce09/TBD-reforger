# Offline service worker source

The source of `website-offline-service-worker`: five pure policy modules that the worker and the
page share, the library root that declares them, and the worker binary that applies them.

## Contents

```text
apps/website/offline-service-worker/src/
├── bin/                       the `offline_service_worker` binary
├── cache_names.rs             the build identifier, the four cache names and stale-cache selection
├── lib.rs                     the library root; declares the five policy modules
├── network_fallback.rs        when a saved copy answers (unreachable or 5xx), its marker header and date
├── offline_pack.rs            the terrain pack list from the manifest and the map tile index
├── range_slicing.rs           `Range: bytes=…` parsing and the 206 and 416 arithmetic
├── request_classification.rs  request classes, their strategies, caches and cache keys
└── tests/                     unit tests of the five policy modules
```

## How it works

The worker's fetch handler asks `request_classification::classify` for the class of each request,
takes the cache from `cache_names::CacheNames` and the key from
`request_classification::cache_key`, and, for a map asset with a `Range` header, asks
`range_slicing::plan_range_response` how to answer from the cached body. After a network-first
fetch it asks `network_fallback::prefers_saved_copy` whether the cached copy answers instead. The
page asks
`offline_pack::terrain_pack` which files to download and writes them under the same names and
keys. No module touches a browser type, so `cargo test` checks every decision natively.

## Public surface

The five modules `cache_names`, `request_classification`, `range_slicing`, `network_fallback` and
`offline_pack`, as the crate README lists them.

## Boundaries

- Depends on: `serde`, `serde_json` and `url`.
- Used by: the binary in `bin/` and the frontend's offline core.
- Rules: no module imports `web_sys`, `js_sys` or `wasm_bindgen`, so the library compiles and tests
  on the host; tests live in `tests/`, one file per module.
