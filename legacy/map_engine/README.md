# Map engine

The `map_engine` crate: everything between the platform's map data and the pixels, and
the [mission](/documentation/glossary/g_to_m.md#mission) domain the
[API](/documentation/glossary/a_to_f.md#api) and the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) share. It holds the mission
compiler and CRDT document, the headless editing tools, the browser loaders and GPU belts of the
static world streamed from a terrain's assets, the viewshed and symbology uploads, the camera
viewport, and the render engine that hands frame packets to `graphics_engine`. The world's data
models, the spatial indexes, the line of sight and the map overlay's lanes and symbology are crates
under `crates/` that every consumer imports directly. It depends on no UI
framework; the frontend supplies the canvas, the preference readers and the page around them.

## Contents

```text
legacy/map_engine/
├── Cargo.toml  the `map_engine` package: its feature tiers, dependencies and wasm32 crates
├── src/        the library `map_engine`: ten modules, each behind its feature
└── tests/      integration suites: headless document operations
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
   │           └──────▶ scenario, rkyv, world_file_formats, flate2, prefab_catalog, world_chunks,
   │                    satellite_imagery, water_bodies, world_store, interior_line_of_sight,
   │                    world_line_of_sight
   └──────────▶ graphics_engine, browser_platform
editing ──▶ store ──▶ scenario ──▶ serde, serde_json, thiserror, libm   (store adds yrs)
editing ──▶ world, streaming
```

`scenario`, the default and the tier the API links, is the headless mission compiler and
validator: it pulls no graphics crate, PNG decoder, rkyv or flate2. `store` adds the Yjs document
(`yrs`). `world` adds the static world, the viewshed upload, the overlay and the frame
vocabulary, links the renderer and the terrain, world-object, overlay and spatial crates those
modules draw; `streaming` adds the loader and scheduler stack with the on-disk formats
(`world_file_formats`, rkyv, flate2) and the world format and line of sight crates it reads;
`render` adds the GPU frame path, the diagnostics and the doll. `editing` joins the document to the streamed world, so it takes
`streaming` too. Browser code (canvas, fetch, image decoding,
timers, the console) compiles only for wasm32; native builds keep the geometry, codecs and state
machines and test them without a browser.

## Getting started

Run these from the repository root:

```bash
cargo xtask ci lfs-dem           # pull the Everon elevation model from Git LFS; one test decodes it
cargo xtask mk wasm-ci           # fmt, clippy (all features, wasm32) and tests of both engines
cargo xtask verify engine-layers # the layer rules between the engine crates and inside this one
```

`cargo test -p map_engine --all-features` runs this crate's tests alone. Without
`--all-features` the tripwire test `map_engine_tests_require_all_features` fails, since the
default tier compiles only a fraction of the crate. The tests read the Everon terrain under
`assets/terrains/everon/` from disk. The crate has no binary of its own: the Mission Creator
runs it in the browser, inside the single-page app that `cargo xtask mk leptos` builds and serves,
and `cargo xtask mk leptos-gates` runs the editor gate, whose `selfcheck` smoke calls the render
engine's readback checks.

## Configuration

Cargo features, in `Cargo.toml`:

| Feature | Turns on | Taken by |
|---|---|---|
| `scenario` (default) | `serde`, `serde_json`, `thiserror`, `libm`; `data::scenario` | the API |
| `store` | `scenario`, `yrs`; `data::store` | the frontend |
| `world` | `graphics_engine`, `render_primitives`, `terrain_elevation`, `terrain_relief`, `map_draw_lanes`, `label_layout`, `unit_symbology`, `overlay_instances`, `road_network`, `vegetation`, `place_names`, `spatial_indexes`, `terrain_line_of_sight`; `world`, `spatial`, `overlay`, `frame` | the frontend, `tools/developer_tools` |
| `streaming` | `world`, `scenario`, float round trips in `serde_json`, rkyv, `world_file_formats`, `flate2`, `prefab_catalog`, `world_chunks`, `satellite_imagery`, `water_bodies`, `world_store`, `interior_line_of_sight`, `world_line_of_sight`; the `streaming` module: loaders, scheduler, buffers, memory ledger, bridge | the frontend on wasm32 and in its tests, `tools/developer_tools` |
| `render` | `streaming`, `graphics_engine`, `browser_platform`; the GPU frame path, `diagnostics`, `doll` | the frontend on wasm32 |
| `editing` | `store`, `world`, `streaming`; `editing` | the frontend |

In the browser the crate also reads the page's query string: `memBudgetMb` sets the streaming
memory budget in MiB (or `window.__memBudgetMb`; default 1536), `sat=preview` loads the satellite
imagery by range requests alone and never fetches the whole bundle, and `t9382=1` (or
`window.__t9382Log`) logs chunk allocation. At build time `data::scenario` embeds
`contracts/rules/kit-aliases.json`.

## Public surface

- The library `map_engine`: `data::scenario` for the API's missions and operations
  domains; `data`, `editing`, `streaming`, `frame`, `camera` and `doll` for the frontend;
  `data::scenario` and `frame` for the offline tools.
- The JavaScript-facing methods of `RenderEngine` and `DollEngine` (`#[wasm_bindgen]`), which the
  frontend calls from Rust and the editor gate reaches through the globals the Mission Creator
  publishes (`window.__selfChecks`, `window.__editorBench`, `window.__arsenalDoll`).

## Boundaries

- Depends on: `graphics_engine` and `render_primitives` (optional, from the `world` tier up);
  `world_file_formats` (from the `streaming` tier up); `serde`,
  `serde_json`, `thiserror`, `libm`, `yrs`, `rkyv`, `flate2` and `bytemuck`; from the `world`
  tier up, the terrain crates `terrain_elevation`, `terrain_relief` and `road_network`, the
  world-object crates `vegetation` and `place_names`, the map overlay crates `map_draw_lanes`,
  `label_layout`, `unit_symbology` and `overlay_instances`, `spatial_indexes` and
  `terrain_line_of_sight`; from the `streaming` tier up, `prefab_catalog`, `world_chunks`,
  `world_store`, `satellite_imagery`, `water_bodies`, `interior_line_of_sight` and
  `world_line_of_sight`; on
  wasm32, `wgpu`, `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys`, `futures`,
  `console_error_panic_hook` and `browser_platform` (from the `render` tier up);
  `contracts/rules/kit-aliases.json`; and at run time the terrain assets of `assets/terrains/`,
  which the API serves under `/map-assets`.
  The geometry crates: `map_coordinates` and `camera_math` on every tier.
  Its tests also use `jsonschema` (dev-dependency), which validates the ballistics catalog sample
  against `contracts/definitions/ballistics-catalog.schema.json`, and the `test_fixtures` of
  `spatial_indexes`, `prefab_catalog` and `world_chunks`.
- Used by:
  - the API (`apps/api/Cargo.toml`), at the default `scenario` tier;
  - the frontend (`apps/frontend/Cargo.toml`): `world`, `store` and `editing` on every target,
    `render` and `streaming` on wasm32, and `streaming` for its native tests;
  - `tools/developer_tools/Cargo.toml`: `world`, `streaming` and `scenario`, for the mission
    compiler and the render engine's frame vocabulary;
  - `tools/xtask/`: the `wasm-ci` lane and the `engine-layers` gate.
- Rules:
  - the arrow is one-way: `legacy/graphics_engine/` never imports this crate (rule 1 of
    `cargo xtask verify engine-layers`) and the frontend never imports the graphics engine (rule
    6); the rules inside the crate are listed in the source README;
  - the API's tier stays thin: `data/scenario/` names no module of a higher tier and no graphics
    crate (rule 4; the source README gives the gate's exact list);
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
