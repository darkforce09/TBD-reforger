**Status:** live

# Crate catalogue

Every crate the program creates, grouped by category: where its code comes from, what it depends
on inside the workspace, and what it fixes. The tree is in
[target_file_tree.md](/documentation/restructure/target_file_tree.md); the tier law that checks
the dependency columns is in [laws_and_gates.md](/documentation/restructure/laws_and_gates.md).

Abbreviations in code spans:

| Prefix | Path |
|---|---|
| `me` | the map engine source folder: legacy/map_engine/src/ until S8, which dissolved it into crates and deleted it |
| `ge` | the graphics engine source folder: legacy/graphics_engine/src/ until S8, which dissolved it into crates and deleted it |
| `api` | `apps/api/src/` |
| `fe` | `apps/frontend/src/` |
| `xt` | `tools/xtask/src/` |
| `dt` | `tools/developer_tools/src/` |
| `te` | the ticket tool source folder: tools/ticket_engine/src/ until S11, now split into `tools/tickets/` |
| `vc` | the verification core source folder: tools/verification_core/src/ until S4b, now split into `tools/foundation/` |

A bare module path such as `world/scene.rs` is relative to the map engine's source folder. "W" marks a
wasm-only crate. Tiers follow tier = 1 + the highest dependency tier, and the crate-tiers law
checks them.

## Built so far

| Stage | Crate | Folder | Tier |
|---|---|---|---|
| S2 | http_url_guard | `crates/foundation/http_url_guard/` | 0 |
| S2 | offline_cache_policy | `crates/contracts/offline_cache_policy/` | 0 |
| S4a | newtype_ids | `crates/foundation/newtype_ids/` | 0 |
| S4a | time_source | `crates/foundation/time_source/` | 0 |
| S4a | deterministic_random | `crates/foundation/deterministic_random/` | 0 |
| S4a | content_digest | `crates/foundation/content_digest/` | 0 |
| S4a | browser_platform (W) | `crates/foundation/browser_platform/` | 0 |
| S4a | geometry_primitives | `crates/geometry/geometry_primitives/` | 0 |
| S4a | map_coordinates | `crates/geometry/map_coordinates/` | 0 |
| S4a | camera_math | `crates/geometry/camera_math/` | 1 |
| S4a | world_file_formats | `crates/world_formats/world_file_formats/` | 1 |
| S4a | render_primitives | `crates/graphics/render_primitives/` | 0 |
| S4b | fleet_wire_contract | `crates/contracts/fleet_wire_contract/` | 0 |
| S4b | contract_schema_types | `crates/contracts/contract_schema_types/` | 0 |
| S4b | verification_core | `tools/foundation/verification_core/` | 0 |
| S4b | process_runner | `tools/foundation/process_runner/` | 1 |
| S4b | repository_laws | `tools/foundation/repository_laws/` | 1 |
| S4b | repository_layout | `tools/foundation/repository_layout/` | 0 |
| S6 | spatial_indexes | `crates/geometry/spatial_indexes/` | 1 |
| S6 | prefab_catalog | `crates/world_formats/prefab_catalog/` | 2 |
| S6 | world_chunks | `crates/world_formats/world_chunks/` | 3 |
| S6 | world_store | `crates/world_formats/world_store/` | 5 |
| S6 | terrain_elevation | `crates/terrain/terrain_elevation/` | 2 |
| S6 | terrain_relief | `crates/terrain/terrain_relief/` | 3 |
| S6 | satellite_imagery | `crates/terrain/satellite_imagery/` | 2 |
| S6 | water_bodies | `crates/terrain/water_bodies/` | 4 |
| S6 | road_network | `crates/terrain/road_network/` | 3 |
| S6 | building_interiors | `crates/world_objects/building_interiors/` | 2 |
| S6 | vegetation | `crates/world_objects/vegetation/` | 4 |
| S6 | place_names | `crates/world_objects/place_names/` | 4 |
| S6 | terrain_line_of_sight | `crates/line_of_sight/terrain_line_of_sight/` | 3 |
| S6 | interior_line_of_sight | `crates/line_of_sight/interior_line_of_sight/` | 4 |
| S6 | world_line_of_sight | `crates/line_of_sight/world_line_of_sight/` | 5 |
| S6 | map_draw_lanes | `crates/map_overlay/map_draw_lanes/` | 1 |
| S6 | label_layout | `crates/map_overlay/label_layout/` | 1 |
| S6 | unit_symbology | `crates/map_overlay/unit_symbology/` | 2 |
| S6 | overlay_instances | `crates/map_overlay/overlay_instances/` | 3 |
| S5 | mission_wire_safety | `crates/mission/mission_wire_safety/` | 0 |
| S5 | mission_model | `crates/mission/mission_model/` | 2 |
| S5 | mission_payload | `crates/mission/mission_payload/` | 3 |
| S5 | mission_validation | `crates/mission/mission_validation/` | 4 |
| S5 | mission_compiler | `crates/mission/mission_compiler/` | 5 |
| S5 | formation_geometry | `crates/mission/formation_geometry/` | 1 |
| S5 | mission_crdt | `crates/mission/mission_crdt/` | 1 |
| S5 | mission_document | `crates/mission/mission_document/` | 5 |
| S5 | mission_operations | `crates/mission/mission_operations/` | 6 |
| S5 | ballistics_model | `crates/ballistics/ballistics_model/` | 1 |
| S5 | ballistics_solver | `crates/ballistics/ballistics_solver/` | 2 |
| S5 | fire_mission_planning | `crates/ballistics/fire_mission_planning/` | 3 |
| S5 | ballistics_calibration | `crates/ballistics/ballistics_calibration/` | 3 |
| S5 | ballistics_agreement_cases | `crates/ballistics/ballistics_agreement_cases/` | 4 |
| S11a | deploy_settings | `tools/foundation/deploy_settings/` | 2 |
| S11a | tool_test_support | `tools/foundation/tool_test_support/` | 1 |
| S11a | ticket_model | `tools/tickets/ticket_model/` | 2 |
| S11a | ticket_metrics | `tools/tickets/ticket_metrics/` | 3 |
| S11a | ticket_wave_lock | `tools/tickets/ticket_wave_lock/` | 3 |
| S11a | ticket_registry | `tools/tickets/ticket_registry/` | 4 |
| S11a | ticketboard_model | `tools/tickets/ticketboard_model/` | 4 |
| S11a | repository_checks | `tools/checks/repository_checks/` | 2 |
| S11a | mod_script_checks | `tools/checks/mod_script_checks/` | 2 |
| S11a | schema_tooling | `tools/commands/schema_tooling/` | 5 |
| S11a | ballistics_oracle_tooling | `tools/commands/ballistics_oracle_tooling/` | 1 |
| S11a | api_readiness_checks | `tools/commands/api_readiness_checks/` | 3 |
| S11a | workstation_setup | `tools/commands/workstation_setup/` | 3 |
| S11a | enfusion_mcp | `tools/commands/enfusion_mcp/` | 2 |
| S11a | enfusion_mcp_broker | `tools/enfusion/enfusion_mcp_broker/` | 3 |
| S11a | repository_relocation | `tools/commands/repository_relocation/` | 3 |
| S11a | database_operations | `tools/commands/database_operations/` | 4 |
| S11a | deployment | `tools/commands/deployment/` | 5 |
| S11a | enfusion_pak | `tools/enfusion/enfusion_pak/` | 0 |
| S11a | enfusion_script_index | `tools/enfusion/enfusion_script_index/` | 2 |
| S11a | chrome_devtools_protocol | `tools/browser_testing/chrome_devtools_protocol/` | 1 |
| S11a | browser_gate_suites | `tools/browser_testing/browser_gate_suites/` | 5 |
| S11a | staging_load_plan | `tools/staging/staging_load_plan/` | 1 |
| S11a | staging_load_generator | `tools/staging/staging_load_generator/` | 2 |
| S11a | acknowledgement_dropping_relay | `tools/staging/acknowledgement_dropping_relay/` | 1 |
| S11a | staging_procedures | `tools/commands/staging_procedures/` | 6 |
| S11a | remote_debugging | `tools/commands/remote_debugging/` | 6 |
| S11a | documentation_checks | `tools/checks/documentation_checks/` | 2 |
| S11a | ci_task_catalog | `tools/commands/ci_task_catalog/` | 8 |
| S11a | platform_execution | `tools/commands/platform_execution/` | 9 |
| S11a | mod_operations | `tools/commands/mod_operations/` | 10 |
| S7 | orbat_slot_ids | `crates/foundation/orbat_slot_ids/` | 1 |
| S7 | chunk_scheduler | `crates/streaming/chunk_scheduler/` | 4 |
| S7 | chunk_draw_buffers | `crates/streaming/chunk_draw_buffers/` | 5 |
| S7 | mission_editing_session | `crates/mission_editing/mission_editing_session/` | 6 |
| S7 | mission_editing_commands | `crates/mission_editing/mission_editing_commands/` | 7 |
| S7 | mission_persistence | `crates/mission_editing/mission_persistence/` | 7 |
| S7 | map_editing_tools | `crates/mission_editing/map_editing_tools/` | 7 |
| S11b | blueprint_compiler | `tools/map_assets/blueprint_compiler/` | 6 |
| S11b | world_export_pipeline | `tools/map_assets/world_export_pipeline/` | 6 |
| S11b | map_asset_verification | `tools/map_assets/map_asset_verification/` | 7 |
| S11b | map_raster_pipeline | `tools/map_assets/map_raster_pipeline/` | 7 |
| S9 | api_mission_vocabulary | `crates/api/api_mission_vocabulary/` | 0 |
| S9 | api_identifiers | `crates/api/api_identifiers/` | 1 |
| S9 | api_foundation | `crates/api/api_foundation/` | 1 |
| S9 | api_property_evidence | `crates/api/api_property_evidence/` | 1 |
| S9 | api_failpoints | `crates/api/api_failpoints/` | 2 |
| S9 | api_configuration | `crates/api/api_configuration/` | 2 |
| S9 | api_audit_log | `crates/api/api_audit_log/` | 2 |
| S9 | api_equipment_datasets | `crates/api/api_equipment_datasets/` | 2 |
| S9 | api_database | `crates/api/api_database/` | 3 |
| S9 | api_http_layer | `crates/api/api_http_layer/` | 3 |
| S9 | api_member_activity | `crates/api/api_member_activity/` | 3 |
| S9 | api_discord | `crates/api/api_discord/` | 4 |
| S9 | api_caller_identity | `crates/api/api_caller_identity/` | 4 |
| S9 | api_state | `crates/api/api_state/` | 5 |
| S9 | api_community_content | `crates/api/api_community_content/` | 6 |
| S9 | api_identity_and_access | `crates/api/api_identity_and_access/` | 6 |
| S9 | api_administration | `crates/api/api_administration/` | 7 |
| S9 | api_server_infrastructure | `crates/api/api_server_infrastructure/` | 7 |
| S9 | api_match_telemetry | `crates/api/api_match_telemetry/` | 8 |
| S9 | api_missions | `crates/api/api_missions/` | 8 |
| S9 | api_operations | `crates/api/api_operations/` | 9 |
| S9 | api_command_center | `crates/api/api_command_center/` | 10 |
| S9 | api_background_workers | `crates/api/api_background_workers/` | 10 |
| S9 | staging_fixtures | `tools/staging/staging_fixtures/` | 10 |
| S8 | gpu_device | `crates/graphics/gpu_device/` | 0 |
| S8 | gpu_frame | `crates/graphics/gpu_frame/` | 1 |
| S8 | renderer_core | `crates/graphics/renderer_core/` | 2 |
| S8 | map_streaming_model | `crates/streaming/map_streaming_model/` | 1 |
| S8 | map_asset_loading | `crates/streaming/map_asset_loading/` | 6 |
| S8 | map_streaming_host | `crates/streaming/map_streaming_host/` | 7 |
| S8 | symbology_layers_gpu | `crates/map_rendering/symbology_layers_gpu/` | 4 |
| S8 | world_layers_gpu | `crates/map_rendering/world_layers_gpu/` | 3 |
| S8 | map_renderer | `crates/map_rendering/map_renderer/` | 5 |
| S8 | map_render_diagnostics | `crates/map_rendering/map_render_diagnostics/` | 6 |
| S8 | paper_doll_scene | `crates/paper_doll/paper_doll_scene/` | 2 |
| S8 | paper_doll_renderer | `crates/paper_doll/paper_doll_renderer/` | 3 |

Every other crate in this catalogue is still planned; its From column names the code it will take.

## crates/foundation, crates/contracts (T0)

| Crate | From | Fixes |
|---|---|---|
| newtype_ids | new | `string_id!`/`integer_id!`/`uuid_id!` macros: serde-transparent, `Borrow<str>`, an optional `sqlx,` macro arm that expands `#[sqlx(transparent)]` in the calling crate (the crate has no sqlx dependency) |
| orbat_slot_ids | new (tier 1, on newtype_ids): the ORBAT slot ids `SlotUid` and `SlotId`, whose declarations in mission_model and unit_symbology were deleted | One declaration of each slot id; serde-transparent, so every serialized byte stays equal |
| time_source | the 5 clocks (`me diagnostics/timing/gpu.rs:11`, `editing/tools/viewshed_scheduler/host.rs:89`, `streaming/host/viewport.rs:80`, `world_loader/ingest.rs:59,92`, `data/store/crdt/undo_groups/clocks.rs`), `dt timestamp_formatting.rs`, `te timestamp.rs` | One `Clock` trait (system/browser/fixed) and UTC formatting |
| deterministic_random | SplitMix64 (`store/operations/placement/geometry.rs`, `ballistics/agreement_cases.rs:81`), the LCG (`rows/slot_edits.rs:31`) | One generator |
| content_digest | `dt content_digest.rs`, 11 ad-hoc SHA helpers, hand-rolled `ballistics/calibration/sha256_digest.rs:32` | sha2 0.11 only |
| http_url_guard | built in S2: the API's predicate became `http_url.rs`, the frontend's copy was deleted, and the case table of the dissolved website shared folder became `crates/foundation/http_url_guard/src/cases.rs` | One predicate; the `cases` module (behind the `test_fixtures` feature) replaces the 10 `include!` sites |
| browser_platform (W) | `me diagnostics/platform/console.rs`, `streaming/loaders/fetch.rs`, DEM `gloo_net` use | Breaks the console-macro cycles |
| fleet_wire_contract | `api server_infrastructure/models/fleet_command.rs:40-262`, agent `ledger_client/ledger_messages.rs:13-56`, both `secret_files.rs` | 3 of the 4 copies merged; the frontend DTO stays separate (D7) |
| contract_schema_types | the 266 typify files in `api/*/models/generated` and `missions/contract/generated` | Generated (exempt from the ID rule); codegen emits the anatomy files |
| offline_cache_policy | built in S2 from the service worker's library: `cache_names`, `network_fallback`, `offline_pack`, `request_classification`, plus a new `TerrainId` | `range_slicing` stays in the worker, now a binary-only crate |

## crates/mission, crates/ballistics

| Crate | T | From | Deps |
|---|---|---|---|
| mission_wire_safety | 0 | `scenario/validation/wire_safety` | — |
| mission_model | 2 | `scenario/{ast,extensions,slot_line}` | newtype_ids, orbat_slot_ids |
| mission_payload | 3 | `compiler/{payload,kit}` | model |
| mission_validation | 4 | `validation/validator` | payload, wire_safety, newtype_ids |
| mission_compiler | 5 | `compiler/flatten`, `ast/authoring.rs` as a private module; `COMPILER_PACKAGE_VERSION` pinned as a literal | model, payload, validation, wire_safety |
| formation_geometry | 1 | `store/operations/placement` | deterministic_random |
| mission_crdt | 1 | `store/crdt` (no `js_sys`; clock injected) | time_source |
| mission_document | 5 | `store/{rows,selection}` plus `crdt/id_arrays/mission_doc_tests` | crdt, model, validation, newtype_ids, time_source; dev: payload, compiler (round-trip and former RULE4 tests) |
| mission_operations | 6 | all of `store/operations` (entity + the rest + `assets, cargo_rules, environment, projections, reassign, rotation, rows, slot_ids, zones`) | document, crdt, validation, payload, model, formation_geometry, map_coordinates |
| ballistics_model | 1 | `ballistics/{angular_units,catalog,wind,flight_model}` | newtype_ids |
| ballistics_solver | 2 | `solver, crest_clearance, dispersion` | model; dev: deterministic_random |
| fire_mission_planning | 3 | `fire_mission, fire_mission_comparison, fuze, battery, solution_wording` | model, solver |
| ballistics_calibration | 3 | `calibration` | model, solver, content_digest, newtype_ids |
| ballistics_agreement_cases | 4 | `agreement_cases` (a test oracle moved out of prod) | model, solver, planning, deterministic_random, newtype_ids |

## Engine CPU crates (geometry, world_formats, terrain, world_objects, line_of_sight, map_overlay, streaming, mission_editing)

| Crate | From | Deps (workspace) |
|---|---|---|
| geometry_primitives | `bvh/node.rs:25-39` vector ops, `blueprint/geometry.rs`, compound `Rigid`, `Bounds3`, both `point_at` | — |
| map_coordinates | `world/scene.rs` consts + `world_rect_rel`, `APPLY_ANCHOR_X/Y` (`apply_faction/library.rs:11,14`), `probes/runner.rs:87`, `chunk_math.rs`, `camera/math/shaping.rs`, `grid_reference.rs` | — |
| camera_math | `camera/{math,ortho,orbit}` | map_coordinates |
| spatial_indexes | `spatial/bvh` (the triangle tree, sidecar, surface kinds, the segment-box window from the interior walker) and `indexing/{cluster,point_index,picking}`; one flat-tree build core that the world TLAS also uses; dev feature `test_fixtures` | geometry_primitives |
| world_file_formats | `io/*` (shims `archives/models` and `containers/headers` deleted); typed archive ids | newtype_ids |
| prefab_catalog | `buildings/{prefab,obb}`, `environment/classify`, `loaders/prefab`, and the payload decoding (`WorldError`, `bytes_to_json`) that sat in the world store | world_file_formats |
| world_chunks | `loaders/{chunk,chunk_bin,manifest}` with `ChunkId`; the chunk ingest into the residency (`loaders/residency.rs`) went to the scheduler for chunk_scheduler | prefab_catalog, world_file_formats, map_coordinates, newtype_ids |
| world_store | `loaders/store.rs` (world store over chunks, roads, vegetation regions) | world_chunks, vegetation, road_network, prefab_catalog, world_file_formats, map_coordinates |
| terrain_elevation | `terrain/dem` without the loader (S4 deleted the `sample/` shim) | world_file_formats, map_coordinates |
| terrain_relief | `relief/{contours,sea_band,hillshade}` (host moves to the GPU crate) | terrain_elevation, map_coordinates |
| water_bodies | `water/{vectors,mesh}` | terrain_relief, render_primitives, world_file_formats |
| road_network | `terrain/roads`, plus the road-class codec (`ROAD_CLASSES`, `road_class_code`, `road_class_name`) moved in from `route_placement` | world_file_formats, prefab_catalog, terrain_elevation, render_primitives, map_coordinates |
| satellite_imagery | `satellite/streamer` | world_file_formats |
| vegetation | `vegetation/{regions,mass,canopy,density}` | world_file_formats, prefab_catalog, map_draw_lanes, world_chunks, map_coordinates |
| place_names | `environment/locations` without the loader, with the location label packers from `text_packing`; owns town/peak/route → `LabelSpec`; `routes/` shim deleted | label_layout, terrain_elevation, road_network, render_primitives, world_file_formats, newtype_ids |
| building_interiors | `architecture/{blueprint,compound,section}`; the section index on spatial_indexes' build core; the blueprint's 2D `annotate_sight_line` (dev feature `test_fixtures`) | spatial_indexes, world_file_formats, geometry_primitives, newtype_ids |
| terrain_line_of_sight | `los/terrain` without `overlay.rs` | terrain_elevation |
| interior_line_of_sight | `los/interior`, with the one 3D `evaluate_los` over `SightLineScene` (the world crate depends on it) | building_interiors, terrain_line_of_sight, spatial_indexes, geometry_primitives |
| world_line_of_sight | `los/world`; reference `dda.rs:84` moved to tests | world_chunks, interior_line_of_sight, spatial_indexes, building_interiors, prefab_catalog, map_coordinates, geometry_primitives, world_file_formats |
| map_draw_lanes | `overlay/{lanes,lod}`, plus `building_visible` on `BUILDING_FOOTPRINT_MIN_ZOOM` (the duplicate `BUILDING_MIN_ZOOM` deleted) and `px_to_m_at_zoom` | render_primitives |
| label_layout | `symbology/{labels,text_packing}` (generic packing; the location packers went to place_names); the glyph-packing copies in `glyph_math` deleted; `LabelId`, `LocationId` | render_primitives, newtype_ids |
| unit_symbology | `symbology/{roles,markers,atlas/raster,links}`; its squad links take `SlotUid` from orbat_slot_ids, which also holds `SlotId` | render_primitives, map_draw_lanes, newtype_ids, orbat_slot_ids |
| overlay_instances | `instances/{symbols,drag,patches}`, `fire_mission_marks`; `slots/` shim deleted | unit_symbology, map_draw_lanes, render_primitives |
| chunk_scheduler | `scheduler/{state,viewport,residency}` plus `deinterleave` and the revision queries, `indexing/world.rs`; the residency core returns rebuild requests | world_chunks, prefab_catalog, world_file_formats, spatial_indexes, map_draw_lanes, map_coordinates |
| chunk_draw_buffers | `streaming/buffers`, `buildings/footprint.rs`, the layer toggles (`toggles.rs`) and the residency statistics (`stats_json`); `WorldResidency`, the composed owner over the scheduler | chunk_scheduler, label_layout, road_network, vegetation, render_primitives, map_draw_lanes, prefab_catalog, world_chunks, map_coordinates |
| mission_editing_session | `editing/{host,history,batch,routing,selection_universe,picking,lanes}` | mission_document, mission_crdt, mission_validation, camera_math, spatial_indexes, unit_symbology; dev: mission_operations |
| mission_editing_commands | `editing/{hosted_commands,commands}` (`commands` as `document_text`) | session, mission_operations, mission_document, mission_model, mission_validation, formation_geometry, map_coordinates, orbat_slot_ids |
| mission_persistence | `editing/persist` | session, mission_document, mission_model, mission_payload |
| map_editing_tools | `editing/tools/*` (selection, ruler, line of sight, viewshed scheduler) with the source scrub their guards read; the LOS↔viewshed cycle stays internal; the placement math is formation_geometry's (S5) | session, the 3 LOS crates, terrain_elevation, camera_math, mission_crdt, mission_document, spatial_indexes, time_source; dev: terrain_relief |

## crates/graphics (map-agnostic), the browser half of crates/streaming, crates/map_rendering, crates/paper_doll

| Crate | From | Notes |
|---|---|---|
| render_primitives | `ge draw::{instances,geometry,compose,grid,triangulate,cull::oracle}`, `frame::{ids,damage,camera}`, `text/*`, WGSL consts | One home for colour normalisation and glyph packing |
| gpu_device (W) | `ge device/*`; `GpuContext` (create, resize, acquire, adapter limits, typed error), which both renderers adopt in place of their bootstraps; `instance_descriptor`, `WebDisplay`, `GpuTimer` | — |
| gpu_frame (W) | `ge pipeline`, `draw::{encode,lines,polygons,cull::compute}`, `frame::{packet,present,batch,buffers,atlas,text}`, `loop` as `frame_pump` | render_primitives |
| renderer_core (W) | new, plus the generic half of `frame/bindings.rs` (`packet_bindings`) | gpu_frame, render_primitives; `LaneSink`, `LayerContext`, `FrameHook`, `RenderStats` and its JSON |
| map_streaming_model | `bridge/{preferences,host_preferences,progress}`, the memory budget model with its tests; the `MapAssetSink` contract and its payload types (D-S8-2: the native build's model types, and the trait that keeps the streaming crates off the renderer) | render_primitives |
| map_asset_loading (W) | `loaders/*`, the world `*/loader.rs` files, `world/mesh.rs`, `satellite/quadtree`, `relief/host.rs`, the live memory budget and the asset statistics; writes the renderer only through `MapAssetSink` | map_streaming_model, chunk_scheduler, chunk_draw_buffers, the terrain, world format, world object, overlay and line of sight crates, browser_platform |
| map_streaming_host (W) | `streaming/host` | map_asset_loading, map_streaming_model, terrain_elevation, terrain_relief, world_chunks, world_line_of_sight, water_bodies, label_layout, browser_platform |
| symbology_layers_gpu (W) | `bridge_1/2/3` split by concern, `instances/{lanes,icon_cull_gpu,icon_uniforms,world_icon_lanes}`, `atlas/gpu`, `lanes_prefs` | renderer_core, gpu_frame, gpu_device, overlay_instances, unit_symbology, map_draw_lanes; `SlotSymbologyGpu`, `GlyphAtlasGpu`, `IconCullGpu` |
| world_layers_gpu (W) | buildings and vegetation `buffers.rs`, `satellite/textures`, `los/terrain/overlay.rs`, the textured lane record | renderer_core, gpu_frame, map_draw_lanes; typed layers own their GPU state |
| map_renderer (W) | `frame/*`, `camera/viewport.rs`, the calibration scene of `world/scene.rs`, the `MapAssetSink` implementation | renderer_core, gpu_frame, gpu_device, symbology_layers_gpu, world_layers_gpu, map_streaming_model; owns `RenderEngine` |
| map_render_diagnostics (W) | `diagnostics/{readback,bench,probes}` (`frame_1` split into the frame benchmark and the stress pool; `frame_2` became the renderer's statistics) | map_renderer, symbology_layers_gpu; functions over the renderer's diagnostic accessors |
| paper_doll_scene | `doll/scene` and interaction | camera_math |
| paper_doll_renderer (W) | `doll/renderer` (`lifecycle_1/2` merged), `doll.wgsl`, `readback/doll.rs` | gpu_device, paper_doll_scene, camera_math |

## crates/api (sqlx and axum only here)

Built in S9 (O12 amendments applied).

| Crate | From |
|---|---|
| api_identifiers | new: the serde- and sqlx-transparent typed ids (`newtype_ids` macros) of every API table key, Discord snowflake and game runtime key (C2, C3) |
| api_failpoints | `core/failpoints` (dev feature `failpoints`; `$crate` path fixed) |
| api_foundation | `core/{error_handling,wire_format,text,http}` |
| api_configuration | `core/{configuration,process_lifecycle}` |
| api_database | `core/database`, `migrations/`, `seeds/` (both folders move into the crate) |
| api_property_evidence | dev-only: `src/tests/property_evidence.rs`, the property run recorder (O6) |
| api_http_layer | `core/{authentication_primitives,middleware,observability,realtime_hub,http_client}` |
| api_audit_log | `required_audit`, `audit_writer`, `AuditSeverity` |
| api_mission_vocabulary | the vocabulary enums only: `TerrainType`, `GameMode`; the ORBAT template parser stays mission crate code (`mission_model::orbat`), which missions imports directly |
| api_equipment_datasets | `community_content/services/equipment_data_viewer/` (the dataset imports, the SQLite navigation index, the read queries); its handlers stay in api_community_content (C4) |
| api_discord | `DiscordService`, `WebhookService`, and its own webhook input `WebhookAnnouncement`, which community content maps its `Announcement` into |
| api_caller_identity | `MachineCaller`, `authenticate_machine` (the machine-caller half of `machine_credentials`), `session_authorization`, `cached_membership_permissions`, `identity_ownership`, `account_authority`, `arma_id_is_linked`, `UserRole`. `ExecutorKind` is in fleet_wire_contract; credential issue/list/revoke stay in server_infrastructure. |
| api_member_activity | `user_stats` (with `ATTENDANCE_RATE_SQL`), `leaderboard_view`, `participation_attribution`, `reevaluation_queue` |
| api_state | `AppState`, holding the concrete `DiscordService`, `WebhookService` and `EquipmentDatasets` and an `Arc<dyn SessionAuthority>`, and every `FromRef` projection; no `dyn EquipmentDatasets` |
| api_administration … api_server_infrastructure (8) | the domain folders; each exposes `routes() -> Router<AppState>`; anyhow → thiserror |
| api_background_workers | `background_workers` |

The api app (apps/api) keeps the router (`router.rs`), the composition root (`composition.rs`),
the `api` and `import_registry` bins, the layout and prose rules, and 150 integration binaries.
`staging_fixtures` moves to `tools/staging/staging_fixtures` with the `staging-fixtures` bin and
its 4 suites, so the total stays 154.

## crates/frontend (Leptos only here)

- **foundation/:**
  - `frontend_route_table` (the `router.rs` table plus `NAVIGATION`)
  - `frontend_ui` (`core/{ui,utils}` plus tokens)
  - `frontend_api_dtos` (newtype IDs)
  - `frontend_transport` (defines `TokenProvider`)
  - `frontend_session` (auth plus logout hooks)
  - `frontend_offline`
  - `frontend_map_view` (W)
  - `frontend_test_support` (dev-only: fixtures, `editor_operations`, `class_r_scrub`; the `pins.rs` pins move to their owners)
- **features/:** `mission_review_record`
- **pages/:** `administration_pages`, `operations_pages`, `mission_hub_pages`, `doctrine_pages`, `field_tools_pages`, `command_center_pages`, `account_pages`
- **workspaces/:**
  - `mission_creator_state` (lifted: outliner and inspector models, toolbelt and layout tokens, the arsenal catalog and rules model, `ConflictInfo`, `WidgetVariant`, `AssetPickerState`, `review_mode`, `world_layer_prefs`)
  - `mission_creator_engine_bridge` (bridge plus input)
  - `mission_creator_session` (shell)
  - `mission_creator_arsenal`
  - `mission_creator_workspace` (ui plus page)
  - `debug_benches`
- **Behaviour edges that cannot lift become injected callbacks:** `schedule_edit_persist`, `hydrate`, `build_catalog_tree`, `map_render_slot_soa`, `context_menu::open`, `publish_compile_findings`, `record_placed`. Any pair still cyclic after that is merged and recorded.

## tools/

Built in S11a, and the map asset crates in S11b.

| Family | Crates |
|---|---|
| foundation | `repository_layout` (the one root finder, the shared layout modules, root walkers, `target/` subdirectories) · `process_runner` (vc `proc`, the host bridge, `secure_shell_transport`, the terminal, binary, file, detached and streaming modes) · `verification_core` (verdict, scan, pattern, gate, lock, report) · `repository_laws` (all laws; `source_scrub` dev helper) · `deploy_settings` (`xt core/deploy_environment*`) · `tool_test_support` (dev-only: the test env lock, the working-directory guard, the test repository root) |
| tickets | `ticket_model` (with `commit_subjects`, `TicketId`) · `ticket_metrics` · `ticket_wave_lock` · `ticket_registry` (store, registry, ops, sync, validation, verbs; sits above metrics) · `ticketboard_model` (ticketboard's headless half) |
| commands | `ci_task_catalog` (`commands/{ci,build}`, `verifications/ci`, the cargo-target verification, the map asset steps over `map_asset_verification`, the wasm32 lint derived from the workspace) · `database_operations` (`commands/db`, `verifications/database`, `deploy/database_*`) · `deployment` (`commands/deploy`, `verifications/deployment`; → database_operations) · `staging_procedures` (`commands/staging`) · `api_readiness_checks` (`verifications/api_readiness`, property test configuration) · `mod_operations` (`commands/mod_ops`) · `platform_execution` (`commands/platform`; `wprintln!` internal) · `schema_tooling` (`commands/{generate,schema}`, `verifications/schemas`, with the font table) · `enfusion_mcp` (xt `mcp` plus dt `enfusion_mcp_entrypoint`) · `ballistics_oracle_tooling` (`commands/ballistics`) · `workstation_setup` (`commands/setup`) · `remote_debugging` (`commands/{debug,reproduction}`) · `repository_relocation` (`commands/refactor`) |
| checks | `documentation_checks` (`verifications/documentation`; the bin injects the `clap::Command` factory) · `mod_script_checks` (`verifications/mod_scripts`) · `repository_checks` (`verifications/{architecture,language_bans,licensing,registry}` and the cross-cutting tooling tests) |
| enfusion | `enfusion_pak` (dt `enfusion_pak`) · `enfusion_script_index` (dt `enfusion_tooling` plus xt `fetch`) · `enfusion_mcp_broker` (dt `mcp_broker`; the `mcpd` bin calls it) |
| map_assets (S11b) | `blueprint_compiler` (dt `blueprint`, its test fixtures) · `world_export_pipeline` (dt `world_export_pipeline`; the `world` bin) · `map_raster_pipeline` (dt `map_raster_pipeline`; the `map` bin; never in xtask's closure) · `map_asset_verification` (dt `map_verification`; xtask `schema`, `verify` and `map world-los`) |
| browser_testing | `chrome_devtools_protocol` (dt `browser_testing/cdp`) · `browser_gate_suites` (`dom_oracle`, gate server, capture, the ballistics and mortar suites (S11b), the `gate` and `capture` command lines) |
| staging | `staging_load_plan` (the tokio-free plan and report types) · `staging_load_generator` (dt `load_generation`; the `staging-load` bin) · `acknowledgement_dropping_relay` · `staging_fixtures` (the `staging-fixtures` bin and its 4 suites; built in S9, decision S11-D5) |

- `tools/xtask` holds the command line and the dispatch plus the command groups that are still its
  own modules (`agent_context`, `fetch`, `map`, `refactor`, `schema`, `ticket`, `verify`, `wave`;
  thin command lines over the crates above; the `map` group's terrain export driver and tile
  index writer live in `world_export_pipeline`); it depends only on tool crates, and no tokio, axum, reqwest, resvg or image
  enters its dependency closure (crate-tiers rule 6, judged from the unjudged binary).
- `tools/developer_tools` holds 8 one-line bins (`enf`, `gate`, `mcpd`, `world`, `map`, `capture`,
  `acknowledgement-dropping-relay`, `staging-load`) over the tool crates and no library; no member
  depends on it and it depends on no member under `legacy/` (S11b).
