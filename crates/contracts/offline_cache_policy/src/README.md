# Offline cache policy source

The source of `offline_cache_policy`: four pure policy modules that the worker and the page share,
the terrain identifier and the error they use, and the crate root that declares them.

## Contents

```text
crates/contracts/offline_cache_policy/src/
├── cache_names.rs             the build identifier, the four cache names and stale-cache selection
├── error.rs                   `Error` and `Result`: why a terrain pack cannot be built
├── lib.rs                     the crate root: module header, `mod` lines and re-exports
├── network_fallback.rs        when a saved copy answers (unreachable or 5xx), its marker header and date
├── offline_pack.rs            the terrain pack list from the manifest and the map tile index
├── prelude.rs                 the names most callers import
├── request_classification.rs  request classes, their strategies, caches and cache keys
├── terrain_id.rs              `TerrainId`: a terrain's identifier, serialised as the bare string
└── tests/                     unit tests, one file per module
```

## How it works

`request_classification::classify` sorts a request into a class with a strategy; the class names
its cache through `cache_names::CacheNames` and its key through `request_classification::cache_key`.
`network_fallback::prefers_saved_copy` decides when a network-first request falls back to its
cached copy. `offline_pack::terrain_pack` reads a served terrain manifest and its map tile index
into the list of files the page downloads, refusing a malformed input with an `error::Error` and
marking a partial index incomplete. `terrain_id::TerrainId` names the terrain in the pack, the
index and the served-path helpers.

## Boundaries

- Depends on: `serde`, `serde_json`, `thiserror` and `url`.
- Used by: the worker's handlers in `apps/offline_service_worker/src/` and the frontend's offline
  core.
- Rules: no module imports `web_sys`, `js_sys` or `wasm_bindgen`; tests live in `tests/`, one file
  per module.
