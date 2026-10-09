# Debug benches

The `debug_benches` crate: the single-page app's URL-only benches that drive one part of the map
engine in isolation, with none of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s document, persistence or
chrome around it. The building viewer loads one extracted building blueprint and probes line of
sight and the viewshed through it; the world line-of-sight bench loads the committed object
catalogue around a map point and probes one segment through it. Beside them, the equipment data
viewer reads the imported Workbench equipment and vehicle datasets from the API, and the
ballistics agreement bench solves seeded fire problems with the solver's WebAssembly build for the
native agreement gate.

## Contents

```text
crates/frontend/workspaces/debug_benches/
├── Cargo.toml  the package: the world, line-of-sight and ballistics crates, `frontend_api_dtos`, `leptos`; the renderer, `frontend_transport` and browser crates for wasm32; layout tier 9
└── src/        the four benches, the interior lanes and scene geometry they share, their tests
```

## How it works

Each bench is a route component that mounts its own canvas, creates its own `RenderEngine` and
drives the map engine directly. It shares none of the Mission Creator's boot machinery: it reads
no IndexedDB draft, hydrates nothing and draws no terrain or satellite layer, and the world bench
fetches its object chunks itself. Every parameter of a run is in the URL, so a reading reproduces
exactly.

| Bench | Route | Loads | Probes |
|---|---|---|---|
| building viewer | `/debug/building-viewer` | one blueprint JSON, its `.bvh` occlusion sidecar, its instances | line of sight through the building, and a per-floor viewshed wash |
| world line of sight | `/debug/world-los` | the object manifest, prefab table and chunks around a map point, then their descriptors and BLAS | one A to B segment through the world occluder, with coverage |
| equipment data viewer | `/debug/data-viewer` | the imported equipment and vehicle datasets, read anonymously from the API | the sources, fields and relationships of each resource |
| ballistics agreement | `/debug/ballistics-agreement` | the published ballistics catalog | seeded fire problems solved by the solver's WebAssembly build |

Each bench splits into a pure half, which compiles on every target and which the native tests
cover, and a browser half (the `live` modules, the data viewer's panels and the route
components), which compiles for `wasm32` only. The [source tree README](src/README.md) walks
through each file.

## Getting started

Run from the repository root:

```bash
cargo test -p debug_benches   # the interior lanes, the scene geometry, the agreement reading, the data viewer's URL state
```

## Configuration

None: no feature, no environment variable. Every parameter of a run is a URL query parameter,
listed in each bench's feature doc.

## Public surface

The routes the app mounts, each at route tier `none` (no sign-in needed), full-bleed and
chromeless, with no navigation entry:

| Route | Component |
|---|---|
| `/debug/building-viewer` | `building_viewer::BuildingViewerPage` |
| `/debug/world-los` | `world_los::WorldLosPage` |
| `/debug/data-viewer` | `data_viewer::DataViewerPage` |
| `/debug/ballistics-agreement` | `ballistics_agreement::BallisticsAgreementPage` |

- The four route components above, compiled for `wasm32` only.
- The pure halves, public so the tests and the browser half share them: `building_interior`
  (`InteriorLanes`, `LevelCuts`), `building_viewer::geom`, `world_los_scene`,
  `ballistics_agreement::{agreement_report, bench_query}` and
  `data_viewer::{navigation_state, browsing_state}`.
- The data viewer's panels in the browser build: `data_viewer::{page, field_inventory, layout,
  overview, relationships, resources, resource_data, source_inspector}` and the components they
  share across modules (`Pager`, `Search`, `Feedback`, `Panel`, `VirtualList` with `LinkRow`,
  `DocumentInspector`, `ValueBrowser`, `CardGrid`, `ComponentCard`, `FieldRow`, `BatchLoader`,
  `FocusCard`, `InlineDetails`, `ValueDetails` and each tab's component).
- `error`: `Error` and `Result`, re-exported at the crate root.
- `prelude`: the four route components and `InteriorLanes`.

## Boundaries

- Depends on: `camera_math`, `map_coordinates` (the world `ANCHOR`), `geometry_primitives`,
  `render_primitives` and `road_network`; the world crates `building_interiors`,
  `interior_line_of_sight`, `spatial_indexes`, `terrain_line_of_sight` and `map_draw_lanes`; the
  viewshed texture of `map_editing_tools`; the ballistics crates `ballistics_model`,
  `ballistics_agreement_cases` and `fire_mission_planning`; `frontend_api_dtos`, `time_source`, `leptos`,
  `serde`, `serde_json` and `url`; in the browser build `frontend_transport` (the anonymous reads),
  `map_renderer`, `gpu_frame`, `map_asset_loading`, `world_line_of_sight`, `chunk_draw_buffers`,
  `browser_platform`, `leptos_router`, `futures`, `gloo-net`, `gloo-timers`, `js-sys`,
  `wasm-bindgen` and `web-sys`; `frontend_test_support` for its tests
  only.
- Used by: the four routes in `crates/frontend/shell/frontend_application/src/app_routes.rs`, with their rows in
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`; nothing in the navigation
  links to them.
- Rules:
  - a map bench reads committed assets and engine code only, and the data viewer only reads the
    API anonymously; no bench writes a [mission](/documentation/glossary/g_to_m.md#mission)
    document or persists anything;
  - the crate depends on foundation crates only, never on a page, a feature or a Mission Creator
    crate, and no Mission Creator crate depends on it (`cargo xtask ci verify-workspace-laws`);
  - the lane ids the benches draw on are the render crate's own `role_id` constants, and no
    wall lands on a borrowed lane;
  - the ballistics agreement bench's reading and case mapping are mirrored by the gate
    `gate ballistics-agreement` in
    `tools/browser_testing/browser_gate_suites/src/ballistics_agreement/`.

## Related documentation

- [Source tree](src/README.md) — each file of the benches.
- [Debug benches documentation](/documentation/crates/frontend/workspaces/debug_benches/README.md)
  — the index of the benches' feature docs.
- [Building viewer](/documentation/crates/frontend/workspaces/debug_benches/building_viewer_page.md)
  — the building bench's purpose and behaviour.
- [World line-of-sight bench](/documentation/crates/frontend/workspaces/debug_benches/world_los_page.md)
  — the world bench's purpose and behaviour.
- [Ballistics agreement bench](/documentation/crates/frontend/workspaces/debug_benches/ballistics_agreement_page.md)
  — the agreement bench's purpose and behaviour.
