# Offline cache policy

The `offline_cache_policy` crate: every decision of the offline service worker as pure functions
with no browser type — which cache holds a response, how each request is served, when a saved copy
answers instead of the network, and which terrain files make up the offline pack. The service
worker (`apps/offline_service_worker`) applies it in the browser; the single-page app links it to
download the offline pack into the same caches under the same keys.

## Contents

```text
crates/contracts/offline_cache_policy/
├── Cargo.toml  the package: `serde`, `serde_json`, `thiserror`, `url`; layout tier 0
└── src/        the four policy modules, the terrain identifier, the error and the prelude
```

## How it works

The worker's fetch handler classifies each request (`request_classification::classify`), takes the
cache from `cache_names::CacheNames` and the key from `request_classification::cache_key`, and
after a network-first fetch asks `network_fallback::prefers_saved_copy` whether the cached copy
answers instead. The page names the build it registers (`cache_names::BuildId`), builds the same
cache names, and asks `offline_pack::terrain_pack` which terrain files to download and store under
those names and keys. Only the shell cache carries the build identifier, so a new build replaces
the shell and keeps the map assets. No module touches a browser type, so `cargo test` checks every
decision natively.

## Getting started

Run from the repository root:

```bash
cargo test -p offline_cache_policy   # every policy decision, native; reads the committed manifests
```

## Configuration

None: the crate reads no environment variable and declares no feature. The unit tests read the
committed Everon and Arland manifests under `assets/terrains/`.

## Public surface

- `cache_names`: `BuildId` (`parse`, `from_script_query`, `script_url`), `CacheNames`
  (`for_build`, `is_stale`, `stale_names`), `CACHE_PREFIX`, `SERVICE_WORKER_SCRIPT_PATH`.
- `request_classification`: `classify`, `cache_key`, `RequestClass`, `CacheStrategy`,
  `InterceptedRequest`, `PassthroughReason`.
- `network_fallback`: `prefers_saved_copy`, `is_gateway_or_server_failure`, `NetworkAnswer`,
  `SAVED_COPY_HEADER`, `saved_on_from_date_header`.
- `offline_pack`: `terrain_pack`, `manifest_url`, `tile_index_url`, `OfflinePack`, `PackEntry`,
  `PackCompleteness`, `IncompleteReason`, and `MapTileIndex`, the shape of
  `/map-assets/<terrain>/tiles/map/index.json`
  (`{"schemaVersion":1,"terrainId":"everon","tiles":[{"z":0,"x":0,"y":0,"bytes":1234}]}`).
- `TerrainId`: a terrain's identifier, serialised as the bare string.
- `Error` and `Result`: why `terrain_pack` cannot build a pack.
- `prelude`: the names most callers import.

## Boundaries

- Depends on: `serde`, `serde_json`, `thiserror` and `url`; no workspace crate.
- Used by: `apps/offline_service_worker` (the worker's handlers), the frontend's offline core
  (`crates/frontend/foundation/frontend_offline/`) and the mortar calculator's catalog source
  tests in `apps/frontend`.
- Rules:
  - no module imports `web_sys`, `js_sys` or `wasm_bindgen`, so the crate compiles and tests on
    the host; contracts tier, so it depends on no workspace crate outside the foundation tier
    (`cargo xtask verify crate-tiers`);
  - a missing or partial map tile index makes the pack incomplete, never complete
    (`a_missing_empty_or_partial_index_is_incomplete_and_never_complete`);
  - a gateway or server failure answers from the cached copy and a `4xx` never does
    (`gateway_and_server_failures_prefer_the_saved_copy`,
    `successes_redirects_and_refusals_are_never_masked`);
  - `TerrainId` keeps the wire shape of a plain string
    (`terrain_id_serialises_as_the_bare_string_in_both_directions`).

## Related documentation

- [Offline mortar page runbook](/documentation/runbooks/offline_mortar_page.md) — checking the
  offline pack and the saved copies in a browser.
