# Frontend offline core

The `frontend_offline` crate: the page's half of offline use. It registers the offline service
worker at boot, downloads the offline pack on the first visit of the mortar calculator in a page
lifetime — the app shell, the ballistics catalogs, the icon font and the Everon terrain — into the
caches the worker answers from, reads a saved copy back when the server cannot answer, and
publishes where the pack stands as page-wide signals mirrored onto the document element.

## Contents

```text
crates/frontend/foundation/frontend_offline/
├── Cargo.toml  the package: `frontend_transport`, `frontend_api_dtos`, `offline_cache_policy`, `leptos`, browser crates for wasm32, layout tier 9
└── src/        the pack status and its signals, the pack list, the download, the saved copies, the worker registration, the quota check
```

## How it works

The app's entry point calls `service_worker_registration::register_at_boot`, which registers
`/service_worker.js` under the running build's bundle hash, and mounts
`offline_pack::OfflinePackRouteWatcher` inside the router, which starts the download on the
mortar calculator's route. The download lists every file through `offline_manifest`, each under
the cache and key the worker's own classification names (`offline_cache_policy`), checks the
storage quota first, fetches six files at a time and publishes its state, its optional-file
coverage and whether it refreshed the saved copy through `status_signals`. The
[source tree README](src/README.md) walks through the download, the four document-element
attributes and the saved copies.

The browser halves (the download, the Cache Storage reads, the registration, the storage estimate)
are gated to the wasm32 build inside each file; the pack list, the progress and outcome rules, the
status words and their signals compile on every target, so their tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_offline   # the status words, the signals, the pack list, the outcome rules, the quota and the build id
```

## Configuration

None: no feature, no environment variable. The pack route is `/tools/mortar`
(`offline_pack::OFFLINE_PACK_ROUTE`), the terrain is `everon`
(`offline_manifest::OFFLINE_TERRAIN_ID`), and the cache names come from `offline_cache_policy`.

## Public surface

- `pack_status`: `OfflineState`, `OfflineStatus`, `OptionalFiles`, `PackRefresh` and the four
  attribute names, re-exported at the crate root.
- `status_signals`: `offline_status`, `offline_optional_files` and `offline_pack_refresh`,
  re-exported at the crate root.
- `offline_manifest`, `offline_pack`, `saved_copies`, `service_worker_registration`,
  `storage_quota`: the pack list, the route watcher and outcome rules, the saved-copy read, the
  registration and the quota check.
- `error`: `Error` and `Result`, re-exported at the crate root.
- `prelude`: the status words and the signal readers.

## Boundaries

- Depends on: `frontend_transport`, `frontend_api_dtos`, `offline_cache_policy`, `leptos`,
  `leptos_router`, `serde`, `serde_json`, `thiserror`, `url`; `web-sys`, `js-sys`,
  `wasm-bindgen`, `wasm-bindgen-futures`, `futures` and `gloo-timers` in the wasm32 build only.
- Used by: the single-page app (`apps/frontend`): its entry point and the mortar calculator; the
  offline browser gate reads the document-element attributes this crate writes.
- Rules: the pack writes every file under the cache and key the worker reads; the attribute words
  never change, since the browser gate reads them; the crate depends on no frontend crate above
  the foundation crates it names (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Offline service worker](/apps/offline_service_worker/README.md) — the worker and how it answers
  each request class.
- [Offline cache policy](/crates/contracts/offline_cache_policy/README.md) — the request classes,
  caches and terrain pack list the worker and the page share.
- [Offline mortar page runbook](/documentation/runbooks/offline_mortar_page.md) — using the mortar
  calculator offline.
