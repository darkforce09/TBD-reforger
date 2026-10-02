# Offline service worker

The `website-offline-service-worker` crate: the service worker that keeps the single-page app, the
ballistics catalogs and the Everon map working with no network. Its library holds the offline
policy as pure functions that the app also links; its binary applies that policy inside the
browser's service worker.

## Contents

```text
apps/website/offline-service-worker/
├── Cargo.toml  the package: the pure library, the `offline_service_worker` binary, wasm32-only deps
└── src/        the policy library and the worker binary
```

## How it works

Trunk builds the binary from the app's `index.html` as a `no-modules` worker, next to the app
bundle: `offline_service_worker.js` and `offline_service_worker_bg.wasm` at the site root, with no
content hash in either name. The page registers `/service_worker.js?build=<id>`; that loader,
`apps/website/frontend/service_worker.js`, imports the bindgen script, starts the module and hands
`install`, `activate` and `fetch` to the exported Rust handlers.

```text
page ── register /service_worker.js?build=<id> ──▶ loader (JS, no policy)
                                                    │ importScripts + wasm_bindgen(...)
                                                    ▼
install ─▶ on_install:  shell cache ⟵ "/", "/manifest.webmanifest"; skipWaiting
activate ─▶ on_activate: delete every stale "tbd-offline-*" cache; clients.claim
fetch ───▶ on_fetch:    classify ─▶ cache-first │ network-first │ passthrough
                                       └─ map asset + Range ─▶ 206 slice or 416 from the cached body
```

| Request | Class | Strategy | Cache |
|---|---|---|---|
| navigation | shell document | network-first, stored under `/`, cached copy on unreachable or `5xx` | `tbd-offline-shell-<build>` |
| other same-origin file | shell asset | cache-first | `tbd-offline-shell-<build>` |
| `GET /api/v1/ballistics-catalogs` | catalog list | network-first, cached copy on unreachable or `5xx` | `tbd-offline-catalogs-v1` |
| `GET /api/v1/ballistics-catalogs/{id}/versions/{v}` | catalog version | cache-first | `tbd-offline-catalogs-v1` |
| `/map-assets/**` except satellite XYZ tiles | map asset | cache-first, `Range` sliced | `tbd-offline-map-assets-v1` |
| Google Fonts stylesheet and font files | icon font | cache-first | `tbd-offline-icon-font-v1` |
| any other API route, non-`GET`, other origin, satellite XYZ tile, the worker's own files | passthrough | network only | none |

A network-first request falls back to its cached copy when the network is unreachable or answers
a gateway or server failure (`502`, `503`, `504`, any `5xx`: a proxy such as Caddy answers `502`
while the API behind it is down); a `4xx` is never masked. That copy carries the header
`x-served-from-offline-cache: 1`, which the page reads to word "offline copy from <date>" from the
copy's `Date` header. A cache-first request reads the cache before the network, so a server failure
reaches the page only when nothing is cached.

Only the shell cache carries the build identifier, so a new build replaces the shell and keeps the
roughly 250 MB of map assets. Stored responses keep every header, so the document's
`Cross-Origin-Opener-Policy` and `Cross-Origin-Embedder-Policy` hold offline and the Mission
Creator stays cross-origin isolated. The page downloads the Everon pack itself (the list comes from
`offline_pack::terrain_pack`) into the same caches the worker reads.

## Getting started

Run these from the repository root, in the container with
`CARGO_TARGET_DIR=target-container-api-v2`:

```bash
cargo test -p website-offline-service-worker     # the policy library's unit tests, native
cargo clippy -p website-offline-service-worker --all-targets --target wasm32-unknown-unknown -- -D warnings
cargo xtask mk leptos                            # Trunk builds the worker with the app on :3000
```

The worker runs only in a browser; the native binary is an empty `main`.

## Configuration

None: the crate reads no environment variable or feature flag. The worker takes its build
identifier from the `build` query of its script URL; a missing or malformed value selects the
build `unversioned`. `Cargo.toml` pins edition 2024 and Rust 1.95.

## Public surface

- `cache_names`: `BuildId` (`parse`, `from_script_query`, `script_url`), `CacheNames`
  (`for_build`, `is_stale`, `stale_names`), `CACHE_PREFIX`, `SERVICE_WORKER_SCRIPT_PATH`.
- `request_classification`: `classify`, `cache_key`, `RequestClass`, `CacheStrategy`,
  `InterceptedRequest`, `PassthroughReason`.
- `network_fallback`: `prefers_saved_copy`, `is_gateway_or_server_failure`, `NetworkAnswer`,
  `SAVED_COPY_HEADER`, `saved_on_from_date_header`.
- `range_slicing`: `parse_range_header`, `resolve_range`, `plan_range_response`, `ByteSlice`,
  `RangeResponsePlan`, `unsatisfied_content_range`.
- `offline_pack`: `terrain_pack`, `manifest_url`, `tile_index_url`, `OfflinePack`, `PackEntry`,
  `PackCompleteness`, `IncompleteReason`, and `MapTileIndex`, the shape of
  `/map-assets/<terrain>/tiles/map/index.json`
  (`{"schemaVersion":1,"terrainId":"everon","tiles":[{"z":0,"x":0,"y":0,"bytes":1234}]}`).
- The binary `offline_service_worker`, exporting `on_install`, `on_activate` and `on_fetch` to the
  loader.

## Boundaries

- Depends on: `serde`, `serde_json` and `url`; `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`
  and `web-sys` on `wasm32`. No workspace crate. The unit tests read the committed Everon and
  Arland manifests under `assets/terrains/`.
- Used by: `apps/website/frontend/index.html`, which builds the binary as a Trunk worker, and
  `apps/website/frontend/service_worker.js`, which loads it; the frontend's offline core links the
  library.
- Rules:
  - the crate depends on none of `website-api`, `website-frontend` or `website-graphics-engine`
    (`OFFLINE_SERVICE_WORKER_RULE` in
    `tools/verification_core/src/repository_laws/crate_dependencies.rs`, test
    `the_offline_service_worker_may_link_none_of_the_server_page_or_renderer`);
  - the loader holds no policy: every decision lives in this crate's library and is unit-tested
    natively;
  - a missing or partial map tile index makes the pack incomplete, never complete
    (`a_missing_empty_or_partial_index_is_incomplete_and_never_complete`);
  - a gateway or server failure answers from the cached copy and a `4xx` never does
    (`gateway_and_server_failures_prefer_the_saved_copy`,
    `successes_redirects_and_refusals_are_never_masked`).

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the dependency
  directions between the website crates.
