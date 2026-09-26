# Building viewer bench

The `/debug/building-viewer` bench: one extracted building blueprint drawn as a floor plan, with a
draggable observer and target whose line of sight is traced through the building's occlusion
mesh, and a per-floor viewshed wash. It exists to check what the
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) extractor produced for a building, and how
the line-of-sight code reads it, without the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) around it.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/building_viewer/
├── geom/     the static lane tessellation of one blueprint, mesh drawing and viewed floor
├── geom.rs   pure geometry: world mapping, camera fit, colours, `StaticLanes`, the wash texture
├── live/     `wire`: the engine boot, the fetches, the upload effects and the pointer listeners
├── live.rs   the browser host: URL readers, lane and wash uploads, the compound loader
├── page.rs   `BuildingViewerPage`: the signals, the pure effects, the header, verdict and legend
└── tests/    unit tests for the world mapping, the camera, the lanes and the wash texture
```

## How it works

The module root, `apps/website/frontend/src/v2/apps/debug/building_viewer.rs`, declares the types
the bench shares (`RayEnd`, `Cam`, `Drag`, `ViewFloor`) and the default prefab.
`BuildingViewerPage` in `page.rs` owns every signal and four pure effects: the line-of-sight
verdict, from the assembled compound building when one loaded and from the blueprint and its
sidecar otherwise; the viewed level's wash while the viewshed is on; the mesh drawing; and each
level's section cuts, which re-run whenever a door swings. In the browser build it calls
`live::wire`, which fetches the assets, boots the `RenderEngine` and uploads the lanes that
`apps/website/frontend/src/v2/apps/debug/building_interior.rs` tessellates from `geom`'s static
lanes. `live.rs` and `live/` compile for `wasm32` only; the rest compiles in the native build too,
where the tests cover `geom`.

The building sits at world `ANCHOR` (6400, 6400) with blueprint +z, game north, up on screen, so
the lane coordinates stay small. Every run reproduces from its URL. Without an occlusion sidecar
the plan still draws from the blueprint, and line of sight, the wash and the mesh drawing stay off.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/debug/building-viewer` | `BuildingViewerPage` | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |

The URL flags are `?prefab=<map-assets path>` (the blueprint to load), `?scene=1` (exterior trees
from `<slug>.scene.json`), `?doors=open` (every door open on load), `?a=x,y,z` and `?b=x,y,z` (the
ray ends in blueprint-local metres) and `?force=webgl` (the WebGL backend the headless capture
uses).

## Data

- `GET` of the blueprint JSON, `/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.json`
  unless `?prefab=` names another, read as `BuildingBlueprint`.
- `GET` of the same path with `.bvh` for `.json`: the occlusion sidecar, read as `BvhSidecar`.
- `GET` of `<slug>.instances.json`, with `<slug>.scene.json` under `?scene=1`, read as
  `InstancesFile`, then every BLAS those instances name, resolved against the prefab folder's
  parent; together they assemble the `CompoundBuilding`.
- The `/map-assets` paths are static files the [API](/documentation_v2/glossary/a_to_f.md#api) serves,
  proxied by Trunk in development; the bench calls no `/api/v1` route, reads no context or
  storage, and writes nothing.

## States

| State | What the viewer sees |
|---|---|
| loading | "loading…" in the header, in place of the building's label |
| blueprint failed | a red line naming the path: "HTTP <status>", "read failed — …" or "parse failed — …" |
| no sidecar | an amber line ending "LOS, viewshed and mesh drawing off (blueprint fallback)"; the plan without a verdict |
| no instances | an amber line ending "shell-only bench (no doors, glass, furniture)" |
| no scene file under `?scene=1` | an amber line ending "scene off"; the building loads without trees |
| furnished | "furnished: <n> instances · doors <open> open · <closed> closed" |
| verdict | "CLEAR" or "BLOCKED" with "concealment <n>%" and the glass, doors, canopy, cover or blocker it crossed |
| viewshed on | "viewshed from A · <level>", or "· no wash on the roof view", with a "✕ clear" button |
| engine failed | "engine create failed: …" in red |

## Boundaries

- Depends on: `website_map_engine`: `world` (the architecture blueprints, the compound building
  and its doors, the section cutter, and the triangulation and strip helpers),
  `spatial::bvh::sidecar` and `spatial::los::interior::wash`,
  `editing::tools::line_of_sight::viewshed_texture`, `frame` (`RenderEngine`, `RafPump`) and
  `overlay::lanes::role_id`; the module root's types and the bench's lane set in
  `apps/website/frontend/src/v2/apps/debug/building_interior.rs`; `gloo_net`, `wasm_bindgen` and
  `web_sys` in the browser build. Nothing from `crate::v2::core`.
- Used by:
  - `apps/website/frontend/src/v2/apps/debug/building_viewer.rs`, the module root, which declares
    these modules and re-exports `BuildingViewerPage` for the `/debug/building-viewer` route in
    `apps/website/frontend/src/app_routes.rs`;
  - `apps/website/frontend/src/v2/apps/debug/building_interior.rs`, which builds on `geom`'s static
    lanes, colours and packing helpers;
  - `apps/website/frontend/src/v2/apps/debug/world_los/live.rs`, for `geom::screen_to_world`;
  - the unit tests in `apps/website/frontend/src/v2/apps/debug/tests/building_interior.rs`.
- Rules: the pure half stays free of browser and engine handles, so the native test build proves
  the geometry (`world_mapping_round_trips_and_points_north_up`,
  `wash_escapes_only_through_the_window` and the rest of `tests/geometry_and_lanes.rs`); lanes are
  addressed by `role_id` constants, never by a copied number; the route's row in
  `apps/website/frontend/src/router.rs` must match
  `tools_v2/developer-tools/fixtures/dom_oracle/manifests/routes.csv`, which the route-drift gate
  `gate s-routes` of `tools_v2/developer-tools/` compares.

## Related documentation

- [Building viewer](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) — the
  bench's purpose and behaviour.
