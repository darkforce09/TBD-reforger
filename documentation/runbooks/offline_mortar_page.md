**Status:** live

# Prepare, use and check the offline mortar calculator

The mortar calculator at `/tools/mortar` stores a service worker pack on its first visit (the app
shell, every published ballistics catalog version, the Everon manifest, elevation, imagery and map
tiles z0–6, about 248 MB) and then solves with no connection. This runbook prepares the map files
the pack lists, runs the offline gate, and checks the pack by hand in a browser. The gate takes a
few minutes, most of it the release build and the first download.

## Prerequisites

- The Everon map assets, gitignored local build output: `assets/terrains/everon/manifest.json`,
  the elevation model, `everon-sat.tbd-sat` and the map tile pyramid under `tiles/map/`. Without
  them the pack stays incomplete and the gate fails closed ("assets missing").
- At least one published ballistics catalog for a manual check
  ([ballistics oracle run](/documentation/runbooks/ballistics_oracle_run.md), step 11); the gate
  instead serves the recorded catalog reads under `contracts/fixtures/api_goldens/`.
- A browser with service workers and Cache Storage on a secure origin: `localhost` counts as one,
  a plain-HTTP LAN address does not.
- About 250 MB of free browser storage for the origin.

## Steps

1. Write the Everon tile index the pack reads its tile list from.

   ```bash
   cargo xtask map tile-index --terrain everon
   ```

   Expected: exit 0 and `assets/terrains/everon/tiles/map/index.json` written (5,461 tiles for
   z0–6); a warning names any zoom level of the manifest's range without a tile, which leaves the
   pack incomplete.

2. Run the offline gate: a release build, then the developer_tools `gate mortar-offline`.

   ```bash
   cargo xtask mk mortar-offline-gate
   ```

   Expected: ten lines `case mortar_offline_<step> ... ok` (`preflight`, `pack_ready`,
   `worker_active`, `listener_stopped`, `navigation_from_service_worker`,
   `cross_origin_isolated`, `catalog_and_map_offline`, `mission_entered`,
   `solution_matches_native`, `tiles_drew`), then `mortar-offline: PASS` and exit 0.

3. For a manual check, bring up the local stack of
   [local development](/documentation/runbooks/local_development.md) and serve the single-page
   app.

   ```bash
   cargo xtask mk leptos
   ```

   Expected: the app answers on http://localhost:3000; `/api` and `/map-assets` are proxied to
   the API on :8080.

4. Open http://localhost:3000/tools/mortar and wait on the page. No command: watch the line under
   the header.

   Expected: "Saving the calculator for offline use… n%", then "Offline copy ready: the
   calculator, its catalogs and the Everon map work without a connection."; the document element
   carries `data-offline-state="ready"` and `data-offline-progress="100"`.

5. Cut the connection (the browser's developer tools set the network offline, or stop the API and
   Trunk), then reload `/tools/mortar`.

   Expected: the page, the catalog and the Everon map load; the catalog line reads "Offline —
   solving with the catalog saved on this device."; Calculate Solution solves. Saving, the event
   list and the saved fire missions need the connection back: the save area reads "Saving fire
   missions and the saved fire missions need a connection to the platform; the calculator above
   keeps working offline." with no sign-in link.

## Verify

In the browser console of the offline page:

```js
document.documentElement.dataset.offlineState + " " + crossOriginIsolated
```

Expected: `ready true`. Cross-origin isolation holds offline because the worker keeps every stored
response header, COOP and COEP included.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| "Offline copy incomplete: the server lists fewer map files than the offline pack needs." | `tiles/map/index.json` is missing (a 404), or lists fewer tiles than the manifest's zoom range | run step 1; the pack is never ready without the index |
| "Offline copy not saved: this browser has too little free storage." | the storage estimate leaves less free space than the files still missing | free space for the origin, or use another browser profile; nothing was fetched |
| "Offline use is not available in this browser." | no service worker, no Cache Storage, or an insecure origin | use `localhost` or HTTPS in a browser with service workers |
| "Offline copy failed: a file could not be downloaded or stored." | a pack file answered an error or the cache refused it | check the listed map assets exist, then reload the page to retry |
| the gate stops at `preflight` with "assets missing" | a map asset, the tile index or a recorded catalog read is absent; every missing path is named | build the named asset, run step 1, or capture the API corpus again |
| after a rebuild the page shows the previous build | the old worker still controls the tab | reload once more: the worker file is served `no-cache`, and activation deletes the stale shell cache while the map assets stay |
| another browser gate answers from a cached build | it did not bypass the worker | the gates set `Network.setBypassServiceWorker`; only `gate mortar-offline` runs with the worker |

## Related

- [Mortar calculator page](/documentation/crates/frontend/pages/field_tools_pages/mortar/mortar_calculator_page.md)
  — the page, its offline behaviour and its save.
- [Offline core](/crates/frontend/foundation/frontend_offline/src/README.md) — the pack download, the
  quota check and the offline state.
- [Offline service worker](/crates/frontend/shell/offline_service_worker/README.md) — the request classes,
  the caches and the Range answers.
- [Mortar offline gate](/tools/browser_testing/browser_gate_suites/src/mortar_offline/README.md) —
  what each gate step checks.
- [Game ballistics design note](/documentation/crates/api/api_server/verification_evidence/game_ballistics.md#offline-design)
  — the offline design and the operator decisions behind it.
