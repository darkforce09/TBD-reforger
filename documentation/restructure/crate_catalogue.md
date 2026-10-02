**Status:** live

# Crate catalogue

Every crate the program creates, grouped by category: where its code comes from, what it depends
on inside the workspace, and what it fixes. The tree is in
[target_file_tree.md](/documentation/restructure/target_file_tree.md); the tier law that checks
the dependency columns is in [laws_and_gates.md](/documentation/restructure/laws_and_gates.md).

Abbreviations in code spans:

| Prefix | Path |
|---|---|
| `me` | `apps/website/map-engine/src/` |
| `ge` | `apps/website/graphics-engine/src/` |
| `api` | `apps/website/api_v2/src/` |
| `fe` | `apps/website/frontend/src/v2/` |
| `xt` | `tools/xtask/src/` |
| `dt` | `tools/developer_tools/src/` |
| `te` | `tools/ticket_engine/src/` |
| `vc` | `tools/verification_core/src/` |

A bare module path such as `world/scene.rs` is relative to the map engine's source folder. "W" marks a
wasm-only crate. Tiers follow tier = 1 + the highest dependency tier, and the crate-tiers law
checks them.

## crates/foundation, crates/contracts (T0)

| Crate | From | Fixes |
|---|---|---|
| newtype_ids | new | `string_id!`/`integer_id!`/`uuid_id!` macros: serde-transparent, `Borrow<str>`, `#[sqlx(transparent)]` behind a cfg |
| time_source | the 5 clocks (`me diagnostics/timing/gpu.rs:11`, `editing/tools/viewshed_scheduler/host.rs:89`, `streaming/host/viewport.rs:80`, `world_loader/ingest.rs:59,92`, `data/store/crdt/undo_groups/clocks.rs`), `dt timestamp_formatting.rs`, `te timestamp.rs` | One `Clock` trait (system/browser/fixed) and UTC formatting |
| deterministic_random | SplitMix64 (`store/operations/placement/geometry.rs`, `ballistics/agreement_cases.rs:81`), the LCG (`rows/slot_edits.rs:31`) | One generator |
| content_digest | `dt content_digest.rs`, 11 ad-hoc SHA helpers, hand-rolled `ballistics/calibration/sha256_digest.rs:32` | sha2 0.11 only |
| http_url_guard | `fe core/auth/url_guard.rs:66-76`, `api core/text/http_url_guard.rs:62-79`, `apps/website/shared/is_http_url_cases.rs` | One predicate; a `cases` module replaces the 10 `include!` sites |
| browser_platform (W) | `me diagnostics/platform/console.rs`, `streaming/loaders/fetch.rs`, DEM `gloo_net` use | Breaks the console-macro cycles |
| fleet_wire_contract | `api server_infrastructure/models/fleet_command.rs:40-262`, agent `ledger_client/ledger_messages.rs:13-56`, both `secret_files.rs` | 3 of the 4 copies merged; the frontend DTO stays separate (D7) |
| contract_schema_types | the 266 typify files in `api/*/models/generated` and `missions/contract/generated` | Generated (exempt from the ID rule); codegen emits the anatomy files |
| offline_cache_policy | service-worker lib: `cache_names`, `network_fallback`, `offline_pack`, `request_classification` | `range_slicing` stays in the bin |

## crates/mission, crates/ballistics

| Crate | T | From | Deps |
|---|---|---|---|
| mission_wire_safety | 0 | `scenario/validation/wire_safety` | — |
| mission_model | 1 | `scenario/{ast,extensions,slot_line}` | newtype_ids |
| mission_payload | 2 | `compiler/{payload,kit}` | model |
| mission_validation | 3 | `validation/validator` | payload, wire_safety |
| mission_compiler | 4 | `compiler/flatten`; `COMPILER_PACKAGE_VERSION` pinned as a literal | the chain |
| formation_geometry | 1 | `store/operations/placement` | deterministic_random |
| mission_crdt | 1 | `store/crdt` (no `js_sys`; clock injected) | time_source |
| mission_document | 2 | `store/{rows,selection}` plus `crdt/id_arrays/mission_doc_tests` | crdt, newtype_ids; dev: payload, compiler (round-trip and former RULE4 tests) |
| mission_operations | 3 | all of `store/operations` (entity + the rest + `assets, cargo_rules, environment, projections, reassign, rotation, rows, slot_ids, zones`) | document, payload, model, formation_geometry, map_coordinates |
| ballistics_model | 0 | `ballistics/{angular_units,catalog,wind,flight_model}` | — |
| ballistics_solver | 1 | `solver, crest_clearance, dispersion` | model |
| fire_mission_planning | 2 | `fire_mission, fire_mission_comparison, fuze, battery, solution_wording` | solver |
| ballistics_calibration | 2 | `calibration` | solver, content_digest |
| ballistics_agreement_cases | 3 | `agreement_cases` (a test oracle moved out of prod) | planning, deterministic_random |

## Engine CPU crates (geometry, world_formats, terrain, world_objects, line_of_sight, map_overlay, streaming, mission_editing)

| Crate | From | Deps (workspace) |
|---|---|---|
| geometry_primitives | `bvh/node.rs:25-39` vector ops, `blueprint/geometry.rs`, compound `Rigid`, `Bounds3`, both `point_at` | — |
| map_coordinates | `world/scene.rs` consts + `world_rect_rel`, `APPLY_ANCHOR_X/Y` (`apply_faction/library.rs:11,14`), `probes/runner.rs:87`, `chunk_math.rs`, `camera/math/shaping.rs`, `grid_reference.rs` | — |
| camera_math | `camera/{math,ortho,orbit}` | map_coordinates |
| spatial_indexes | `spatial/bvh`, `indexing/{cluster,point_index,picking}`, TLAS build core (`los/world/tlas.rs:51`) unified with `bvh/traversal.rs:50` | geometry_primitives |
| world_file_formats | `io/*` (shims `archives/models` and `containers/headers` deleted) | — |
| prefab_catalog | `buildings/{prefab,obb}`, `environment/classify`, `loaders/prefab` | formats |
| world_chunks | `loaders/{chunk,chunk_bin,manifest,residency}` | prefab, formats |
| world_store | `loaders/store.rs` (world store over chunks, roads, vegetation regions) | chunks, vegetation, road_network |
| terrain_elevation | `terrain/dem` without the loader; the `sample/` shim deleted | formats, map_coordinates |
| terrain_relief | `relief/{contours,sea_band,hillshade}` (host moves to the GPU crate) | elevation, map_coordinates |
| water_bodies | `water/{vectors,mesh}` | relief, render_primitives, formats |
| road_network | `terrain/roads`, plus `road_class_name` moved in from `route_placement` | formats, prefab, elevation, render_primitives, map_coordinates, map_draw_lanes |
| satellite_imagery | `satellite/streamer` | formats |
| vegetation | `vegetation/{regions,mass,canopy,density}` | formats, prefab, map_draw_lanes, world_chunks |
| place_names | `environment/locations` without the loader; owns town/peak/route → `LabelSpec`; `routes/` shim deleted | label_layout, elevation, road_network |
| building_interiors | `architecture/{blueprint,compound,section}` (dev feature `test_fixtures`) | spatial_indexes, formats, geometry_primitives |
| terrain_line_of_sight | `los/terrain` without `overlay.rs` | elevation |
| interior_line_of_sight | `los/interior` | building_interiors, terrain_line_of_sight |
| world_line_of_sight | `los/world`; one `evaluate_los` (replacing the 3 at `attribution_1.rs:119`, `los.rs:24`, `walker.rs:420`); reference `dda.rs:84` moves to tests | world_chunks, interior_line_of_sight, map_coordinates |
| map_draw_lanes | `overlay/{lanes,lod}`, plus `building_visible` (`footprint.rs:46`) and `BUILDING_MIN_ZOOM` | render_primitives |
| label_layout | `symbology/{labels,text_packing,text_metrics}`; the glyph-packing copies in `glyph_math` deleted | render_primitives, map_draw_lanes |
| unit_symbology | `symbology/{roles,markers,atlas/raster,links}` | render_primitives |
| overlay_instances | `instances/{symbols,drag,patches}`, `fire_mission_marks`; `slots/` shim deleted | unit_symbology, spatial_indexes |
| chunk_scheduler | `scheduler/{state,viewport,residency}` plus `deinterleave`, `indexing/world.rs` | world_store, prefab, spatial_indexes, map_draw_lanes |
| chunk_draw_buffers | `streaming/buffers`, `buildings/footprint.rs` | scheduler, label_layout, road_network, vegetation |
| mission_editing_session | `editing/{host,history,batch,routing,selection_universe,picking,lanes}` | mission_document, mission_operations, spatial_indexes |
| mission_editing_commands | `editing/{hosted_commands,commands}` | session, mission_validation, camera_math |
| mission_persistence | `editing/persist` | session, mission_compiler |
| map_editing_tools | `editing/tools/*`; the LOS↔viewshed cycle stays internal; `editing/tools/placement.rs` deleted | commands, the 3 LOS crates, camera_math |

## crates/graphics (map-agnostic), crates/map_rendering, crates/paper_doll

| Crate | From | Notes |
|---|---|---|
| render_primitives | `ge draw::{instances,geometry,compose,grid,triangulate,cull::oracle}`, `frame::{ids,damage,camera}`, `text/*`, WGSL consts | One home for colour normalisation and glyph packing |
| gpu_device (W) | `ge device/*`; `GpuContext` replacing both bootstraps (`frame/boot.rs:57-112`, `doll/renderer/lifecycle_1.rs:123-166`); `instance_descriptor` | — |
| gpu_frame (W) | `ge pipeline`, `draw::{encode,lines,polygons,cull::compute}`, `frame::{packet,present,batch,buffers,atlas,text}`, `loop` | — |
| renderer_core (W) | new, plus the `frame/bindings.rs` registries | `LaneSink`, `LayerContext`, `RenderStats`, `FrameHook` |
| symbology_layers_gpu (W) | `bridge_1/2/3` merged, `instances/lanes`, `atlas/gpu`, `lanes_prefs` | `SlotSymbologyGpu`, `GlyphAtlasGpu`, `IconCullGpu` |
| world_layers_gpu (W) | buildings/vegetation `buffers.rs`, `satellite/{textures,quadtree}`, `relief/host.rs`, `los/terrain/overlay.rs` | typed layers own their GPU state |
| map_asset_loading (W) | `loaders/{world_loader,occluder_loader}`, the world `*/loader.rs` files, `bridge/progress` | in `streaming/` |
| map_streaming_host (W) | `streaming/{host,memory}`, `bridge/{preferences,toggles,statistics}` | in `streaming/` |
| map_renderer (W) | `frame/*`, `camera/viewport.rs`, `publish_engine` | owns `RenderEngine` |
| map_render_diagnostics (W) | `diagnostics/{readback,bench,probes}` (`frame_1/2` merged) | functions over accessors |
| paper_doll_scene | `doll/scene` and interaction | camera_math |
| paper_doll_renderer (W) | `doll/renderer` (`lifecycle_1/2` merged), `doll.wgsl`, `readback/doll.rs` | gpu_device, paper_doll_scene, camera_math |

## crates/api (sqlx and axum only here)

| Crate | From |
|---|---|
| api_failpoints | `core/failpoints` (dev feature `failpoints`; `$crate` path fixed) |
| api_foundation | `core/{error_handling,wire_format,text,http}` |
| api_configuration | `core/{configuration,process_lifecycle}` |
| api_database | `core/database`, `migrations/`, `seeds/` |
| api_audit_log | `required_audit`, `audit_writer`, the `audit_log` model |
| api_mission_vocabulary | `TerrainType`, the ORBAT template parser |
| api_http_layer | `core/{authentication_primitives,middleware,observability,realtime_hub,http_client}` |
| api_discord | `DiscordService`, `WebhookService` |
| api_state | `AppState`, with `Arc<dyn SessionAuthority>` and `Arc<dyn EquipmentDatasets>` |
| api_caller_identity | `MachineCaller`, `authenticate_machine`, the `ExecutorKind` model; `session_authorization`, `identity_ownership`, `account_authority`, `arma_id_is_linked`, `UserRole`. Credential issue/list/revoke stay in server_infrastructure. |
| api_member_activity | `user_stats` (with `ATTENDANCE_RATE_SQL`), `leaderboard_view`, `participation_attribution`, `reevaluation_queue` |
| api_administration … api_server_infrastructure (8) | the domain folders; each exposes `routes() -> Router<AppState>`; anyhow → thiserror |
| api_background_workers | `background_workers` |

The api app (apps/api) keeps the router composition, the `api` and `import_registry` bins, and the 154 integration binaries. `staging_fixtures` moves to tools/staging.

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

| Family | Crates |
|---|---|
| foundation | `repository_layout` (the 7 root finders, the 4 layout modules, root walkers, `target/` subdirectories) · `process_runner` (vc `proc`, `xt core/host_execution.rs`, `secure_shell_transport`) · `verification_core` (verdict, scan, pattern, gate, lock, report) · `repository_laws` (all laws; `source_scrub` dev helper) |
| tickets | `ticket_model` (with `commit_subjects`) · `ticket_metrics` · `ticket_registry` (store, registry, ops, sync, validation, verbs; sits above metrics) · `ticket_wave_lock` · `ticketboard_model` (headless) |
| commands | `ci_task_catalog` (threads the injected `clap::Command` for link-check) · `database_operations` (`commands/db`, `verifications/database`, `deploy/database_*`) · `deployment` (→ database_operations) · `staging_procedures` · `api_readiness_checks` · `mod_operations` · `platform_execution` (`wprintln!` internal) · `schema_tooling` (with `gen_font_table`) · `enfusion_mcp` (xt daemon plus dt broker) · `ballistics_oracle_tooling` · `workstation_setup` · `remote_debugging` |
| checks | `documentation_checks` · `mod_script_checks` · `repository_checks` |
| enfusion | `enfusion_pak` · `enfusion_script_index` |
| map_assets | `blueprint_compiler` · `world_export_pipeline` · `map_raster_pipeline` · `map_asset_verification` |
| browser_testing | `chrome_devtools_protocol` · `browser_gate_suites` (`dom_oracle`, gate server, capture) |
| staging | `staging_load_generator` · `acknowledgement_dropping_relay` · `staging_fixtures` (bin) |

- `tools/xtask` keeps only the CLI and dispatch; the `TopCmd` tests move here.
- `tools/developer_tools` keeps 7 one-line bins.

