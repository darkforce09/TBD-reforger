# Map engine

The `map_engine` crate: everything between the platform's map data and the pixels, and the
editing layer through which the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
drives the [mission](/documentation/glossary/g_to_m.md#mission) document. It holds the headless
editing tools and the hosted document, the browser loaders and GPU belts of the static world
streamed from a terrain's assets, the viewshed and symbology uploads, the camera viewport, and the
render engine that hands frame packets to `graphics_engine`. The mission domain (the model, the
compilers, the validator, the document and its authoring commands) is the
[mission crates](/crates/mission/README.md), and the world's data models, the spatial indexes, the
line of sight and the map overlay's lanes and symbology are crates under `crates/`; every consumer
imports those directly. It depends on no UI framework; the frontend supplies the canvas, the
preference readers and the page around them.

## Contents

```text
legacy/map_engine/
├── Cargo.toml  the `map_engine` package: its feature tiers, dependencies and wasm32 crates
├── src/        the library `map_engine`: nine modules, each but `camera` behind its feature
└── tests/      integration suites: headless document operations over the mission crates
```

## How it works

The dependency arrow runs one way: the frontend uses this crate, and this crate uses
`graphics_engine`, the pure renderer, which never names a map concept. The crate speaks
the renderer's frame vocabulary through `src/frame/mod.rs` alone: it builds its pipelines and
atlases with the renderer's constructors and hands it draw batches and frame packets.
`RenderEngine`, which owns the device, the surface and every lane, lives here in `src/frame/`.

A consumer takes only the tier it needs, because each module compiles under the feature it
belongs to (the table in the [source README](/legacy/map_engine/src/README.md)):

```text
render ──▶ streaming ──▶ world ──▶ graphics_engine, render_primitives, terrain_elevation,
   │           │                    terrain_relief, map_draw_lanes, label_layout, unit_symbology,
   │           │                    overlay_instances, road_network, vegetation, place_names,
   │           │                    spatial_indexes, terrain_line_of_sight
   │           └──────▶ serde, serde_json, rkyv, world_file_formats, flate2, prefab_catalog,
   │                    world_chunks, satellite_imagery, water_bodies, world_store,
   │                    interior_line_of_sight, world_line_of_sight
   └──────────▶ graphics_engine, browser_platform
editing ──▶ streaming ──▶ world
   └──────────▶ serde_json, mission_payload, mission_validation, formation_geometry, mission_crdt,
                mission_document, mission_operations
```

No tier is on by default. `world` is the static world, the viewshed upload, the overlay and the
frame vocabulary, and links the renderer and the terrain, world-object, overlay and spatial crates
those modules draw; `streaming` adds the loader and scheduler stack with the on-disk formats
(`world_file_formats`, rkyv, flate2), the served-data descriptors (`serde`, `serde_json`) and the
world format and line of sight crates it reads; `render` adds the GPU frame path, the diagnostics
and the doll. `editing` adds the editing layer and the six mission crates it hosts and drives, and
joins the document to the streamed world, so it takes `streaming` too. Browser code (canvas,
fetch, image decoding, timers, the console) compiles only for wasm32; native builds keep the
geometry, codecs and state machines and test them without a browser.

## Getting started

Run these from the repository root:

```bash
cargo xtask ci lfs-dem           # pull the Everon elevation model from Git LFS; one test decodes it
cargo xtask mk wasm-ci           # fmt, clippy (all features, wasm32) and tests of both engines
cargo xtask verify engine-layers # the layer rules between the engine crates and inside this one
```

`cargo test -p map_engine --all-features` runs this crate's tests alone. Without
`--all-features` the tripwire test `map_engine_tests_require_all_features` fails, since a build
without every tier compiles only a fraction of the crate. The tests read the Everon terrain under
`assets/terrains/everon/` from disk. The crate has no binary of its own: the Mission Creator
runs it in the browser, inside the single-page app that `cargo xtask mk leptos` builds and serves,
and `cargo xtask mk leptos-gates` runs the editor gate, whose `selfcheck` smoke calls the render
engine's readback checks.

## Configuration

Cargo features, in `Cargo.toml`:

| Feature | Turns on | Taken by |
|---|---|---|
| `world` | `graphics_engine`, `render_primitives`, `terrain_elevation`, `terrain_relief`, `map_draw_lanes`, `label_layout`, `unit_symbology`, `overlay_instances`, `road_network`, `vegetation`, `place_names`, `spatial_indexes`, `terrain_line_of_sight`; `world`, `spatial`, `overlay`, `frame` | the frontend, `tools/developer_tools` |
| `streaming` | `world`, `serde`, `serde_json` with float round trips, rkyv, `world_file_formats`, `flate2`, `prefab_catalog`, `world_chunks`, `satellite_imagery`, `water_bodies`, `world_store`, `interior_line_of_sight`, `world_line_of_sight`; the `streaming` module: loaders, scheduler, buffers, memory ledger, bridge | the frontend on wasm32 and in its tests, `tools/developer_tools` |
| `render` | `streaming`, `graphics_engine`, `browser_platform`; the GPU frame path, `diagnostics`, `doll` | the frontend on wasm32 |
| `editing` | `world`, `streaming`, `serde_json`, `mission_payload`, `mission_validation`, `formation_geometry`, `mission_crdt`, `mission_document`, `mission_operations`; `editing` | the frontend |

In the browser the crate also reads the page's query string: `memBudgetMb` sets the streaming
memory budget in MiB (or `window.__memBudgetMb`; default 1536), `sat=preview` loads the satellite
imagery by range requests alone and never fetches the whole bundle, and `t9382=1` (or
`window.__t9382Log`) logs chunk allocation.

## Public surface

- The library `map_engine`: `editing`, `streaming`, `frame`, `camera` and `doll` for the
  frontend; `frame` for the offline tools.
- The JavaScript-facing methods of `RenderEngine` and `DollEngine` (`#[wasm_bindgen]`), which the
  frontend calls from Rust and the editor gate reaches through the globals the Mission Creator
  publishes (`window.__selfChecks`, `window.__editorBench`, `window.__arsenalDoll`).

## Boundaries

- Depends on: `graphics_engine` and `render_primitives` (optional, from the `world` tier up);
  `world_file_formats`, `serde`, `serde_json`, `rkyv` and `flate2` (from the `streaming` tier
  up); `bytemuck`; from the `editing` tier, the mission crates `mission_payload`,
  `mission_validation`, `formation_geometry`, `mission_crdt`, `mission_document` and
  `mission_operations`; from the `world`
  tier up, the terrain crates `terrain_elevation`, `terrain_relief` and `road_network`, the
  world-object crates `vegetation` and `place_names`, the map overlay crates `map_draw_lanes`,
  `label_layout`, `unit_symbology` and `overlay_instances`, `spatial_indexes` and
  `terrain_line_of_sight`; from the `streaming` tier up, `prefab_catalog`, `world_chunks`,
  `world_store`, `satellite_imagery`, `water_bodies`, `interior_line_of_sight` and
  `world_line_of_sight`; on
  wasm32, `wgpu`, `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys`, `futures`,
  `console_error_panic_hook` and `browser_platform` (from the `render` tier up); and at run time
  the terrain assets of `assets/terrains/`,
  which the API serves under `/map-assets`.
  The geometry crates: `map_coordinates` and `camera_math` on every tier.
  Its tests also use the `test_fixtures` of `spatial_indexes`, `prefab_catalog` and
  `world_chunks`.
- Used by:
  - the frontend (`apps/frontend/Cargo.toml`): `world` and `editing` on every target,
    `render` and `streaming` on wasm32, and `streaming` for its native tests;
  - `tools/developer_tools/Cargo.toml`: `world` and `streaming`, for the render engine's frame
    vocabulary;
  - `tools/xtask/`: the `wasm-ci` lane and the `engine-layers` gate.
- Rules:
  - the arrow is one-way: `legacy/graphics_engine/` never imports this crate (rule 1 of
    `cargo xtask verify engine-layers`) and the frontend never imports the graphics engine (rule
    6); the rules inside the crate are listed in the source README;
  - the API links no tier of this crate (the `engineering_laws_api_depends_on_no_graphics_or_frontend_crate`
    engineering law); the mission crates it links reach no world or graphics crate (the crate-tier
    law's category matrix);
  - the tests run with `--all-features` (`map_engine_tests_require_all_features`); the camera's
    deck.gl parity suite runs in `crates/geometry/camera_math/`.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — the terrain assets: what
  Git LFS holds, how `/map-assets` is served, and the LFS pulls.
- [Editor gates](/documentation/runbooks/editor_gates.md) — running the editor gate, whose
  `selfcheck` smoke calls the render engine's readback checks.
- [Map engine documentation](/documentation/legacy/map_engine/README.md) — the crate's
  overview, map streaming, the editing layer and draft persistence.
- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the layer rules
  `cargo xtask verify engine-layers` enforces, and why.
