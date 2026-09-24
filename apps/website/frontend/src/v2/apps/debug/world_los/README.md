# World line-of-sight bench host

The browser half of the `/debug/world-los` bench: it loads the committed object catalogue around a
map point, draws the placed objects as plan footprints with each building cut at eye height, and
probes one segment from A to B through the world occluder, reporting the verdict, the hits and
the catalogue coverage. It compiles for `wasm32` only.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/world_los/
├── live/    `mount`: URL run parameters, engine boot, catalogue load, lane upload, probe input
└── live.rs  `Signals`, the URL readers, the catalogue `load`, the plan scene, lane upload, `probe`
```

## How it works

`WorldLosPage`, in the module root `apps/website/frontend/src/v2/apps/debug/world_los.rs`, draws
the canvas and the read-out panel and hands its `Signals` to `live::mount`. The catalogue load in
`live.rs` reads the Everon asset manifest and the prefab table, ingests every object chunk that
covers the radius into a `WorldResidency`, then runs up to 64 `OccluderHost` passes that expand
the descriptors and fetch the BLAS those chunks place; the mean row elevation inside the radius
stands in for the ground. `scene_of` turns the resident rows inside the radius into footprints,
with a proxy flag while an object's geometry is still missing, and cuts up to
`MAX_CUT_BUILDINGS` (96) upright buildings at eye height;
`apps/website/frontend/src/v2/apps/debug/world_los_scene.rs` packs them onto the bench's lanes.
`probe` runs `WorldOccluder::evaluate_los` from A to B and writes the verdict, the first 16 hits
and the coverage, and draws the ray coloured by what it crossed.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/debug/world-los` | `WorldLosPage`, in the module root | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |

The URL flags are `?x=` and `&y=` (the centre in map metres, default 9363, 285), `&r=` (the radius,
default 150 m, clamped to 20 to 600), `&a=x,y,z` and `&b=x,y,z` (the ray ends in the engine frame:
east, up, north), `&eye=` (the cut height above the ground, default 1.8 m) and `&force=webgl`.

## Data

- `GET /map-assets/everon/manifest.json`, the asset manifest, and
  `GET /map-assets/everon/objects/prefabs.json.gz`, the prefab table, both loaded into the
  `WorldResidency`.
- `GET /map-assets/everon/objects/chunks/{cx}_{cy}.json.gz` for every chunk the radius covers.
- The descriptors and BLAS files `OccluderHost` fetches for the resident chunks.
- The `/map-assets` paths are static files the [API](/documentation_v2/glossary.md#api) serves,
  proxied by Trunk in development; the bench calls no `/api/v1` route, reads no context or
  storage, and writes nothing.

## States

| State | What the viewer sees |
|---|---|
| booting | "booting…" on the status line |
| fetching | "fetching <n> chunk(s) around (<x>, <y>) r <r> m…" |
| loading geometry | "loading geometry… pass <n>: <n> descriptors expanded · <n> BLAS" |
| loaded | "<n> chunk(s) ingested · <n> passes · … · ground ≈ <y> m (<n> rows in radius)", then the stats line |
| load failed | "load failed: …", for example "manifest.json unreachable" |
| engine failed | "engine create failed: …" in red |
| probed | the verdict line "A […] → B […] · <m> m · <verdict> · concealment <c> · <ms> ms", the coverage line and the hit list |

## Boundaries

- Depends on: `website_map_engine`: `streaming::loaders` (`fetch_bytes`, `fetch_text`,
  `OccluderHost`), `streaming::scheduler::state::WorldResidency`, `spatial::los::world`
  (`WorldOccluder`, `WorldVerdict`), `world::architecture::section::cutter::section_at`, `frame`
  (`RenderEngine`, `RafPump`) and `overlay::lanes::role_id`; in
  `apps/website/frontend/src/v2/apps/debug/`, the module root's defaults, `world_los_scene.rs`
  (`Footprint`, `build_bench_lanes`, `ray_strip`), `InteriorLanes` from `building_interior.rs` and
  `screen_to_world` from `building_viewer/geom.rs`; `futures`, `js_sys`, `wasm_bindgen` and
  `web_sys`.
- Used by: `apps/website/frontend/src/v2/apps/debug/world_los.rs`, which declares `live` in the
  browser build and calls `live::mount` from `WorldLosPage`, the component of the
  `/debug/world-los` route in `apps/website/frontend/src/app_routes.rs`.
- Rules: the bench reads committed assets only and never touches the
  [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s code; it loads objects
  through the same `OccluderHost` the Mission Creator's line-of-sight tool reads.
  No test covers these browser-only files; the pure scene code they call is tested in
  `apps/website/frontend/src/v2/apps/debug/tests/world_los_scene/`. The route's row in
  `apps/website/frontend/src/router.rs` must match
  `tools_v2/developer-tools/fixtures/dom_oracle/manifests/routes.csv` (`gate s-routes`).
