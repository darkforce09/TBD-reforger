# Mortar calculator offline gate

`gate mortar-offline`: proves that the mortar calculator (`/tools/mortar`) works with the API down
behind its proxy and with the server gone. The first visit stores the offline pack; the gate then
makes every `/api/` request answer `502`, reloads, and holds the page to the saved copy and the
native solution; then it stops its server, reloads, and holds the page to the native solution of a
fire mission it types and places on the map.

## Contents

```text
tools/developer_tools/src/browser_testing/mortar_offline/
├── api_down_visit.rs     the visit with every `/api/` request answering 502: worker cache, dated offline copy, kept pack, solution
├── expected_solution.rs  the native solve, worded by `fire_mission_planning`'s shared `solution_wording`
├── map_pixels.rs         whether the map rectangle of a screenshot shows drawn imagery
├── mission_entry.rs      the typed mission (shell, charge, map click, gun, heights, wind) and its native comparison
├── mission_plan.rs       the required files, the recorded catalog reads, the shell and the gun position
├── mod.rs                `run`: `STEPS` in order; `StepProgress` prints each step's `ok` or `FAILED` line
└── page_driver.rs        the page scripts: pack state, fields, map click, Calculate, the tables
```

## How it works

```text
preflight   dist (index.html, service_worker.js), Everon manifest, DEM, tbd-sat, tiles/map/index.json,
            and the recorded reads named from the committed catalog's catalog_id and version
serve       dist + /map-assets + /api from contracts/fixtures/api_goldens (server/)
online      /tools/mortar ─▶ data-offline-state on <html> = ready (incomplete, quota-short,
            unsupported or failed end the run), with data-offline-optional = complete or
            missing (the cross-origin icon font; printed, absent fails) ─▶ an active service worker
api down    set_api_down: the server answers 502 for /api/ (a proxy whose API is stopped) ─▶
            navigate again ─▶ fetch('/api/v1/ballistics-catalogs') from the page is 200 with
            x-served-from-offline-cache: 1 (the worker's cache) ─▶ the catalog loads and the page
            says "Offline copy from <date>" ─▶ the re-run pack ends ready with
            data-offline-refresh = kept-saved-copy and "Refresh failed, using the saved copy from"
            ─▶ the mission below solves to the native solution ─▶ the API is marked up again
offline     close the server (a request to it must fail) ─▶ navigate again
            ─▶ the document response comes from the service worker, crossOriginIsolated is true
            ─▶ the catalog loads and the map is ready
mission     the weapon's high-explosive shell, recommended charge; a map click places the target;
            the gun 1 200 m south as a 10-figure grid; manual heights 42.0 and 18.5 m; wind 3.5 m/s
            from 250°; Calculate
compare     the battery table and every charge table, cell for cell with the laid charge, against
            solve_fire_mission on the served catalog, worded by the same solution_wording functions
            the page renders
map         an Everon map asset answered by the service worker, and at least 256 colours with a
            luma deviation of 4 or more inside the map canvas of a screenshot
```

Each step prints `case mortar_offline_<step> ... ok`: `preflight`, `pack_ready`, `worker_active`,
`api_down_behind_proxy`, `catalog_list_from_worker_cache`, `catalog_from_saved_copy`,
`pack_kept_ready`, `api_down_solution_matches_native`, `listener_stopped`, `navigation_from_service_worker`, `cross_origin_isolated`,
`catalog_and_map_offline`, `mission_entered`, `solution_matches_native`, `tiles_drew`. A run where
every step passes prints `mortar-offline: PASS` and exits 0; the first failure prints
`case mortar_offline_<step> ... FAILED: <cause>` for the step that failed, then `mortar-offline: FAIL`,
and exits 1. A missing file fails the preflight with every missing path
named ("assets missing"): the map tile index comes from `cargo xtask map tile-index --terrain
everon`, the recorded catalog reads from the API corpus capture. The crest line is not compared,
because the page samples it from the elevation model while the gate types the heights.

The page is opened without `Page::bypass_service_worker`, unlike every other gate, and with the
network domain enabled so each response says whether the service worker answered it.

## Boundaries

- Depends on: `crate::browser_testing::cdp` (launch, pages, input, screenshots),
  `crate::browser_testing::server` (the server and its `api_fixture_corpus`),
  `crate::repository_layout::MapAssetMounts`; `map_engine` (`camera::grid_reference`);
  `ballistics_model` (`catalog`) and `fire_mission_planning` (`fire_mission`, `battery`,
  `solution_wording`); `image` for the
  screenshot; the committed catalog in `contracts/catalogs/ballistics/`.
- Used by: `gate mortar-offline` in `tools/developer_tools/src/browser_testing/cli.rs`, run by
  `cargo xtask mk mortar-offline-gate` after `trunk build --release`.
- Rules: the page's DOM hooks it reads are `data-offline-state`, `data-offline-optional`,
  `data-offline-refresh`, `data-mortar-catalog`, `data-mortar-offline`,
  `data-mortar-input`, `data-mortar-position`, `data-mortar-map-state`, `data-mortar-solution`,
  `data-mortar-problems`, `data-mortar-battery` and `data-mortar-gun`; the page and `expected_solution.rs` both word a
  solution with `crates/ballistics/fire_mission_planning/src/solution_wording.rs`, so the
  comparison is exact at every shown digit and never a tolerance.
- The gate runs against the `dist` it is given: run it through `cargo xtask mk mortar-offline-gate`,
  which builds the release app first; a `dist` built before a solver change fails
  `solution_matches_native`.

## Related documentation

- [Mortar calculator page](/apps/frontend/src/pages/field_tools/mortar/README.md) — the
  page, its inputs and its solution panel.
- [Offline core](/apps/frontend/src/foundation/offline/README.md) — the pack download and the
  `data-offline-state` values.
- [Offline service worker](/apps/offline_service_worker/README.md) — the worker the reload
  is answered by.
