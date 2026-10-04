# Debug benches source

The source tree of `debug_benches`: the four URL-only benches, the interior lanes and scene
geometry the two map benches share, and their tests. The [crate README](../README.md) lists the
routes, the public surface and the boundaries.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/
├── ballistics_agreement/    the `/debug/ballistics-agreement` bench: reading, URL parameters, browser host
├── ballistics_agreement.rs  `BallisticsAgreementPage`, the route component, its defaults and states
├── building_interior/       the building viewer's door hit test and probe-ray lane
├── building_interior.rs     `InteriorLanes`: the lane set both benches draw on; compound tessellation
├── building_viewer/         the `/debug/building-viewer` bench: page, pure geometry, browser host
├── building_viewer.rs       the building viewer's module root: shared types and the default prefab
├── data_viewer/             the `/debug/data-viewer` equipment and vehicle dataset reader
├── error.rs                 `Error` and `Result`: why the agreement bench cannot start a run
├── lib.rs                   the module tree
├── prelude.rs               the four route components and `InteriorLanes`
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
The data viewer's panel modules and the components they share across modules are public in the
browser build, since leptos's component builder always has a `pub` `build` method.

## Boundaries

- Depends on: the crates the [crate README](../README.md) lists; the browser half alone uses
  `frontend_transport`, the renderer and the browser crates.
- Used by: the crate's callers through `lib.rs`; `app_routes.rs` mounts the four route components.
- Rules: the pure halves (`building_interior.rs`, `building_viewer/geom`, `world_los_scene.rs`,
  the agreement bench's `agreement_report.rs` and `bench_query.rs`, the data viewer's
  `navigation_state.rs` and `browsing_state.rs`) name no browser type, so they compile on every target; a bench imports no page and no other
  workspace; the lane ids come from `map_draw_lanes::lane_roles::role_id` on every target
  (`lane_ids_match_the_render_crate` in `tests/building_interior.rs`), and no wall lands on a
  borrowed lane (`walls_never_use_borrowed_lanes`).
