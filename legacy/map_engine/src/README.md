# Map engine source

The source of the `map_engine` library: nine top-level modules, each but `camera` compiled only
under the crate feature it needs, and the crate's own unit tests. It holds no UI framework: browser code
compiles only for wasm32, most of it only with the `render` feature as well, and everything else
builds and tests natively.

## Contents

```text
legacy/map_engine/src/
├── camera/       the render engine's viewport: resize, pan and zoom, world and screen answers
├── diagnostics/  readback checks, the benchmark and statistics, and the GPU and frame clocks
├── doll/         the arsenal's 3D mannequin preview: scene, picking and its own renderer
├── editing/      the live mission document, its undo drive and the headless map tools
├── frame/        the render engine, its batch list and upload belts, the frame vocabulary
├── lib.rs        the crate root: each module behind its feature
├── overlay/      the lane preferences and the symbology's GPU bridges
├── shaders/      the doll renderer's WGSL program
├── spatial/      the viewshed lane upload
├── streaming/    served map data into the map: fetch, chunk residency, draw buffers, memory
├── tests/        unit tests for the feature floor, and the source scrub the guards share
└── world/        the static world's browser loaders, GPU belts and CPU meshes
```

## How it works

```text
static side     /map-assets/<terrain>/ ──▶ streaming ──▶ world (decoded by the world format,
                                                           │   terrain and world-object crates)
                                                           ▼
                                               spatial_indexes, line of sight crates
                                                           │
                                                           ▼
authored side   mission crates (crates/mission/) ──▶ editing: hosted document, tools, selection, undo

draw path       map overlay crates ──▶ overlay ──▶ frame ◀── camera
                                              │
                                              ▼
                                   graphics_engine
```

The [mission](/documentation/glossary/g_to_m.md#mission) domain and the CRDT document the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) edits are the
[mission crates](/crates/mission/README.md), which reach no module of this crate. `world` holds the
static ground and never names the document, so inside the crate the authored mission and the
streamed world meet only in `editing`, which hosts the document, drives it and the map tools with
no browser in reach and asks the
[`spatial_indexes`](/crates/geometry/spatial_indexes/README.md) and line of sight crates where a
click or a sight line lands. `streaming` fetches a terrain's served files and chunks, the
[`world_file_formats`](/crates/world_formats/world_file_formats/README.md) crate and the world
format, terrain and world-object crates under `crates/` decode them, `world` composes and uploads
the ground and what stands on it, and `spatial` shows a viewshed as a lane. The map overlay crates
under `crates/map_overlay/` decide the lanes, in paint order, and the symbols drawn in them;
`overlay` keeps the lane preferences and uploads the symbology; `frame` holds the
render engine, which uploads the lanes both sides produce and hands the graphics engine a frame
packet whenever something changed; `camera` moves the render engine's view, with the cameras
themselves in `camera_math`. `diagnostics` measures the render
engine, and `doll` is a second, small renderer for the
[arsenal](/documentation/glossary/a_to_f.md#arsenal)'s preview, with its WGSL in `shaders/`.

`lib.rs` gates each module on the lowest tier that holds everything it needs:

| Module | Compiled with |
|---|---|
| `camera` | always; its one file, `viewport.rs`, only for wasm32 with `render` |
| `world`, `spatial`, `overlay`, `frame` | `world`; the GPU half of `frame` only for wasm32 with `render` |
| `streaming` | `streaming`; its browser host and loaders only for wasm32 with `render` |
| `editing` | `editing` |
| `diagnostics`, `doll` | `render` |

## Public surface

- `editing`: the hosted mission document and the headless editing layer, for the Mission
  Creator.
- `frame` (`RenderEngine`, `RafPump`, `EngineHandle`), `streaming` (`host`, `bridge`, the
  occluder loader, the residency) and `camera`: the map canvas of the Mission Creator and the
  debug benches.
- `doll`: the arsenal's preview.
- `frame`: the offline tools in `tools/developer_tools/`.

## Boundaries

- Depends on: `map_coordinates`, `camera_math` and `bytemuck` always; for `world`, `graphics_engine`,
  `render_primitives`, `terrain_elevation`, `terrain_relief`, `map_draw_lanes`, `label_layout`,
  `unit_symbology`, `overlay_instances`, `road_network`, `vegetation`, `place_names`,
  `spatial_indexes` and `terrain_line_of_sight`; for `streaming`, `serde`, `serde_json`, `rkyv`,
  `world_file_formats`, `flate2`, `prefab_catalog`, `world_chunks`, `satellite_imagery`,
  `water_bodies`, `world_store`, `interior_line_of_sight` and `world_line_of_sight`; for
  `editing`, `mission_payload`, `mission_validation`, `formation_geometry`, `mission_crdt`,
  `mission_document` and `mission_operations`; `wgpu`, `wasm-bindgen`, `js-sys`, `web-sys` on
  wasm32 and `browser_platform` on wasm32 for `render`; the terrain assets in `assets/terrains/`,
  fetched as `/map-assets` at run time and read by the tests.
- Used by: the Mission Creator, the mission library and the debug benches under
  `apps/frontend/src/`; the tools in `tools/developer_tools/src/`; and the gates in
  `tools/xtask/src/verifications/` that read this tree.
- Rules:
  - the crate's tests run with `--all-features` (`map_engine_tests_require_all_features` in
    `tests/feature_gate_tripwire.rs`);
  - `cargo xtask verify engine-layers` holds the layering, by the rule numbers it prints:
    - 3a and 3b: only `frame/mod.rs` names `graphics_engine::frame` (five lines), and the
      graphics engine's GPU modules are named only in `frame/mod.rs` and `frame/pump.rs`;
    - 5: no `web_sys`, `leptos` or `wasm_bindgen` anywhere under `editing/`;
    - 7: `world/` names neither `yrs`, nor `mission_crdt`, `mission_document` or
      `mission_operations`, nor `crate::editing`;
  - the mission crates depend on no module of this crate and no world or graphics crate
    (`cargo xtask verify crate-tiers`).
