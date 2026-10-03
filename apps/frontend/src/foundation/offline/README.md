# Offline core

The page's half of offline use: it registers the offline service worker at boot and, on the first
visit of the mortar calculator (`/tools/mortar`) in a page lifetime, downloads the offline pack —
the app shell, the ballistics catalogs, the Material Symbols icon font and the Everon terrain —
into the caches the worker answers from. The cross-origin icon font is the pack's only optional
part: a browser that cannot reach its host still gets a `ready` pack, flagged as missing it. When
the server cannot answer (unreachable, or a `5xx` from a proxy whose API is down) the saved copies
stand in: a re-run pack stays `ready` while every essential file is still cached, and pages read
the saved catalog copies back through `saved_copies`. The worker itself lives in
`apps/offline_service_worker/`; the policy both halves share is the `offline_cache_policy` crate
(`crates/contracts/offline_cache_policy/`).

## Contents

```text
apps/frontend/src/foundation/offline/
├── mod.rs                          `OfflineState`, `OfflineStatus`, `OptionalFiles`, `PackRefresh`, their signals and four DOM attributes
├── offline_manifest.rs             `OfflineTarget`: every pack file with the cache class and key the worker reads, essential or optional
├── offline_pack.rs                 the route trigger, `PackProgress`, `FailedFiles`, `final_outcome`
├── pack_download.rs                the browser download (wasm32 only): listing, quota, six-at-a-time fetch and store, kept saved copies
├── saved_copies.rs                 `read_text`: a cache-backed file from the server, or its saved copy when the server cannot answer
├── service_worker_registration.rs  the build identifier from the bundle hash, `register_at_boot`
├── storage_quota.rs                `quota_verdict` over `navigator.storage.estimate()`, `persist()`
└── tests/                          unit tests for the state words, targets, saved copies, quota, build id, progress and outcome
```

## How it works

```text
main.rs start_app
├── register_at_boot ─▶ no service worker / cache storage / secure context ─▶ unsupported
│                    └▶ navigator.serviceWorker.register(/service_worker.js?build=<bundle hash>)
└── <Router> … <OfflinePackRouteWatcher/>
        pathname is /tools/mortar[/…] ─▶ ensure_offline_pack (once per page lifetime)
            downloading 0
            ├─ document link/script URLs ─▶ shell files + icon font stylesheet ─▶ its gstatic font files
            │     (an unreadable stylesheet leaves its font files unlisted: an optional failure)
            ├─ GET /api/v1/ballistics-catalogs ─▶ the list + every /versions/{v}
            ├─ GET /map-assets/everon/manifest.json + tiles/map/index.json (404 = no index)
            │     (each listing through saved_copies::read_text: unreachable or 5xx ─▶ the saved copy)
            │     └▶ offline_pack::terrain_pack: manifest, DEM, tbd-sat, every map tile
            ├─ drop what the cache already holds (the catalog list is always refreshed)
            ├─ storage estimate: free < declared bytes still missing ─▶ quota-short (nothing fetched)
            ├─ navigator.storage.persist()
            └─ fetch + cache.put, six at a time, progress after each file
                  unreachable or 5xx on a file still cached ─▶ kept saved copy (never a failure)
                  essential file missing from the cache ─▶ failed; index missing or partial ─▶
                  incomplete; else ready
                  optional (icon font) failure ─▶ data-offline-optional missing, else complete
                  any kept saved copy ─▶ data-offline-refresh kept-saved-copy, else refreshed
```

Each file lands in the cache the worker's `RequestClass::cache_name` names for its URL, under the
worker's `cache_key`, with every response header it arrived with. The shell cache carries the
build identifier: the hash Trunk puts in `frontend-<hash>.js`, which is also the `build`
query the worker is registered under, so page and worker agree on the cache names.

Every file is essential except the cross-origin icon font's stylesheet and font files
(`OfflineTarget::is_optional`, the worker's `IconFont` class): without them the calculator still
works offline, its icons show as their ligature text. The state is published to the
`ArcRwSignal` behind `offline_status()`, the optional-file coverage to the one behind
`offline_optional_files()` and the refresh outcome to the one behind `offline_pack_refresh()`
(both before the final state), and all are mirrored onto the document element (`<html>`) as four
attributes:

| Attribute | Values |
|---|---|
| `data-offline-state` | `idle`, `downloading`, `ready`, `incomplete`, `quota-short`, `unsupported`, `failed` |
| `data-offline-progress` | an integer 0 to 100; 100 only once every file still missing at the start is stored, so a `ready` pack missing an optional file stays below 100 |
| `data-offline-optional` | absent until a download finishes; then `complete` (the icon font is cached) or `missing` |
| `data-offline-refresh` | absent until a download finishes; then `refreshed`, or `kept-saved-copy` when the server could not answer a listing or file whose saved copy stays in use |

A saved copy is a response the pack stored. The worker answers a network-first request with it
when the network is unreachable or answers a gateway or server failure, adding the header
`x-served-from-offline-cache: 1`; without a controlling worker, `saved_copies::read_text` reads the
same entry from Cache Storage itself. Either way the copy's `Date` header gives the date the page
words ("offline copy from 28 Sep 2026, 14:05 UTC"). `read_text` first waits out a `429` and sends
again through the API client's rate-limit retry (`core::api::client::rate_limit_retry`, three
sends at most). A `4xx` is never replaced by a saved copy, and
a response marked as a saved copy is never stored again. The mortar calculator reads its catalogs
through `read_text` under `offline_manifest::catalog_list_target` and
`offline_manifest::catalog_version_target`, the same functions `catalog_targets` writes the pack
under.

## Public surface

- `mod.rs`: `OfflineState` (`attribute_value`), `OfflineStatus` (`IDLE`), `offline_status`,
  `OptionalFiles` (`attribute_value`), `offline_optional_files`, `PackRefresh`
  (`attribute_value`), `offline_pack_refresh`, `OFFLINE_STATE_ATTRIBUTE`,
  `OFFLINE_PROGRESS_ATTRIBUTE`, `OFFLINE_OPTIONAL_ATTRIBUTE`, `OFFLINE_REFRESH_ATTRIBUTE`.
- `offline_manifest`: `OfflineTarget` (`is_optional`), `target_for`, `document_targets`, `catalog_targets`,
  `catalog_list_target`, `catalog_version_target`,
  `icon_font_file_targets`, `terrain_targets`, `declared_bytes`, `CatalogListUnreadable`,
  `OFFLINE_TERRAIN_ID`, `MAP_ASSETS_ROOT`, `ICON_FONT_FILE_ORIGIN`.
- `offline_pack`: `OfflinePackRouteWatcher`, `ensure_offline_pack`, `is_offline_pack_route`,
  `PackProgress`, `FailedFiles` (`record`, `record_refresh_failure`, `record_kept_saved_copy`),
  `PackOutcome`, `final_outcome`, `OFFLINE_PACK_ROUTE`, `PARALLEL_DOWNLOADS`.
- `saved_copies`: `ReadSource`, `TextRead`, `source_of_answer`; browser only: `read_text`,
  `saved_copy_date`.
- `service_worker_registration`: `build_id_from_asset_urls`, `APP_BUNDLE_PREFIX`; browser only:
  `register_at_boot`, `current_build_id`, `document_asset_urls`, `offline_supported`.
- `storage_quota`: `StorageEstimate`, `QuotaVerdict`, `quota_verdict`, `bytes_from_js_number`;
  browser only: `estimate`, `request_persistence`.

## Boundaries

- Depends on: `offline_cache_policy` (`cache_names`, `request_classification`, `offline_pack`,
  `network_fallback`, `TerrainId`); `leptos` and `leptos_router` (the signal and the route
  watcher); `web-sys`,
  `js-sys`, `wasm-bindgen-futures`, `futures` and `url`.
- Used by: `src/main.rs`, which registers the worker and mounts the watcher; the mortar
  calculator, which renders `offline_status` and `offline_optional_files`; the offline browser
  gate, which waits on `data-offline-state` and reads `data-offline-optional`.
- Rules:
  - nothing here imports from `pages` or `apps`;
  - a URL the worker passes through is never a pack file (`target_for_refuses_passthrough_urls`);
  - an incomplete pack is never `ready`, and an essential failure is never `ready`
    (`final_outcome_is_failed_whenever_an_essential_file_failed`);
  - a refresh the server could not answer keeps a cached essential file and the pack `ready`, and
    is a failure when the saved copy is missing or the server refused
    (`a_refresh_the_server_could_not_answer_keeps_a_cached_essential_file_and_the_pack_ready`,
    `a_refresh_failure_is_a_failure_when_the_saved_copy_is_missing_or_the_server_refused`);
  - the page reads a catalog's saved copy under the key the pack writes
    (`the_catalog_list_and_version_targets_are_the_keys_catalog_targets_writes`);
  - a failure of only the optional icon font is `ready` with `missing`
    (`final_outcome_is_ready_with_missing_optional_files_when_only_the_icon_font_failed`);
  - the download never starts when the files still missing exceed the free storage
    (`quota_verdict_refuses_a_download_larger_than_the_free_space`).

## Related documentation

- [Offline service worker](/apps/offline_service_worker/README.md) — the worker and how it
  answers each request class.
- [Offline cache policy](/crates/contracts/offline_cache_policy/README.md) — the request classes,
  caches and terrain pack list the worker and the page share.
- [Contract definitions](/contracts/definitions/README.md) — `map-tile-index` and
  `ballistics-catalog`, the two server documents the pack list reads.
