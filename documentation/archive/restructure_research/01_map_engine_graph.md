**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# Map engine module graph

I've finished the read-only analysis. Line counts are `wc -l` and include comments and blank lines. Unless I say otherwise, edge counts are the number of distinct `.rs` files that name `crate::<module>` in code, with comment lines (`//`, `///`, `//!`) excluded. The "prod" figure leaves out files under `tests/` or named `*_tests.rs`.

Three findings matter most:
- The crate is one large dependency cycle; only `data` and `io` sit outside it.
- `RenderEngine` has impl blocks spread across six modules. Rust only allows those in the crate that defines the type.
- The repo's own law checker hard-codes this crate's file paths and feature names.

## 1. Top-level module dependency graph (map-engine)

**Edges, "source -> target: files (prod)"**

| From | To |
|---|---|
| camera | frame 1 (1), overlay 1 (1) |
| data | none. The only mentions of `crate::editing` are in READMEs. |
| diagnostics | camera 9, doll 1, frame 13, overlay 3, world 10 (all prod) |
| doll | camera 5 (4), frame 1 (1) |
| editing | camera 6 (5), data 35 (30), frame 1, overlay 3 (2), spatial 22 (14), world 1 |
| frame | camera 2, diagnostics 3, overlay 10, world 8 |
| io | none. The `io -> world` hits are doc comments only, so io is a leaf. |
| overlay | frame 7, spatial 1, world 5 |
| spatial | frame 1, io 2 (1), overlay 1, streaming 5 (4), world 18 (14) |
| streaming | diagnostics 2, frame 3, io 5 (2), overlay 10 (9), spatial 4, world 29 (23) |
| world | camera 2, diagnostics 5, frame 8, io 19 (13), overlay 9, spatial 9 (6), streaming 13 (10) |
| shaders/ | not a Rust module: only `doll.wgsl` and a README |
| src/tests/ | `source_scrub.rs`, used by `data/store` (1 file) and `editing/tools` (2 files); `feature_gate_tripwire.rs` |

No cross-module path uses `crate::{...}` group imports or `super::` (checked).

**Structure:** `{camera, frame, overlay, world, spatial, streaming, diagnostics, doll}` form one strongly-connected cluster. The leaves are `io` and `data`. `editing` sits on top: it depends on data, camera, frame, overlay, spatial and world, and nothing depends on it.

**Two-way cycles and the back-edge lines**
1. **camera <-> frame**
   - `camera/viewport.rs:6` uses `frame::engine::RenderEngine` (the file is all `impl RenderEngine`).
   - `frame/engine.rs:13` and `frame/boot.rs:13` use `camera::ortho::state::OrthoCamera`.
2. **frame <-> overlay**
   - overlay -> frame (7 files): `overlay/lanes.rs:230,232` (`lane_id` returns `crate::frame::LaneId`), `lanes_prefs.rs:12-18`, `symbology/atlas/gpu.rs:6,16,38`, `symbology/instances/{bridge_1,bridge_2,bridge_3,lanes}.rs`, `symbology/markers.rs`.
   - frame -> overlay (10 files): `bindings.rs:17,82`, `encode.rs:11-12`, `engine.rs:18-20`, `boot.rs:18-19,36-37`, `upload/{polygons,hairlines,text,selection}.rs`, `cull.rs:15-16`, `lifecycle.rs:12-15,198-205`.
3. **frame <-> world**
   - frame -> world: mostly `world::scene::{ANCHOR, EVERON_BOUNDS, INITIAL_*}` and `world::terrain::satellite::textures::{TexLane, PendingTex}`, in `encode.rs:13-14`, `engine.rs:21-22`, `boot.rs:20-22,312`, `upload/*`, `cull.rs:17`, `lifecycle.rs:16-17`.
   - world -> frame: `world/environment/{locations/loader.rs:25, buildings/buffers.rs:6-11, vegetation/buffers.rs:6-11, vegetation/loader.rs:19}` and `world/terrain/{relief/host.rs:23, satellite/quadtree/decode.rs:57, quadtree/mod.rs:6, satellite/textures.rs:6-7}`.
4. **frame <-> diagnostics**
   - frame -> diagnostics: `frame/engine.rs:14`, `frame/boot.rs:14` (`diagnostics::timing::gpu::GpuTimer`) and `frame/lifecycle.rs:6` (`perf_now_ms`).
5. **overlay <-> world**
   - overlay -> world: `overlay/lanes_prefs.rs:125` (`scene::ANCHOR`), `symbology/text_packing.rs:16-19,69` (`world::environment::locations::{peaks, route_placement, towns}`), `symbology/text_metrics.rs:16`, `symbology/instances/bridge_1.rs:14`, `instances/lanes.rs:12`.
   - world -> overlay: `locations/{loader, towns, peaks}.rs`, `buildings/buffers.rs:8-9`, `vegetation/{buffers, loader, canopy}.rs`, `terrain/relief/host.rs:6-7,82-130`, `terrain/satellite/textures.rs:8`.
6. **overlay <-> spatial**
   - `overlay/symbology/instances/bridge_1.rs:49,236` uses `spatial::indexing::cluster::ClusterIndex`.
   - `spatial/los/terrain/overlay.rs:8` uses `overlay::lanes::LaneRole`.
7. **spatial <-> world**
   - spatial -> world (18 files): mostly `world::architecture::*` (17 hits) and `world::terrain::dem::{manifest, sampling}`.
   - world -> spatial:
     - `world/architecture/{compound/instances.rs:6, compound/assembly.rs:6-8, section/index.rs:8, section/cutter.rs:6-8, blueprint/attribution_1.rs:6}` use `spatial::bvh::*`.
     - `world/terrain/dem/sample/mod.rs:34-67` has 12 `pub use crate::spatial::los::terrain::*` lines. No caller anywhere in the repo uses `dem::sample`, so this back-edge is a dead re-export layer.
8. **spatial <-> streaming**
   - spatial -> streaming: `spatial/los/world/state.rs:12` and `raycast.rs:20` (`streaming::scheduler::chunk_math::{TerrainSizeM, chunk_id}`); `residency.rs:14` and `placed.rs:7` (`streaming::loaders::chunk::WorldChunk`); the test `los/world/tests/occluder.rs:14-15`.
   - streaming -> spatial: `streaming/scheduler/state.rs:10`, `host/queries.rs:40`, `loaders/world_loader/viewport.rs:89`, `loaders/occluder_loader.rs:9-14`.
9. **streaming <-> world**
   - world -> streaming (13 files):
     - `world/environment/locations/loader.rs:11-32`
     - `buildings/footprint.rs:6-8`
     - `vegetation/loader.rs:20-26`
     - `vegetation/canopy.rs:7-8`
     - `terrain/dem/loader.rs:6-13`
     - `terrain/roads/airfield.rs:11-12`
     - `terrain/satellite/quadtree/{mod.rs:7-18, bootstrap.rs:52-73, selection.rs:56}`
     - `terrain/water/loader.rs:8-20`
     - tests: `prefab_tests.rs:75`, `regions_tests.rs:50`, `canopy_tests.rs:6`
   - streaming -> world: 29 files, about 60 distinct items, mostly `world::environment::{buildings::prefab, classify, vegetation}` and `world::terrain::{dem, roads, water, satellite}`.
10. **diagnostics <-> world**
    - world -> diagnostics: 19 calls to the `diagnostics::platform::console::{error!, warn!, log!}` macros. They are in `world/terrain/satellite/quadtree/{downloads, bootstrap, basemap, selection, retry}.rs`.
    - diagnostics -> world: `world::scene::ANCHOR` in 10 files, plus `stress_chunk_into` at `bench/frame_1.rs:111`.

**Three-way cycles (no direct back-edge)**
- camera -> overlay -> frame -> camera, via `camera/viewport.rs:119` (`overlay::symbology::instances::symbols::cluster_mode`).
- world -> camera -> frame -> world, via `world/terrain/dem/grid.rs:43-58` and `relief/hillshade.rs:70`, which use `camera::math::shaping::round`, a 15-line file.
- diagnostics -> doll -> frame -> diagnostics, via `diagnostics/readback/doll.rs:6-12`, `doll/renderer/lifecycle_1.rs:9` (`frame::boot::instance_descriptor`) and `doll/renderer/lifecycle_2.rs:20`.
- streaming -> diagnostics -> world -> streaming, via `streaming/host/preferences.rs:16` and `loaders/occluder_loader.rs:123` (console macros).
- spatial -> frame -> overlay -> spatial, via `spatial/los/terrain/overlay.rs:6-7`.

**Non-import blockers to extraction**
- **Inherent impls on a foreign type (E0116).** Blocks starting `impl RenderEngine` (type defined at `frame/engine.rs:82`): frame 40, overlay 42, diagnostics 19, world 13, camera 10, spatial 2.
  - Outside frame they are in `overlay/symbology/instances/bridge_{1,2,3}.rs`, `instances/lanes.rs`, `lanes_prefs.rs`, `atlas/gpu.rs`, `world/environment/{buildings,vegetation}/buffers.rs`, `world/terrain/satellite/textures.rs`, `camera/viewport.rs`, `spatial/los/terrain/overlay.rs` and every `diagnostics/readback/*` and `diagnostics/bench/*` file.
  - There are also 12 `impl DollEngine` blocks: `doll/renderer/lifecycle_1.rs` (10), `lifecycle_2.rs` (1) and `diagnostics/readback/doll.rs` (1).
- **`pub(crate)` counts per module** (items another crate would need opened up): frame 122, streaming 85, spatial 80, overlay 62, world 46, doll 37, diagnostics 28, camera 24, io 2, data 4. `pub(super)`: data 461, streaming 110, world 37.
- **Source-text tests that `include_str!` other modules' files:**
  - `overlay/tests/tests/draw_order_t748_comments_bind_pick_bridge.rs`, `draw_order_t780_connections_bind_pick_bridge.rs` and `draw_order_t808_symbology_bind_paths.rs` each include about 33 files from frame/, diagnostics/, camera/, world/ and spatial/.
  - `editing/tools/line_of_sight/tests/wash_palette.rs:91` includes `world/terrain/relief/host.rs`.

**Second-level detail (prod, occurrences, including some doc mentions)**
- **world**
  - `architecture` (18 files, 2,953 lines): depends only on `spatial::bvh` and `io::archives`.
  - `environment` (3,721 lines): depends on io, overlay, streaming and frame.
  - `terrain` (5,743 lines): depends on `spatial::los`, `diagnostics::platform`, io, streaming, overlay, `camera::math` and frame.
  - `mesh.rs` and `scene.rs` have no crate-internal deps.
- **streaming**
  - `memory` (937 lines) has no deps.
  - `scheduler/chunk_math.rs` (129 lines) has no deps.
  - `loaders/chunk.rs` depends on `world::environment::{buildings::prefab, classify}`.
- **spatial**
  - `bvh` (1,101 lines) has no deps.
  - `indexing` (737 lines) depends only on `world::environment`.
  - `los/interior` depends only on `world::architecture`.
- **overlay**: `lanes.rs`, `lod.rs`, `symbology/{labels, links, roles}` have no deps except `frame::LaneId` in `lanes.rs`.
- **camera**: `math`, `ortho`, `orbit` and `grid_reference` have no deps. Only `viewport.rs` (wasm and render only) reaches frame and overlay.

**data/ breakdown** (lines, with test lines in brackets)

data/scenario (`scenario` feature):
| Submodule | Files | Lines [test] |
|---|---|---|
| ast | 8 | 1,177 [172] |
| ballistics | 55 | 14,474 [7,812] |
| compiler | 36 | 8,136 [5,113] |
| extensions | 36 | 5,111 [2,273] |
| slot_line | 4 | 137 [68] |
| validation | 17 | 3,751 [1,667] |

data/store (`store` feature):
| Submodule | Files | Lines [test] |
|---|---|---|
| crdt | 12 | 1,196 [364] |
| operations | 65 | 9,050 [2,194] |
| rows | 47 | 13,623 [7,566] |
| selection.rs | 1 | 99 |
| tests | 1 | 95 |

- data imports no other top-level module and no graphics crate. Its external crates are serde, serde_json, thiserror, libm, yrs, and `js_sys` in `store/crdt/undo_groups/clocks.rs`.
- store -> scenario: 14 files (`compile` 53 hits, `flatten` 18, `tactical_graphics` 6, `validate` 2).
- scenario -> store: test-only, in `scenario/compiler/payload/tests/cases_1.rs:278` and `scenario/compiler/flatten/tests/mod.rs:138` (`MissionDocCore`, under `cfg(feature = "store")`). These two files are pinned as `RULE4_PIN`.

## 2. Feature gating

**`lib.rs`**
- `camera` and `data` are ungated.
- `editing` is gated on `editing`.
- `frame`, `overlay`, `spatial`, `world` are gated on `world`.
- `diagnostics`, `doll` are gated on `render`.
- `io`, `streaming` are gated on `io` (not `streaming`, whatever the Cargo comment says).
- `source_scrub` is gated on `all(test, store)`.
- `data/mod.rs` gates `scenario` on `scenario` and `store` on `store`.

**Counts.** There are 257 lines mentioning a cfg feature (including comments). Occurrences per feature: scenario 81, render 63, streaming 55, io 43, world 9, store 6, bvh 5, editing 2.

**Classified attribute sites (module × feature × target)**
| Module | Gates |
|---|---|
| camera | render 1 mod |
| data | scenario 79 items + 1 mod; store 3 items + 1 mod. All 79+3 item gates are inside `data/store/rows/tests/*`, `operations/apply_faction/tests`, `scenario/compiler/{flatten,payload}/tests`, so they are test-only. |
| diagnostics | render 15 mods |
| doll | render 5 mods |
| editing | render 2 `use` (`tools/selection/gesture.rs:36` and `selection/mod.rs`, both for `EngineHandle`) |
| frame | render 8 mods + 3 `use` + 1 item |
| io | io 16 mods (redundant, since lib.rs already gates io) |
| overlay | streaming 9 mods + 1 test fn; render 6 mods |
| spatial | bvh 4, io 3, streaming 7, render 1 (all mods) |
| streaming | streaming 16, io 2, render 5 (all mods) |
| world | streaming 20 mods; io 16 mods + 3 items; render 9 mods + 3 items; world 1 mod + 3 test fns |
| src/tests | `feature_gate_tripwire.rs` uses `cfg!()` over all 8 features |

**Gates inside modules (beyond the `mod` lines)**
- `world/architecture/blueprint/archive.rs:17,61,67` (`io`).
- `world/scene.rs:22,26,30` (`INITIAL_TARGET`, `INITIAL_ZOOM`, `EVERON_BOUNDS` under `all(wasm32, render)`).
- `frame/mod.rs` re-exports (`all(wasm32, render)`).
- `editing/tools/selection/gesture.rs:36`.
- Test fns: `peaks_tests.rs:265`, `raw_tests.rs:256,293`, `slots/tests/cases_1.rs:509`.
- Everything else is a submodule `mod` declaration.

The submodule gates inside `world` line up with its outgoing edges, which suggests natural cut lines:
- `architecture`, `mesh`, `dem/raw` are under `io`.
- `environment/*`, `roads`, `water/{vectors, mesh}`, `satellite/streamer`, `scene` are under `streaming`.
- `*/loader`, `relief/host`, `satellite/{textures, quadtree}` are under `render`.

**`cfg(target_arch = "wasm32")` lines per module**

| Module | Total | Combined with a feature | `not(wasm32)` |
|---|---|---|---|
| camera | 1 | 1 | 0 |
| data | 2 | 0 | 0 |
| diagnostics | 15 | 15 | 0 |
| doll | 5 | 5 | 0 |
| editing | 5 | 2 | 2 |
| frame | 18 | 14 | 0 |
| io | 0 | 0 | 0 |
| overlay | 6 | 6 | 0 |
| spatial | 1 | 1 | 0 |
| streaming | 16 | 5 | 5 |
| world | 12 | 12 | 0 |

Wasm-only gates that have no feature attached: `streaming/host/mod.rs:6` (`#![cfg(wasm32)]`), `streaming/memory/budget/{mod.rs, stats.rs, platform.rs}`, `editing/tools/viewshed_scheduler/host.rs:37`, `frame/mod.rs:99-121`, and `data/store/crdt/undo_groups/clocks.rs:70`.

## 3. Use of `website_graphics_engine`

**Map-engine modules × graphics-engine submodule** (non-comment occurrences)

| Module | draw | layout | text | frame | device | pipeline | loop |
|---|---|---|---|---|---|---|---|
| frame | 8 | 6 | | 8 | 1 | 1 | 3 |
| overlay | 7 | 2 | 20 | | | | |
| world | 17 | 3 | | | | | |
| diagnostics | 1 | 13 | | | | | |
| spatial | | 1 | | | | | |

- **frame:** draw is `draw::{encode, polygons, lines, geometry, cull::{oracle, compute}}`; frame covers the `damage`, `packet`, `present`, `CameraUniform`, ids, batch, buffer and text groups in `frame/mod.rs`; device is `device::buffers`; pipeline is `pipeline as pipelines`; loop is `r#loop::{FrameTarget, RafPump}` in `pump.rs`.
- **overlay:** text is `text::{scale::REF_ZOOM, layout, atlas, font, metrics, pack}`; draw is `draw::{lines, grid, geometry}`; layout is `ATLAS_GLYPH_COUNT`.
- **world:** draw is `draw::{triangulate, compose, geometry, instances, lines, polygons}` in `world/mesh.rs`, `scene.rs`, `environment/*/buffers.rs`; layout is `QuadInstance`, `BuildingInstance`.
- **diagnostics:** layout is `LineVertex`, `IconInstance`, `QuadInstance`, `BuildingInstance`, `CHUNK_CAPACITY`; draw is `draw::encode`.
- **spatial:** layout is `QuadInstance` in `los/terrain/overlay.rs:72`.
- camera, data, doll, editing, io and streaming make no direct use. streaming reaches `REF_ZOOM` indirectly through `overlay::lod` and `overlay::symbology::labels::glyph_math`.

**Graphics-engine internal graph (prod plus tests)**
| Module | Files / lines | Imports | CPU vs wasm |
|---|---|---|---|
| device | 6 / 374 | none | `buffers/pool.rs` has a wasm-gated `wgpu::Buffer` field and method; `readback.rs` is pure |
| draw | 17 / 1,966 | frame (3), shaders (1, test) | wasm-only: `encode`, `lines`, `polygons`, `cull::compute`. CPU: `compose`, `geometry`, `grid`, `instances`, `triangulate`, `cull::oracle` |
| frame | 11 / 769 | text (`atlas.rs:135`, `text::pack`) | CPU: `camera`, `damage`, `ids`. wasm-only: `atlas`, `batch`, `buffers`, `packet`, `present`, `text` |
| layout | 1 / 42 | draw, text | CPU; pure re-export layer |
| loop | 3 / 309 | none | `pump.rs` has a CPU `FrameTarget`/`RafPump` core, with `wasm_bindgen`/`web_sys` rAF under cfg at lines 128-163 |
| pipeline | 7 / 556 | shaders | every submodule is wasm-only (wgpu) |
| shaders | 2 / 83 | none | CPU (`SHADER_WGSL` const) |
| text | 8 / 586 | none | pure CPU |

- The crate has no features. The only external dependencies are bytemuck and earcutr, plus wgpu, wasm-bindgen, js-sys and web-sys on wasm32.
- Nothing outside map-engine imports it, apart from probe strings in verification-core.
- A CPU/wgpu split would move: `draw::{encode, lines, polygons, cull::compute}`, `frame::{atlas, batch, buffers, packet, present, text}`, `pipeline/*`, `device::buffers::pool` (GPU half) and `loop` (rAF half).
- That split has two internal frictions: `draw::encode/lines/polygons` import `frame::{batch, buffers, packet, text}`, and `frame/atlas.rs` imports `text::pack`.

## 4. Consumers

| Consumer | Dependency line | Features | `website_map_engine::<module>` uses (occurrences / files) |
|---|---|---|---|
| `apps/website/api_v2/Cargo.toml:73` | normal | default = `scenario` | data 43 / 23. All are `data::scenario` (37 second-level). |
| `apps/website/frontend/Cargo.toml:33` | all targets | `world, io, store, editing` (editing pulls in streaming) | |
| `apps/website/frontend/Cargo.toml:135` | wasm32 | `render, streaming` | |
| `apps/website/frontend/Cargo.toml:150` | dev-deps | `streaming` | |
| frontend, all three lines combined | | | data 208/88, editing 158/69, world 64/17, streaming 63/27, overlay 27/11, spatial 28/9, frame 21/13, camera 20/18, doll 3/1 |
| `tools_v2/developer-tools/Cargo.toml:24` | normal | `world, streaming, io, bvh, scenario` | world 129/34, io 69/19, spatial 57/14, data 20/11 (`data::scenario` only), streaming 18/11, overlay 4/1, camera 1/1 |
| `map-engine/tests/` (integration) | self | | camera 4 (`ortho::state`), data 4 (`store`, `scenario::compile`) |

**Frontend detail at the second level:**
- data: `data::scenario` 123, `data::store` 79.
- editing: `editing::{tools 54, hosted_commands 46, host 11, persist 11, lanes 7, history 4, commands 4, selection_universe 3, picking 2, batch 1, routing 1}`.
- streaming: `streaming::{host 35, bridge 17, loaders 3, memory 1, scheduler 1}`.
- world: `world::{architecture 50, terrain 12, mesh 1}`.
- spatial: `spatial::{los 17, bvh 9}`.
- frame, camera, overlay, doll: `frame::engine` 11 (plus `EngineHandle`, `RafPump`), `camera::{ortho 12, grid_reference 7}`, `overlay::{symbology 17, lanes 6, fire_mission_marks 2}`, `doll::renderer` 2.

**developer-tools detail:**
- `world::{architecture 50, environment 49, terrain 25}`.
- `io::{archives 39, containers 13, density 11, pod 6}`.
- `spatial::{bvh 31, los 26}`.
- `streaming::{loaders 16, scheduler 2}`.
- Some doc comments are stale: `website_map_engine::{bvh, building_blueprint, dem, geometry}` in `blueprint/bvh/construction.rs:3,412`, `blueprint/ingest.rs:5`, `map_verification/labels.rs:4`, `world_export_pipeline/vegetation_density.rs:2,21`.

**Crates that don't depend on either engine:** `apps/website/offline-service-worker`, `tools_v2/xtask`, `apps/fleet_host_agent`, `apps/ticketboard`.

**Tooling that names the crates, feature names or source paths (will need edits in a split):**
- `tools_v2/verification-core/src/repository_laws/engine_layers/rules.rs`:
  - `MAP_CRATE_REL`, `RULE3A_PIN` and `RULE3B_PIN` pin `map-engine/src/frame/mod.rs` and `frame/pump.rs`.
  - `RULE4_RE`, `RULE7_DATA_RE`, `RULE7_WORLD_RE` and `DATA_REL`/`WORLD_REL`/`SCENARIO_REL`/`EDITING_REL` hard-code paths.
  - `mod.rs` holds rules 1–7.
- `verification-core/src/repository_laws/crate_dependencies.rs:33-80`: the per-crate forbidden-dependency rules.
- `tools_v2/xtask/src/commands/ci/task_definitions.rs:415-425`: `cargo test/clippy -p website-map-engine --all-features`.
- `xtask/.../wave_execution/gate/gate_dispatch.rs:155-233` and `touch.rs:342-357`: `--all-features`.
- `xtask/src/verifications/architecture/editor_orbat_coherency.rs:200-214`: feature tiers `"scenario store"` and `"render"`.
- `.github/workflows/ci.yml:114-122` and `editor-gates.yml:21`.
- `src/tests/feature_gate_tripwire.rs`: asserts all 8 features are on.

## 5. Shared foundational types

- **No `core`/`math`/`geom` module exists.** Coordinates are raw arrays: `[f64; 2]` ×192, `[f64; 3]` ×257, `[f64; 4]` ×26, `[f32; 2]` ×23, `[f32; 3]` ×16. The only alias is `pub type Bbox = [f64; 4]` at `streaming/scheduler/chunk_math.rs:7`.
- **What de facto plays that role:**
  - `world/scene.rs`: `ANCHOR` (pub), imported by diagnostics (10 files), frame (7), overlay (4) and world (1). Also `EVERON_BOUNDS`, `INITIAL_*` (pub(crate), render-gated). The file itself depends on graphics `draw::{geometry, instances::QuadInstance}`.
  - `camera/math/{glmat4, shaping, dimensions}.rs` (370 lines, no deps), used by camera, doll (3), world (2) and diagnostics (1).
  - `streaming/scheduler/chunk_math.rs` (no deps): `Bbox`, `TerrainSizeM`, `chunk_id*`. Used by spatial and world.
  - `overlay/lanes.rs`: `LaneRole`, `lane_id`, `role_id`. Used by frame, world, streaming, spatial.
  - `overlay/lod.rs`: `class_visible`, `INSTANCE_BUDGET`, `REF_ZOOM`.
  - `io/archives/codec.rs`: `BinaryError`, `to_bytes`, `access_checked`.
- **IDs:** map-engine defines no ID newtypes. `LaneId`, `PipelineId` and `BindGroupId` come from `website_graphics_engine::frame::ids` through `crate::frame`.
- **Errors:** each subsystem has its own error enum; there is no shared error type.
  - data: 15 enums, plus `CompileError`, `ApplyFactionError`, `PlaceOrbatError`.
  - io: `BinaryError`, `TbddError`.
  - spatial: `BvhParseError`, `ArchiveProjectionError`.
  - streaming: `ChunkBinError`, `WorldError`.
  - world: `PngError`, `TbdSatError`, `CompoundError`.
  - camera: `GridParseError`.
  - thiserror is used only in data (14 files) and streaming (1).
- **`core::culling::lod`** (Cargo.toml:27) does not exist; it's a leftover name. The real module is `apps/website/map-engine/src/overlay/lod.rs:16` (`pub use website_graphics_engine::text::scale::REF_ZOOM`), reached by streaming belts through `overlay/symbology/labels/glyph_math.rs:6`. Ticket `.ai/tickets/T-1068.toml` already records this, and that the `renderers::primitives::triangulate::triangulate_simple` path in the same comment is really `website_graphics_engine::draw::triangulate`.
- **Unused dependency:** `earcutr` is declared in map-engine's Cargo.toml but no `.rs` file uses it (README only).

## 6. Sizes (lines / files, prod vs tests)

**map-engine** (total about 126.9k lines; prod about 78.0k)
| Module | Prod files / lines | Test files / lines |
|---|---|---|
| data | 196 / 29,625 | 89 / 27,324 |
| world | 87 / 12,731 | 34 / 5,857 |
| streaming | 59 / 7,674 | 8 / 2,124 |
| editing | 72 / 7,167 | 28 / 4,109 |
| spatial | 39 / 5,953 | 8 / 3,487 |
| overlay | 29 / 4,892 | 16 / 3,565 |
| diagnostics | 21 / 3,091 | 0 |
| frame | 13 / 2,347 | 1 / 237 |
| io | 23 / 2,032 | 9 / 1,488 |
| doll | 14 / 1,284 | 3 / 233 |
| camera | 16 / 1,215 | 2 / 213 |
| src/tests | 2 / 173 | — |
| `map-engine/tests/` (integration) | — | 5 files, 843 lines total |

**graphics-engine** (total about 4.7k lines)
| Module | Files / lines | Test lines |
|---|---|---|
| draw | 17 / 1,966 | 335 |
| frame | 11 / 769 | 44 |
| text | 8 / 586 | 42 |
| pipeline | 7 / 556 | 0 |
| device | 6 / 374 | 132 |
| loop | 3 / 309 | 126 |
| shaders | 2 / 83 | 70 |
| layout | 1 / 42 | 0 |

## 7. WGSL shaders

- **`apps/website/graphics-engine/src/shaders/shader.wgsl`** (343 lines)
  - Loaded at `graphics-engine/src/shaders/mod.rs:9` as `pub const SHADER_WGSL = include_str!("shader.wgsl")`.
  - Used by `pipeline/mod.rs:40-43` (`create_map_shader`, wasm only), which map-engine calls from `frame/boot.rs:26,121` through `crate::frame::pipelines`.
  - Read again directly by `draw/cull/oracle.rs:111` (`include_str!("../../shaders/shader.wgsl")`, a source-text check) and by tests `draw/tests/instances_tests.rs:62` and `shaders/tests/contract_tests.rs:15`.
  - The compute cull (`frame::compute::IconComputeCull::create`) reuses the same module: `frame/boot.rs:269` and `diagnostics/readback/compute_cull.rs:46`.
- **`apps/website/map-engine/src/shaders/doll.wgsl`** (51 lines): loaded only at `doll/renderer/lifecycle_1.rs:174` via `include_str!("../../shaders/doll.wgsl")`.
- There are no other `.wgsl` files in the repo.
