# Map engine source

The source of the `website_map_engine` library: eleven top-level modules, each compiled only under
the crate feature it needs, and the crate's own unit tests. It holds no UI framework: browser code
compiles only for wasm32, most of it only with the `render` feature as well, and everything else
builds and tests natively.

## Contents

```text
apps/website/map-engine/src/
├── camera/       the map's orthographic camera, the doll's orbit camera and the grid reference
├── data/         the mission domain and the Yjs CRDT document the Mission Creator edits
├── diagnostics/  readback checks, the benchmark and statistics, clocks and console macros
├── doll/         the arsenal's 3D mannequin preview: scene, picking and its own renderer
├── editing/      the live mission document, its undo drive and the headless map tools
├── frame/        the render engine, its batch list and upload belts, the frame vocabulary
├── io/           on-disk formats: rkyv archives, containers, density grids and POD layouts
├── lib.rs        the crate root: declares each module behind the feature it belongs to
├── overlay/      what the map draws on top, and in what order: the 48 lanes and the symbology
├── shaders/      the doll renderer's WGSL program
├── spatial/      BVHs, point indexes, and line of sight over terrain, the world and buildings
├── streaming/    served map data into the map: fetch, chunk residency, draw buffers, memory
├── tests/        unit tests for the feature floor, and the source scrub the guards share
└── world/        the static world: terrain, what stands on it, and the inside of its buildings
```

## How it works

```text
static side     /map-assets/<terrain>/ ──▶ streaming ──▶ world (formats decoded by io)
                                                           │
                                                           ▼
                                               spatial: picking, line of sight
                                                           │
                                                           ▼
authored side   data::scenario ──▶ data::store ──▶ editing: tools, selection, undo

draw path       overlay: lanes, symbols ──▶ frame ◀── camera
                                              │
                                              ▼
                                   website-graphics-engine
```

`data` holds the [mission](/documentation_v2/glossary.md#mission) domain and the CRDT document the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) edits; it reads no other module
of the crate. `world` holds the static ground and never names `data`, so inside the crate the
authored mission and the streamed world meet only in `editing`, which drives the document and the
map tools with no browser in reach and asks `spatial` where a click or a sight line lands.
`streaming` fetches a terrain's served files and chunks, `io` decodes their binary formats, `world`
composes the ground and what stands on it, and `spatial` answers geometric queries over them.
`overlay` decides the lanes, in paint order, and the symbols drawn in them; `frame` holds the
render engine, which uploads the lanes both sides produce and hands the graphics engine a frame
packet whenever something changed; `camera` places the view. `diagnostics` measures the render
engine, and `doll` is a second, small renderer for the
[arsenal](/documentation_v2/glossary.md#arsenal)'s preview, with its WGSL in `shaders/`.

`lib.rs` gates each module on the lowest tier that holds everything it needs:

| Module | Compiled with |
|---|---|
| `camera` | always; its `viewport.rs` only for wasm32 with `render` |
| `data` | always; its `scenario` half with `scenario`, its `store` half with `store` |
| `world`, `spatial`, `overlay`, `frame` | `world`; the GPU half of `frame` only for wasm32 with `render` |
| `io`, `streaming` | `io`; most of `streaming` with `streaming`, its browser host only for wasm32 with `render` |
| `editing` | `editing` |
| `diagnostics`, `doll` | `render` |

## Public surface

- `data::scenario`: the mission compiler and validator, for the
  [API](/documentation_v2/glossary.md#api) at the default `scenario` tier and for the Mission
  Creator.
- `data::store` and `editing`: the mission's Yjs document and the headless editing layer, for the
  Mission Creator.
- `frame` (`RenderEngine`, `RafPump`, `EngineHandle`), `streaming::host`, `camera`, `overlay`,
  `spatial` and `world`: the map canvas of the Mission Creator and the debug benches.
- `doll`: the arsenal's preview.
- `world`, `io`, `spatial`, `streaming` and `overlay`: the offline tools in
  `tools_v2/developer-tools/`.

## Boundaries

- Depends on: `website-graphics-engine` from the `world` tier up; `serde`, `serde_json` and
  `thiserror`; `yrs` for `store`; `png`, `rkyv` and `flate2` for the world, archive and streaming
  tiers; `bytemuck` and `earcutr` always; `wgpu`, `wasm-bindgen`, `js-sys`, `web-sys` and
  `gloo-net` on wasm32; `contracts_v2/rules/kit-aliases.json`, embedded at build time; the terrain
  assets in `assets_v2/terrains/`, fetched as `/map-assets` at run time and read by the tests.
- Used by: the API's missions and operations domains under `apps/website/api_v2/src/` (`data`
  only); the Mission Creator, the mission library, the DTOs and the debug benches under
  `apps/website/frontend/src/v2/`; the tools in `tools_v2/developer-tools/src/`; and the gates in
  `tools_v2/xtask/src/verifications/` that read this tree.
- Rules:
  - the crate's tests run with `--all-features` (`map_engine_tests_require_all_features` in
    `tests/feature_gate_tripwire.rs`);
  - `cargo xtask verify engine-layers` holds the layering, by the rule numbers it prints:
    - 3a and 3b: only `frame/mod.rs` names `website_graphics_engine::frame` (eight lines), and the
      graphics engine's GPU modules are named only in `frame/mod.rs` and `frame/pump.rs`;
    - 4: `data/scenario/` names neither `data::store`, the graphics engine nor any of the nine
      modules the gate lists (every top-level module but `data` and `editing`), so the API's
      `scenario` build pulls no graphics crate, `png`, `rkyv` or `flate2`; two store-gated tests
      are pinned exceptions;
    - 5: no `web_sys`, `leptos` or `wasm_bindgen` anywhere under `editing/`;
    - 7: `data/` names neither the graphics engine nor those nine modules, and `world/` names
      neither `crate::data` nor `yrs`.
