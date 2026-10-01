**Status:** archived — see [the restructure program](/documentation_v2/restructure/README.md)

# Fine-grained engine analysis

I've mapped both crates down to the second and third level. The main findings:

- **`data` is a true leaf.** Only `editing` imports it.
- **`io` is also a real leaf.** Its one edge into `world` is a dead doc link.
- **A large share of the module cycles are fake.** About 18 re-export shim `mod.rs` files cause them.
- **The `impl RenderEngine` blocks split cleanly into three access patterns.** That points to a narrow "lane sink" interface for layers, not a `Vec<Box<dyn RenderLayer>>`.
- **The biggest structural blocker is JS export.** All but 9 of the 136 out-of-frame `impl RenderEngine` blocks are `#[wasm_bindgen]`, and Rust only allows inherent impls in the crate that defines the type.

How the numbers were made: line counts are prod/test, where test means the path contains `tests`. Two test folders don't match that rule, so their lines count as prod: `streaming/memory/budget/t938_6` (~305 lines) and `world/terrain/satellite/streamer/t935_10` (~320 lines). Edges are counted from `crate::…` paths in prod files, with the shim `mod.rs` files excluded. `super::` paths are not counted. All paths are under `/home/user/TBD-reforger/apps/website/map-engine/src/` unless prefixed `ge/`, which means `/home/user/TBD-reforger/apps/website/graphics-engine/src/`.

---

## 0. Inventory (prod/test lines)

**data/scenario (15.7k / 17.1k)**

| Module | Lines (prod/test) | Imports from other scenario modules |
|---|---|---|
| `ast` | 1005/172 | authoring 312, entities 289, factions 200, scenario 191 → `extensions::objectives` (win_conditions) |
| `extensions` | 2838/2273 | authored 216, environment 642, modules 367, objectives 769, radio 344, tactical_graphics 476 → only each other (plus `compile_payload` in tests) |
| `compiler/payload` | 341/762 | → `ast::factions`, `extensions` |
| `compiler/kit` | 126/72 | leaf |
| `compiler/flatten` | 2543/4279 | 17 files (compile_graph 398, diagnostics 227, roster 170, …) → ast, kit, payload, extensions, validator, wire_safety |
| `validation/validator` | 1668/1411 | orbat 307, cargo 262, loadout 234, registry 230, … → wire_safety, `payload::terrain_bounds` |
| `validation/wire_safety` | 406/256 | leaf |
| `slot_line` | 69/68 | leaf |
| `ballistics` | 6662/7812 | **no imports outside ballistics.** Children: solver 1057/1216, calibration 1448/849, flight_model 709/984, catalog 456/358, dispersion 480, fire_mission 447, fire_mission_comparison 401, agreement_cases 375, fuze 312, crest_clearance 245, solution_wording 231, battery 224, angular_units 133, wind 75 |

- Internal ballistics cycles: wind↔flight_model, angular_units↔catalog, fuze↔fire_mission, battery↔agreement_cases.
- The only importer of ballistics outside it is a doc mention in `overlay/fire_mission_marks.rs:5`.

**data/store (13.7k / 10.2k)**
- `crdt` 682/364 (id_arrays 294, soa 113, undo_groups 262). Uses `yrs`; also `js_sys` for the clock.
- `rows` 6057/7566: 34 files, almost all `impl MissionDocCore`. `yrs` is used in 25 files.
- `selection` 99: `impl MissionDocCore`.
- `operations` 6856/2194:
  - entity 2798/743 (zones, roster, vehicles, clipboard, markers, triggers, comments, connections, layers, …)
  - placement 655 (patterns, alignment, geometry, garrison)
  - apply_faction 623/724, place_orbat 211/430
  - attrs 341, tactical_graphics 323, compositions 291, document_index 282, transform 271, cargo 210, plus small files
  - Imports: `MissionDocCore`, `scenario::compile`, `scenario::tactical_graphics`.

**world (12.7k / 5.9k)**

| Module | Lines (prod/test) | Imports outside its own folder |
|---|---|---|
| `terrain/dem` | 1050/1302 | io/containers, io/archives, camera/math, streaming/bridge (progress), streaming/loaders (`DemRawBlock`); `sample/mod.rs` is a shim back into los/terrain |
| `terrain/relief` | 879/225 | contours 400, sea_band 233, host 144 (wasm), hillshade 85 → dem, water (host only), overlay/lanes, lod, world/mesh, frame |
| `terrain/roads` | 1180/581 | styling 456, airfield 258, network 213, cartographic_strip 168 → io/archives, streaming/scheduler, buildings, locations, world/mesh |
| `terrain/water` | 580/416 | vectors 441, loader 99 (wasm) → io, relief::sea_band, streaming bridge/loaders, world/mesh |
| `terrain/satellite` | 2035/0 | streamer 878 (pure), quadtree 849 (wasm), textures 292 (`impl RenderEngine`) → diagnostics/platform (19 uses), streaming bridge/loaders/memory, io, frame |
| `environment/buildings` | 918/552 | prefab 447, footprint 165, buffers 153 (impl), obb 130 → io/archives, streaming/buffers, streaming/scheduler, classify, overlay/lanes, frame |
| `environment/vegetation` | 1294/570 | regions 296, mass 288, loader 222, buffers 189 (impl) → io/archives, io/density, streaming bridge/loaders/scheduler, overlay lanes/lod, world/mesh, satellite |
| `environment/locations` | 1381/743 | route_placement 300, peaks 255, towns 201, loader 196 → io/archives, overlay/symbology labels/text_packing/text_metrics, streaming, dem, roads |
| `environment/classify` | 111 | leaf; `class_code` has 13 users |
| `architecture/blueprint` | 1208/490 | → io/archives, spatial/bvh |
| `architecture/compound` | 941/163 | → bvh |
| `architecture/section` | 787/497 | → bvh, blueprint |
| `mesh` | 170 | hub: → vegetation, relief; imported back by vegetation, relief, roads, water, streaming/loaders |
| `scene` | 110 | `ANCHOR` / `EVERON_BOUNDS` / `world_rect_rel`; 25 importers |

**spatial (5.9k / 3.5k)**
- `bvh` 1101/827: node, sidecar 483, traversal 286, surface. **No imports outside bvh.**
- `indexing` 737/233: cluster 296, point_index 161, world 174, picking 89. Only outside import is `classify::NO_CLASS`.
- `los/terrain` 799: viewshed 321, scheduler 183, overlay 119 (wasm impl), march, sampler → dem, frame, overlay/lanes.
- `los/interior` 912/1098: walker 490, wash 410 → bvh, blueprint, compound, `los/terrain::viewshed` types.
- `los/world` 2368/1329: descriptor 605, coverage 375, tlas 274, residency 246, raycast 199, placed 150, los 137, dda 129 → io/archives (9), bvh, streaming/loaders, streaming/scheduler, blueprint, compound, buildings, los/interior (12).

**streaming (6.9k / 2.1k)**
- `scheduler` 925/670: state 321, viewport 276, chunk_math 129 → buildings (8), classify, indexing, streaming/buffers, streaming/loaders.
- `memory` 937, about 305 of it test → scheduler (1).
- `buffers` 846: glyphs 264, revision 261, packer 177, strips 124 → overlay/symbology/labels (15), lod, roads (9), buildings, vegetation, classify.
- `loaders` 2566/1454:
  - pure part (~1.1k): chunk, chunk_bin, manifest 272, prefab, store, residency
  - wasm part: fetch 136, occluder_loader 300, world_loader 1044
- `bridge` 698: progress, preferences, toggles, statistics 319 (wasm). Imported by 5 world modules for `BootEvent`.
- `host` 922: all wasm.

**overlay (4.9k / 3.6k)**
- `lanes` 482: `LaneRole` with 48 variants. `lod` 127 (zoom constants). `lanes_prefs` 130 (impl). `fire_mission_marks` 272.
- `symbology`:
  - atlas 385 (raster 318, gpu 56 impl)
  - instances 2119/972 (bridge_1/2/3 = 1146 lines of impl, symbols 304, drag 166, lanes 155 impl, patches 94)
  - labels 505/374 (glyph_math 217, importance 176, declutter 97)
  - markers 288, roles 192, links 138/211, text_packing 144, text_metrics 44
  - symbology ↔ world/environment/locations is a cycle (text_packing/text_metrics → peaks/towns/route_placement; locations → labels/text_packing).

**editing (7.2k / 4.1k)**
- `tools`:
  - line_of_sight 1500/1330
  - ruler 586/427
  - selection 477
  - viewshed_scheduler 395/119
  - placement 18 (re-export only)
- `hosted_commands` 1690: 16 files, 20–211 lines each.
- persist 782/1145, commands 566/537, lanes 354, selection_universe 218, routing 152, picking 124, host 102, history 106, batch 36.

**frame 2347/237, diagnostics 3091/0 (all wasm), doll 1284/233, camera 1215/213**
- camera: math 370, ortho 379, orbit 74, grid_reference 238, viewport 130 (impl).
- **io 2032/1488** — archives 1056/521, containers 644/307, density 199/570, pod 112/90. `io/pod → world` is only stale doc links (`io/pod/instance.rs:13,44`), so **io is a real leaf**.

**graphics-engine (4.7k total)**
- device/buffers 235/132 (pool is wasm, readback is pure)
- draw:
  - compose 161, geometry 63, grid 80, instances 71, triangulate 226 (earcutr): all pure
  - cull: oracle 220 pure, compute 408 wasm
  - encode 196, lines 51, polygons 108: wasm
- frame ~630 (ids, damage, camera are pure; packet, present, atlas, batch, buffers are wasm)
- pipeline 556 (wasm, 6 pipelines) + shaders (WGSL)
- text 544/42 (atlas, font, layout, metrics, pack, scale: all pure)
- loop/pump 170/126
- layout 42 (facade only)
- Internal edges: draw/encode → frame; layout → draw, text; frame/atlas → text/pack; pipeline → shaders.

---

## 1. Proposed fine-grained crates (about 55)

`W` = wasm-only. Lines are prod.

**Tier 0: foundations**

| Crate | Source | Lines | Deps | Notes |
|---|---|---|---|---|
| `world_frame` | `world/scene.rs` consts, `EVERON_BOUNDS`, `INITIAL_TARGET`, `APPLY_ANCHOR_*` | ~60 | — | Removes 4 copies of 6400/12800 |
| `lane_roles` | `overlay/lanes.rs` + `overlay/lod.rs` | 609 | ge ids | 19 files use LaneRole, 15 use `lane_id` |
| `land_class` | `world/environment/classify.rs` | 111 | — | 13 users |
| `boot_progress` | `streaming/bridge/{progress,preferences,toggles}` | ~290 | — | Imported by dem, water, satellite, vegetation, locations, loaders |
| `web_platform` (W) | `diagnostics/platform/console.rs`, `diagnostics/timing/gpu.rs::now_ms`, `streaming/loaders/fetch.rs` | ~260 | web-sys, js-sys, gloo-net | Replaces scattered `js_sys::Date::now` |
| `geom3` | `spatial/bvh/node.rs:25-39` sub/cross/dot, `blueprint/geometry.rs`, `compound/transform.rs` `Rigid` | ~500 | — | Consolidate duplicates (see §5) |
| `tbd_formats` | io/archives, containers, density, pod | 2.0k | rkyv, bytemuck, serde | Could split into 4; rkyv is only needed by archives and pod |

**Graphics-engine split**
- `gfx_layout`: instances, geometry, compose, text/*. ~1.0k, pure, bytemuck.
- `gfx_triangulate`: earcutr, 226 lines.
- `gfx_frame_ids`: ids, damage, camera.
- `gfx_pipelines` + shaders (W).
- `gfx_device`: buffers pool/readback, packet, present, encode (W).
- `gfx_cull`: oracle pure, compute W.
- `gfx_loop` (W).

**Mission data (pure, no wasm)**

| Crate | Lines | Deps | Imports from | Imported by |
|---|---|---|---|---|
| `mission_wire_safety` | 406 | serde_json | — | validator, flatten |
| `mission_extensions` | 2838 | serde | — | ast, payload, flatten, store/ops |
| `mission_ast` | 1005 | serde | extensions | payload, flatten |
| `mission_payload` (+kit) | 467 | serde | ast, extensions | validator, flatten, store, editing/persist |
| `mission_validation` | 1668 | serde | wire_safety, payload | flatten, editing/commands |
| `mission_flatten` | 2543 | serde, thiserror | all of the above | API |
| `slot_line` | 69 | — | — | — |
| `ballistics_core` (units, wind, flight_model, catalog) | ~1.4k | libm, serde, thiserror | — | — |
| `ballistics_solver` (solver, crest, dispersion, battery, fuze, fire_mission, comparison, wording) | ~3.4k | libm | core | — |
| `ballistics_calibration` | 1448 | — | core, solver | — |

- Blockers: the four internal ballistics cycles listed in §0 have to be untangled first.
- `agreement_cases` is a test oracle shipped in prod; it should become a dev crate.

**Store**
- `crdt` 682: yrs. Could drop wasm-bindgen with an injected clock.
- `mission_doc` = rows + selection, 6.2k. Must stay **one crate**: 30 files of inherent `impl MissionDocCore`.
- `mission_ops_entity` 2.8k.
- `placement_patterns` 655: pure geometry.
- `faction_apply` (apply_faction + place_orbat + faction_library) ~900.
- `mission_ops_misc` (attrs, transform, compositions, tactical_graphics, cargo, document_index …) ~2.5k.
- Ops are free functions over `&MissionDocCore`, so they split freely.

**World / terrain**
- `terrain_dem` ~800 pure (manifest, sampling, grid, raw, full_resolution, png; deps png) + `terrain_dem_loader` 81 (W).
  - Blockers: `dem/loader.rs:6-13` reaches into streaming; the `dem/sample/mod.rs` shim (21 re-exports of los/terrain) creates a fake dem→los cycle.
- `terrain_relief` 735 + `relief_host` 144 (W).
  - Blocker: cycle `relief/host.rs:19 → water/mesh → relief/sea_band` (`water/mesh.rs:8`). Splitting host out fixes it.
- `water_bodies` 481 + loader (W).
- `road_network` 1180.
  - Blockers: `roads → streaming/scheduler` (2) and `roads → buildings / locations` (1 each). `streaming/buffers → roads` (9) is the correct direction.
- `satellite_streamer` (streamer: header, archive, model, validation, selection) ~560, pure.
- `satellite_quadtree` 849 (W) and `satellite_gpu` (textures) 292 (W).
- `buildings` 765 pure + `buildings_gpu` 153.
  - Blocker: `footprint.rs:6-8` imports `streaming::buffers::revision::{BUILDING_MIN_ZOOM, norm}` and `scheduler::state::WorldResidency`, which is backwards.
- `vegetation` ~1.1k + `vegetation_gpu` 189.
  - Blockers: → `world/mesh` (cycle), → streaming/scheduler.
- `place_names` (locations) 1381.
  - Blocker: cycle with `label_layout`. Put `LabelSpec` / declutter in `label_layout` and move town/peak/route → spec conversion into place_names; `overlay/symbology/text_packing.rs:16-19` and `text_metrics.rs:16` then stop importing world.
- `architecture_blueprint` 1208, `architecture_compound` 941, `architecture_section` 787: very clean, deps only bvh and archives.
- `world/mesh` should be dissolved: each `compose_*` moves to its subject.

**Spatial**
- `bvh` 1101: no imports outside itself; serde for the sidecar.
- `spatial_index` 737.
- `los_terrain` 680 + `viewshed_overlay_gpu` 119 (W).
- `los_interior` 912.
- `los_world` 2368.
  - Blockers: → streaming/loaders (`WorldChunk`) and → streaming/scheduler. Fix by inverting: loaders depend on los_world, or `WorldChunk` moves down into a `world_chunk` crate.

**Streaming**
- `world_chunk` (chunk, chunk_bin, manifest, prefab, store, residency) ~1.1k: flate2, rkyv. Pure.
- `chunk_scheduler` 925: depends on buildings (8) only for OBB / prefab info. A `ChunkContent` trait would remove that.
- `memory_budget` ~630.
- `chunk_buffers` 846.
- `world_loader` (W) ~1.5k.
- `stream_host` 922 (W).
- Main blocker: `scheduler → buffers → scheduler` (`streaming/scheduler → streaming/buffers` ×3, back edge ×10) plus `scheduler → loaders` ×5 / `loaders → scheduler` ×8. This is a real three-way cycle. `chunk_math` and `state` need to move down.

**Overlay**
- `symbology_roles` 192, `symbology_atlas_raster` 318, `symbology_markers` 288, `squad_links` 138.
- `label_layout` (labels + text_packing + text_metrics) ~690.
- `slot_symbology` (symbols, drag, patches) ~560 + `slot_symbology_gpu` (bridge_1..3, lanes) ~1.3k (W).
- `fire_mission_marks` 272.

**Rendering**
- `map_renderer_core` (frame) 2.3k (W).
- `render_diagnostics` 3.1k (W, dev-only feature).
- `doll_scene` (scene + interaction) 558, pure.
- `doll_renderer` 713 (W).
- `camera` 1085 pure (+ `viewport.rs` → facade).

**Editing:** see §4.

---

## 2. RenderEngine field-access matrix

`RenderEngine` has 76 fields (`frame/engine.rs:82`). They fall into groups:

| Group | Count | Fields |
|---|---|---|
| GPU core | 6 | device, queue, surface, config, backend_kind, adapter_max… |
| Shared layouts / pipelines / samplers | 20 | |
| Shared buffers | 4 | uniform_buf, bind_group, unit_quad_buf, calibration_buf |
| Camera | 1 | |
| Subject state | 9 | glyph_atlas, text_atlas, slot_atlas, slot_bridge, tex_lanes, pending, icon_cull, tree_icons_20, compute_cull_trees |
| Retained frame | 9 | batches, frame_pipelines, frame_bind_groups, damage, submitted, clear_color, lane_pool, staging, timer |
| **Stats counters** | **27** | |

Out-of-frame impl files (number of uses in parentheses; "calls" = methods on `self` provided by frame):

**A. Lane-sink layers (narrow)**

| File | Fields used | Frame methods called |
|---|---|---|
| `world/environment/buildings/buffers.rs` | device, world_chunks_drawn, strip_lane_uploads, building_uploads | upsert_lane, remove_lane |
| `world/environment/vegetation/buffers.rs` | device, queue, tex_bind_group_layout, density_sampler, batches, damage, forest_* (7 counters) | upsert_textured_lane, remove_lane |
| `world/terrain/satellite/textures.rs` | device, queue, tex_bind_group_layout, sampler, pending | upsert_textured_lane, remove_lane |
| `spatial/los/terrain/overlay.rs` | device, queue, tex_bind_group_layout, density_sampler | upsert_textured_lane, remove_lane |
| `overlay/lanes_prefs.rs` | device, queue, batches, clear_color, damage | upsert_lane, remove_lane |
| `overlay/symbology/atlas/gpu.rs` | device, queue, icon_bind_group_layout, icon_sampler, glyph_atlas | — |
| `overlay/symbology/instances/lanes.rs` | device, queue, icon_lane_uploads, damage, icon_cull, tree_icons_20 | gpu_cull_enabled, clear_cull_lane, upsert_lane, remove_lane |

**B. Stateful subject that already has its own struct but lives on the engine**
- `overlay/symbology/instances/bridge_1.rs`: slot_bridge (44), camera, damage.
- `bridge_2.rs`: slot_bridge (42), slot_atlas (4), device, queue, icon_bind_group_layout, icon_sampler, uniform_bytes_last_frame, damage.
- `bridge_3.rs`: slot_bridge (11), slot_atlas, batches, damage, icon_cull, lane_pool (2). Calls ensure_text_atlas, upsert_pooled_icon_lane, comments_bind, vehicles_bind.
- `camera/viewport.rs`: camera (9), slot_bridge (5), damage (5), device, surface, config (resize). Calls rematerialize_slot_lane, sync_slot_zoom_uniform, feed_cluster_markers.
- `SlotGpuBridge` (`bridge_1.rs:20`, 17 fields) and `SlotAtlasGpu` (`:74`) already exist; only the methods are misplaced.

**C. Diagnostics (read-only to the GPU context)**
- 8 readbacks each use device, queue, backend_kind, shader, plus 1–4 of: pipeline_layout, bind_group_layout, tex/icon/text layouts, unit_quad_buf, sampler.
- `readback/scene.rs` uses about 20 fields.
- `bench/frame_2.rs` reads 30 fields, nearly all stats.
- `bench/frame_1.rs` writes staging, stress_instances, gen_ms, upload_ms.
- `readback/doll.rs` puts `impl DollEngine` in diagnostics, reading device, queue, shader, pipeline_layout, bind_group_layout, cube_vbuf/ibuf/index_count, cyl_vbuf/ibuf/index_count.

**Outside any impl:** `world/terrain/satellite/quadtree/selection.rs` reads `adapter_max_texture_dimension_2d` directly. 24 files use the RenderEngine API through `EngineHandle` (`frame/mod.rs`), including streaming/host/*, world_loader/*, satellite/quadtree/*, relief/host, and vegetation/locations loaders.

**Verdict on a layer-trait design**
- Drawing is already data-driven. `encode.rs` walks `batches: Vec<DrawBatch>` keyed by `LaneId`, with `PipelineId`/`BindGroupId` registries (`frame/bindings.rs:20-110`). Layers never draw themselves.
- So a `Vec<Box<dyn RenderLayer>>` with per-frame `draw()` is the wrong shape. What fits:
  - **`LaneSink` trait** in renderer-core: `upsert_lane(role, DrawBatch)`, `remove_lane`, `upsert_textured_lane(role, bind_group)`, `register_bind_group`, `mark_damage`, `ctx() -> &GpuCtx {device, queue, layouts, samplers}`, `stats_mut()`. Group A uses only this.
  - **Owned layer structs as typed fields:** `BuildingLayerGpu`, `ForestLayerGpu` (the 7 forest counters plus the density texture), `SatelliteTexLayers` (tex_lanes, pending), `ViewshedOverlayGpu`, `GlyphAtlasGpu`, `TextLayerGpu` (text_atlas plus 3 label counters), `SlotSymbologyGpu` (slot_bridge, slot_atlas), `IconCullGpu` (icon_cull, tree_icons_20, compute_cull_trees, icon_pipeline_storage32). Each takes `&mut dyn LaneSink`.
  - A small **`FrameHook`** (`on_camera_changed`, `pre_encode`) is needed for only three of these: slot zoom/drag uniforms, icon compute cull, and satellite pending commits.
  - Stats should become a `RenderStats` struct. The **27 counters** are the biggest fake coupling.

**Blockers**
1. **Inherent `#[wasm_bindgen] impl RenderEngine` can't be split across crates.** 136 of the 145 out-of-frame impl blocks are exported (`frame/upload/text.rs` 7/8, `frame/lifecycle.rs` 11/15, `diagnostics/readback/scene.rs` 1/5, `diagnostics/bench/frame_2.rs` 1/2). A thin `map_engine_wasm` facade crate has to own the JS methods and delegate.
2. `LaneRole` is a closed 48-variant enum that knows every subject (`overlay/lanes.rs:16`), and lane order is centralised (`:163`). It stays in the tier-0 `lane_roles` crate, or becomes registration-based.
3. `tex_bind_id(lane)` (`bindings.rs:88`) ties bind-group ids to lane ids.
4. `frame/boot.rs` builds every subject's state, so construction needs per-layer `new(&GpuCtx)`.

**DollEngine** (`doll/renderer/lifecycle_1.rs:21`, 27 fields)
- All impls live in doll/renderer, except `diagnostics/readback/doll.rs:73`.
- It **duplicates the device/surface bootstrap**: `lifecycle_1.rs:123-166` vs `frame/boot.rs:57-112`. It also borrows `frame::boot::instance_descriptor` (`lifecycle_1.rs:9`).
- Fix: a shared `GpuSurfaceCtx::create(canvas)` in renderer-core. `DollEngine` then holds `GpuSurfaceCtx` plus `DollMeshes` plus the interaction state.

---

## 3. Cross-cutting foundations (number of users)

| Item | Location | Users |
|---|---|---|
| `world::scene` (ANCHOR, `world_rect_rel`, calibration) | `world/scene.rs:34,108` | 25 files (frame 8, diagnostics 10, overlay 4, world 3); ANCHOR alone in 22 |
| `LaneRole` / `lane_id` | `overlay/lanes.rs:16,230` | 19 / 15 files |
| `overlay::lod` zoom constants | `overlay/lod.rs` | streaming scheduler/buffers/bridge, vegetation, relief |
| `class_code` / `NO_CLASS` | `world/environment/classify.rs` | 13 files |
| console macros `graphics_{log,warn,error}` | `diagnostics/platform/console.rs:6-12` | 7 files (satellite 19 uses) |
| clock `now_ms` | `diagnostics/timing/gpu.rs:11` | duplicated, see §5 |
| fetch helpers | `streaming/loaders/fetch.rs:13,22,77,107` | 17 files; dem uses `gloo_net` directly |
| `BootEvent` / progress | `streaming/bridge/progress` | 5 world modules + loaders |
| POD instances `QuadInstance` / `IconInstance` / `BuildingInstance` / `LineVertex` | ge `draw/instances`, `draw/geometry` | 17 files, imported via two paths (`layout::` ×8 vs `draw::geometry::` ×4) |
| `io::archives` | | 10 modules |
| `spatial::bvh` | | 4 modules |
| `MissionDocCore` | | 68 files (data 55, editing 13) |
| error enums | | 25 local enums, no shared one (`ballistics` has 13). Keep them per crate; only `BinaryError` (`io/archives/codec.rs:17`) is cross-cutting |

No shared Bounds/Aabb type exists. Bounds are raw `([f64;3],[f64;3])` everywhere, except `Bounds3` (`spatial/los/world/descriptor/bounds.rs:11`). That is a `geom3` candidate.

---

## 4. Editing: tool separability

- **No tool trait or framework.** The shared "framework" is:
  - `editing/host.rs:19-100`: a thread-local `DocHandle` / `SelectionHandle` with `with_doc` / `with_doc_mut`.
  - `history/` (undo/redo, `after_local_edit`), `batch.rs:14`, `routing.rs:14,72` (`RouteTarget`), `selection_universe.rs`.
  - Each tool keeps its own `thread_local!` registry: `ruler/host_registry.rs:11`, `line_of_sight/host_registry.rs:24`, `viewshed_scheduler/host.rs:39,65`.
- **ruler** (586/427): **no imports outside ruler at all.** It's an immediate crate.
- **line_of_sight ↔ viewshed_scheduler**: mutual cycle.
  - LOS → `viewshed_scheduler::submit_terrain`.
  - Scheduler → `line_of_sight::{viewshed_texture, terrain_verdict, terrain_survey, host_registry}`.
  - Merge them into one crate, `los_tool` ~1.9k → spatial/los/{terrain, interior, world}, dem.
- **selection** (477) → `editing::picking`, `hosted_commands::{selection_transform, placed_vehicles}` (a tool depending on commands is backwards), plus camera/ortho, spatial/indexing, frame (`EngineHandle` in `gesture.rs`).
- **placement** is an 18-line re-export.
- **hosted_commands**: 16 files, each a thin `with_doc` + `entity_ops::*` + `after_local_edit` wrapper. They're independent of each other, so they could be per-domain crates, or one ~1.7k crate on top of `editing_host`.
- **persist** (782) → host, store, compile. Separable.
- **commands** (566) → `camera/grid_reference`, validate, ops. Separable.
- Suggested `editing_host` crate: host + history + batch + routing + selection_universe + picking (~740).

---

## 5. Dead code, duplication, consolidation

1. **Glyph packing duplicated across the two crates.** `overlay/symbology/labels/glyph_math.rs:73,101,121,133` (`size_with_min_px`, `pack_rgba_u32`, private `wrap_deg_180`, `yaw_to_snorm16`, `pack_icon_instance`) copy `ge/text/pack.rs:8,16,48,59` and `ge/text/scale.rs:19` byte for byte.
2. **Color normalisation in three places.** `norm([u8;4])` at `streaming/buffers/revision.rs:15` and `ge/draw/geometry.rs:22`, plus `ge/draw/compose.rs:37` `u8_rgba_to_f32`.
3. **Clock in five forms.** `diagnostics/timing/gpu.rs:11` and `editing/tools/viewshed_scheduler/host.rs:89` (two `now_ms`), plus raw `js_sys::Date::now` at `streaming/host/viewport.rs:80` and `streaming/loaders/world_loader/ingest.rs:59,92`, plus the crdt clock (`data/store/crdt/undo_groups/clocks.rs`).
4. **RNGs.** `SplitMix64` is defined twice: `data/scenario/ballistics/agreement_cases.rs:81` and `data/store/operations/placement/geometry.rs`. An inline LCG appears at `data/store/rows/slot_edits.rs:31,35`.
5. **Map centre constant in four places.** `world/scene.rs:34` `ANCHOR`, `world/scene.rs` `INITIAL_TARGET`, `data/store/operations/apply_faction/library.rs:11,14` `APPLY_ANCHOR_X/Y`, and a literal at `diagnostics/probes/runner.rs:87`.
6. **Three-layer forwarding chains.**
   - Picking: `data/store/selection.rs:27,34,82` → `editing/picking.rs:27,39,74` → `editing/tools/selection/pick.rs:77,88` / `marquee.rs:33`. The last layer is a pure pass-through, marked `#[allow(dead_code)]` (`pick.rs:76`, `picking.rs:26`).
   - Triggers / comments go rows → ops → hosted the same way (`rows/triggers.rs:63` → `operations/entity/triggers.rs:15` → `hosted_commands/map_triggers.rs:46`). Consider letting hosted commands call `MissionDocCore` directly, or dropping the ops wrapper where it adds no validation.
7. **Small geometry duplicates.** `point_at` at `spatial/los/interior/walker.rs:58` and `world/architecture/blueprint/geometry.rs:7`. There are three `evaluate_los`: `blueprint/attribution_1.rs:119`, `spatial/los/world/los.rs:24`, `spatial/los/interior/walker.rs:420`. There are two BVH builders: `spatial/bvh/traversal.rs:50` (triangles) and `spatial/los/world/tlas.rs:51` (AABBs). A shared split/build core is possible.
8. **Reference algorithm shipped in prod.** `spatial/los/world/dda.rs:84` `cells_on_segment_reference` sits beside the real one at `:8`.
9. **Hand-rolled SHA-256** at `data/scenario/ballistics/calibration/sha256_digest.rs:32`. Use the `sha2` crate.
10. **Duplicated wgpu bootstrap**: `frame/boot.rs:57-112` vs `doll/renderer/lifecycle_1.rs:123-166`.
11. **Re-export shim `mod.rs` files.** They exist to host tests at old paths and create fake cycles:
    - `io/archives/models/mod.rs` (88 re-exports)
    - `overlay/symbology/instances/slots/mod.rs` (70)
    - `world/environment/locations/routes/mod.rs` (35)
    - `spatial/los/world/mod.rs` (26)
    - `world/terrain/dem/sample/mod.rs` (21, re-exports **los/terrain**, so a reverse edge)
    - `world/architecture/blueprint/model` (20), `doll/scene/model` (18), `spatial/bvh/tree` (16), `world/architecture/compound/scene` (14), `io/containers/headers` (14), `spatial/los/world/trace` (11), `streaming/scheduler/residency` (8), `camera/ortho/camera` (5)
    - Facades: `world/mesh.rs`, `overlay/symbology/text_metrics.rs`, ge `layout/mod.rs`
12. **Mechanical file splits to merge:** `bridge_1/2/3`, `lifecycle_1/2`, `coverage_1/2`, `frame_1/2`, `attribution_1/2`, `cases_1`.
13. **Four unrelated "residency" modules:** `streaming/memory/residency.rs`, `streaming/scheduler/residency/` (43 lines prod, 1.3k tests), `streaming/loaders/residency.rs`, `spatial/los/world/residency.rs`. There are also two viewshed schedulers: `spatial/los/terrain/scheduler.rs` and `editing/tools/viewshed_scheduler`. Rename or merge.
14. **Stale doc links** (dead paths): `io/pod/instance.rs:13,44`; `world/environment/locations/towns.rs:145,162`; `spatial/los/terrain/viewshed.rs:109` (`crate::building_viewshed`); `ge/draw/cull/oracle.rs:11` (`crate::scene::IconInstance`).
15. **Feature-gate leak.** The `world` feature pulls in `website-graphics-engine` only because `streaming/buffers` and the native frontend need POD/text-packing helpers (Cargo.toml comment). Splitting `gfx_layout` (pure) from the wgpu half removes the reason for that gate.
16. **Test folders that don't match the `tests` path convention** (`t938_6`, `t935_10`, `t151_11_3_tests`, `t152_3_tests`). Normalise them so per-crate test/prod accounting is accurate.
