# Debug benches

URL-only benches that drive one part of the map engine in isolation, with none of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document, persistence or
chrome around it: the building viewer, which loads one extracted building blueprint and probes
line of sight and the viewshed through it, and the world line-of-sight bench, which loads the
committed object catalogue around a map point and probes one segment through it.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/
├── building_interior/     the building viewer's door hit test and probe-ray lane
├── building_interior.rs   `InteriorLanes`: the lane set both benches draw on; compound tessellation
├── building_viewer/       the `/debug/building-viewer` bench: page, pure geometry, browser host
├── building_viewer.rs     the building viewer's module root: shared types and the default prefab
├── mod.rs                 the module tree
├── tests/                 unit tests for the interior lanes and the world bench's scene geometry
├── world_los/             the world bench's browser host: catalogue load, lane upload, the probe
├── world_los.rs           `WorldLosPage`, the `/debug/world-los` route component, and its defaults
└── world_los_scene.rs     the world bench's plan geometry: footprints, section cuts, the probe ray
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
build covers, and a browser half (the `live` modules), which compiles for `wasm32` only.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/debug/building-viewer` | `BuildingViewerPage` | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |
| `/debug/world-los` | `WorldLosPage` | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |

## Public surface

- `building_viewer::BuildingViewerPage` and `world_los::WorldLosPage`: the route components
  `apps/website/frontend/src/app_routes.rs` mounts. Nothing else leaves the folder.

## Boundaries

- Depends on: `website_map_engine` (`world`, `spatial`, `streaming`, `frame`, `overlay::lanes`,
  `camera::ortho` and the viewshed texture of `editing::tools::line_of_sight`), with the `io`
  feature `apps/website/frontend/Cargo.toml` enables for the blueprint raycaster; `gloo_net`,
  `futures`, `js_sys`, `wasm_bindgen` and `web_sys` in the browser build. Nothing from
  `crate::v2::core`.
- Used by: the two routes in `apps/website/frontend/src/app_routes.rs`, with their rows in
  `apps/website/frontend/src/router.rs`; nothing in the navigation links to them.
- Rules: a bench reads committed assets and engine code only; it never writes a
  [mission](/documentation_v2/glossary.md#mission) document, never persists, and imports nothing
  from a page or a sibling workspace. The native `role_id` mirror in `building_interior.rs` must
  equal `apps/website/map-engine/src/overlay/lanes.rs` (`lane_ids_match_the_render_crate` in
  `tests/building_interior.rs`), and no wall may land on a borrowed lane
  (`walls_never_use_borrowed_lanes`). Both route rows must match
  `tools_v2/developer-tools/fixtures/dom_oracle/manifests/routes.csv`, which the route-drift gate
  `gate s-routes` of `tools_v2/developer-tools/` compares.

## Related documentation

- [Debug benches documentation](/documentation_v2/website/frontend/apps/debug/README.md) — the
  index of the benches' feature docs.
- [Building viewer](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) — the
  building bench's purpose and behaviour.
- [World line-of-sight bench](/documentation_v2/website/frontend/apps/debug/world_los_page.md) —
  the world bench's purpose and behaviour.
