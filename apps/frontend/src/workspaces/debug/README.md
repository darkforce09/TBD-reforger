# Debug benches

URL-only benches that drive one part of the map engine in isolation, with none of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s document, persistence or
chrome around it: the building viewer, which loads one extracted building blueprint and probes
line of sight and the viewshed through it, and the world line-of-sight bench, which loads the
committed object catalogue around a map point and probes one segment through it. Beside them, the
equipment data viewer reads the imported Workbench equipment and vehicle datasets from the API,
and the ballistics agreement bench solves seeded fire problems with the solver's WebAssembly
build for the native agreement gate.

## Contents

```text
apps/frontend/src/workspaces/debug/
├── ballistics_agreement/    the `/debug/ballistics-agreement` bench: reading, URL parameters, browser host
├── ballistics_agreement.rs  `BallisticsAgreementPage`, the route component, its defaults and states
├── building_interior/       the building viewer's door hit test and probe-ray lane
├── building_interior.rs     `InteriorLanes`: the lane set both benches draw on; compound tessellation
├── building_viewer/         the `/debug/building-viewer` bench: page, pure geometry, browser host
├── building_viewer.rs       the building viewer's module root: shared types and the default prefab
├── data_viewer/             the `/debug/data-viewer` equipment and vehicle dataset reader
├── mod.rs                   the module tree
├── tests/                   unit tests for the interior lanes and the world bench's scene geometry
├── world_los/               the world bench's browser host: catalogue load, lane upload, the probe
├── world_los.rs             `WorldLosPage`, the `/debug/world-los` route component, and its defaults
└── world_los_scene.rs       the world bench's plan geometry: footprints, section cuts, the probe ray
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

Both benches draw on the interior lanes of `building_interior.rs`, the `INTERIOR_*` and `SCENE_*`
lanes of the map engine's lane table, and never borrow a terrain lane. Each splits into a pure half
(`building_interior.rs`, the building viewer's `geom`, `world_los_scene.rs`), which the native test
build covers, and a browser half (the `live` modules and the route component), which compiles for
`wasm32` only. The route components (the data viewer's too) exist in the browser build only,
because the app's mount chain that names them, `apps/frontend/src/app_routes.rs`, is browser-only.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/debug/building-viewer` | `BuildingViewerPage` | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |
| `/debug/world-los` | `WorldLosPage` | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |
| `/debug/data-viewer` | `DataViewerPage` | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |
| `/debug/ballistics-agreement` | `BallisticsAgreementPage` | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |

## Public surface

- `building_viewer::BuildingViewerPage`, `world_los::WorldLosPage`,
  `data_viewer::DataViewerPage` and `ballistics_agreement::BallisticsAgreementPage`: the route
  components `apps/frontend/src/app_routes.rs`
  mounts. Nothing else leaves the folder.

## Boundaries

- Depends on: `camera_math` (`ortho`), `geometry_primitives` (`Rigid`), `render_primitives`
  (triangulation) and `browser_platform` (`fetch`); the world crates `building_interiors`,
  `interior_line_of_sight`, `world_line_of_sight`, `spatial_indexes`, `terrain_line_of_sight`,
  `road_network` and `map_draw_lanes`; `map_engine` (`streaming` and `frame`); the viewshed
  texture of `map_editing_tools::line_of_sight`; `gloo_net`,
  `futures`, `js_sys`, `wasm_bindgen` and `web_sys` in the browser build. The data viewer depends
  on `crate::foundation::transport` (its anonymous reads and the equipment data viewer DTOs), and the
  ballistics agreement bench on its anonymous reads, the ballistics catalog DTOs and the
  ballistics crates (`ballistics_agreement_cases`, `fire_mission_planning`, `ballistics_model`);
  nothing else here uses `crate::foundation`.
- Used by: the four routes in `apps/frontend/src/app_routes.rs`, with their rows in
  `apps/frontend/src/foundation/route_table/mod.rs`; nothing in the navigation links to them.
- Rules: a map bench reads committed assets and engine code only, and the data viewer only reads
  the API anonymously; no bench writes a
  [mission](/documentation/glossary/g_to_m.md#mission) document or persists anything, and none
  imports from a page or a sibling workspace. The native `role_id` mirror in `building_interior.rs` must
  equal `crates/map_overlay/map_draw_lanes/src/lane_roles.rs` (`lane_ids_match_the_render_crate` in
  `tests/building_interior.rs`), and no wall may land on a borrowed lane
  (`walls_never_use_borrowed_lanes`). The ballistics agreement bench's reading and case mapping
  are mirrored by the gate `gate ballistics-agreement` in
  `tools/browser_testing/browser_gate_suites/src/ballistics_agreement/`. The four route rows must match
  `tools/browser_testing/browser_gate_suites/fixtures/dom_oracle/manifests/routes.csv`, which the route-drift gate
  `gate s-routes` of `tools/developer_tools/` compares.

## Related documentation

- [Debug benches documentation](/documentation/apps/frontend/workspaces/debug/README.md) — the
  index of the benches' feature docs.
- [Building viewer](/documentation/apps/frontend/workspaces/debug/building_viewer_page.md) — the
  building bench's purpose and behaviour.
- [World line-of-sight bench](/documentation/apps/frontend/workspaces/debug/world_los_page.md) —
  the world bench's purpose and behaviour.
- [Ballistics agreement bench](/documentation/apps/frontend/workspaces/debug/ballistics_agreement_page.md) —
  the agreement bench's purpose and behaviour.
