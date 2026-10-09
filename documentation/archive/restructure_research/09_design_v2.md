**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# Foundations-up design, second pass

# Foundations-up workspace plan, v2: 147 workspace members, built bottom-up by the strangler method

**Summary**
- **Size.** The design has 142 library crates in 15 crate categories plus 9 tool families, 5 apps, and 3 legacy crates that shrink to zero. That is above the brief's 70–110. The tools (124k prod lines) and frontend (~100k) are as big as the engine. Q1 asks you to accept the count and lists the merges that would bring it down.
- **New finding that changes the rendering design.** All 148 `#[wasm_bindgen]` sites in map-engine (frame 48, overlay 44, diagnostics 15, camera 13, world 13, doll 12, spatial 2, data 1) are exports that nothing in JavaScript uses. Every caller is Rust: the frontend goes through `EngineHandle = Rc<RefCell<Option<RenderEngine>>>` (`map-engine/src/frame/mod.rs:148`) and `DollEngine` (`frontend/src/v2/apps/editor/arsenal/doll.rs:35,117`). No `wasmBindings` reference exists, and no tool names `RenderEngine` outside the law pins. The gates use `window.__*` hooks the frontend sets itself. So the operator's facade-crate blocker can be removed outright: strip the attribute (Q2).
- **Execution model.** The strangler method wins, with one rule: no shim survives a commit (§D).

## A. Workspace map

Abbreviations: `me/` = map-engine src, `ge/` = graphics-engine src, `api/` = api_v2 src, `fe/` = frontend src/v2, `xt/ dt/ te/ vc/` = xtask, developer-tools, ticket-engine and verification-core src. "kL" is approximate prod kLoC. **W** marks a wasm-only crate: `#![cfg(target_arch="wasm32")]` in lib.rs and its deps in target tables, so native `--workspace` builds stay green. Deps list workspace crates only. T is the longest-path tier.

```
apps/      api · frontend · fleet_host_agent · ticketboard · offline_service_worker · mod/
crates/    foundation contracts mission ballistics geometry world_formats terrain world_objects
           line_of_sight map_overlay streaming graphics map_rendering paper_doll mission_editing
           api frontend/{foundation,features,pages,workspaces}
tools/     xtask · developer_tools · foundation tickets commands checks enfusion map_assets
           browser_testing staging · enfusion_mcp_node_package
legacy/    map_engine graphics_engine ticket_engine   (strangler zone, deleted S8/S11)
deploy/  assets/  contracts/  documentation/
```

**foundation/ and contracts/ (all T0)**

| Crate | From | kL | Notes / fixes |
|---|---|---|---|
| newtype_ids | new | .15 | `string_id!`/`integer_id!` (serde-transparent, `Borrow<str>`). A true tier-0 primitive |
| time_source | me `diagnostics/timing/gpu.rs:11`, `editing/tools/viewshed_scheduler/host.rs:89`, raw `Date::now` at `streaming/host/viewport.rs:80` and `world_loader/ingest.rs:59,92`, `data/store/crdt/undo_groups/clocks.rs`; dt `timestamp_formatting.rs`; te `timestamp.rs` | .35 | `Clock` trait with system, browser and fixed clocks, plus UTC formatting. Replaces 5 clocks and 2 timestamp helpers |
| deterministic_random | SplitMix64 at `store/operations/placement/geometry.rs` and `ballistics/agreement_cases.rs:81`; LCG at `rows/slot_edits.rs:31` | .1 | Under 300 lines, kept because it is one reproducible generator shared by two domains |
| content_digest | dt `content_digest.rs`, SHA helpers in 8 xt and 3 dt files, `ballistics/calibration/sha256_digest.rs:32` | .15 | sha2 0.11 only; the hand-rolled SHA-256 is deleted |
| http_url_guard | `fe/core/auth/url_guard.rs:66-76`, `api/core/text/http_url_guard.rs:62-79`, `apps/website/shared/is_http_url_cases.rs` | .25 | A `cases` module replaces the 9 `include!` sites |
| browser_platform W | `me/diagnostics/platform/console.rs`, `streaming/loaders/fetch.rs`, the DEM's direct gloo_net use | .35 | Removes the world→diagnostics console-macro cycle |
| fleet_wire_contract | `api/server_infrastructure/models/fleet_command.rs:40-262`, agent `ledger_client/ledger_messages.rs:13-56` (with `ErrorEnvelope`), both `secret_files.rs` | .4 | Replaces 3 of the 4 copies. The frontend DTO stays separate (D7) |
| contract_schema_types | the 266 typify files in `api/*/models/generated` and `missions/contract/generated` | 18.7 gen | regress. Generated, so exempt from the ID rule. The output root at `xt/commands/generate/schema_types.rs:23` moves |
| offline_cache_policy | service-worker lib: `cache_names`, `network_fallback`, `offline_pack`, `request_classification` | ~.6 | `range_slicing` stays in the service-worker bin; the frontend never uses it |

**mission/ and ballistics/**

| Crate | T | From | kL | Deps | Notes |
|---|---|---|---|---|---|
| mission_wire_safety | 0 | `data/scenario/validation/wire_safety` | .4 | — | api uses it directly |
| mission_model | 1 | `scenario/{ast,extensions,slot_line}` | 3.9 | newtype_ids | `slot_line` (69 lines) folds in |
| mission_payload | 2 | `compiler/{payload,kit}` | .47 | model | |
| mission_validation | 3 | `validation/validator` | 1.7 | payload, wire_safety | |
| mission_compiler | 4 | `compiler/flatten` | 2.5 | the chain | api's compile/flatten entry point |
| formation_geometry | 1 | `store/operations/placement` | .66 | deterministic_random | Verified pure: only `super::` imports |
| mission_crdt | 1 | `store/crdt` | .7 | time_source | yrs; `js_sys` removed |
| mission_document | 5 | `store/{rows,selection}` plus the two `RULE4_PIN` round-trip tests | 6.2 | crdt, compiler | Stays one crate: 30 files of `impl MissionDocCore` |
| mission_entity_operations | 6 | `operations/entity` | 2.8 | document | |
| mission_document_operations | 6 | `operations/{apply_faction,place_orbat,faction_library,attrs,transform,compositions,tactical_graphics,cargo,document_index}` | 3.9 | document, formation_geometry, map_coordinates | |
| ballistics_model | 0 | `ballistics/{angular_units,catalog,wind,flight_model}` | 1.4 | — | Holds the wind↔flight and units↔catalog cycles internally |
| ballistics_solver | 1 | `solver`, `crest_clearance`, `dispersion` | 1.8 | model | |
| fire_mission_planning | 2 | `fire_mission`, `fire_mission_comparison`, `fuze`, `battery`, `solution_wording` | 1.6 | solver | `battery`→`agreement_cases` is a doc link only (verified) |
| ballistics_calibration | 2 | `calibration` | 1.45 | solver, content_digest | |
| ballistics_agreement_cases | 3 | `agreement_cases` | .4 | planning, deterministic_random | Takes the oracle out of the solver. Consumers: the debug bench and the gate suites |

**Engine CPU crates**

| Crate | T | From | kL | Deps | Notes |
|---|---|---|---|---|---|
| geometry_primitives | 0 | `bvh/node.rs:25-39`, `blueprint/geometry.rs`, compound `Rigid`, `Bounds3`, both `point_at` | .5 | — | Removes the geometry duplicates |
| map_coordinates | 0 | `world/scene.rs` consts, `APPLY_ANCHOR_X/Y`, `probes/runner.rs:87`, `chunk_math.rs`, `shaping.rs`, `grid_reference.rs` | .6 | — | Removes 4 copies of the map centre |
| camera_math | 1 | `camera/{math,ortho,orbit}` | .83 | coordinates | |
| spatial_indexes | 1 | `spatial/bvh`, `indexing/{cluster,point_index,picking}`, TLAS core from `los/world/tlas.rs:51` | 1.9 | primitives | One BVH build core. The sidecar format is shared with the dt blueprint compiler |
| world_file_formats | 0 | `io/*` | 2.0 | — | rkyv. Deletes the `archives/models` (88) and `containers/headers` (14) shims |
| prefab_catalog | 1 | `buildings/{prefab,obb}`, `environment/classify`, `loaders/prefab` | .8 | formats | The render-class codes are wire format |
| world_chunks | 2 | `loaders/{chunk,chunk_bin,manifest,store,residency}` | 1.1 | prefab, formats | Moving `WorldChunk` down breaks the spatial↔streaming cycle |
| terrain_elevation | 1 | `terrain/dem` without `loader` and the `sample/` shim | .8 | formats, coordinates | png |
| terrain_relief | 2 | `relief/{contours,sea_band,hillshade}` | .75 | elevation, draw_lanes, render_primitives | `host.rs` moves out, which breaks the relief↔water cycle |
| water_bodies | 3 | `water/vectors` | .48 | relief | |
| road_network | 2 | `terrain/roads` | 1.2 | formats, prefab, draw_lanes | |
| satellite_imagery | 1 | `satellite/streamer` | .88 | formats | |
| vegetation | 2 | `vegetation/{regions,mass,canopy,density}` | 1.1 | formats, prefab, draw_lanes | |
| place_names | 3 | `locations` without `loader` | 1.2 | label_layout, elevation, roads | Owns the town/peak→`LabelSpec` builders; the `routes/` shim (35) is deleted |
| building_interiors | 2 | `architecture/{blueprint,compound,section}` | 2.9 | spatial_indexes, formats | Carries the `test_fixtures` dev feature (D8). Merged 3→1 because the three share types and consumers |
| terrain_line_of_sight | 2 | `los/terrain` without `overlay.rs` | .68 | elevation | |
| interior_line_of_sight | 3 | `los/interior` | .9 | interiors, terrain_los | |
| world_line_of_sight | 4 | `los/world` | 2.4 | chunks, interior_los | One `evaluate_los` instead of 3. The reference algorithm at `dda.rs:84` moves to tests |
| map_draw_lanes | 1 | `overlay/{lanes,lod}`, `BUILDING_MIN_ZOOM` | .62 | render_primitives | `LaneRole` stays one closed enum because draw order is global |
| label_layout | 2 | `symbology/{labels,text_packing,text_metrics}` | .69 | render_primitives, draw_lanes | Deletes the copies in `glyph_math` |
| unit_symbology | 2 | `symbology/{roles,markers,atlas/raster,links}` | .94 | render_primitives | |
| overlay_instances | 3 | `instances/{symbols,drag,patches}`, `fire_mission_marks` | .83 | unit_symbology, spatial_indexes | Deletes the `slots/` shim (70) |
| chunk_scheduler | 3 | `scheduler/{state,viewport,residency}`, `indexing/world.rs` | 1.1 | chunks, prefab, spatial_indexes | |
| chunk_draw_buffers | 4 | `streaming/buffers`, `buildings/footprint.rs` | 1.0 | scheduler, labels, roads, vegetation | |
| mission_editing_session | 6 | `editing/{host,history,batch,routing,selection_universe,picking,lanes}` | 1.1 | mission_document, spatial_indexes | |
| mission_editing_commands | 7 | `editing/{hosted_commands,commands}` | 2.3 | session, both operations crates | |
| mission_persistence | 7 | `editing/persist` | .8 | session, compiler | |
| map_editing_tools | 8 | `editing/tools/{ruler,selection,line_of_sight,viewshed_scheduler}` | 3.0 | commands, the 3 LOS crates | Keeps the LOS↔viewshed cycle internal |

**graphics/ (map-agnostic, rule 2 kept), map_rendering/ and paper_doll/**

| Crate | T | From | kL | Notes |
|---|---|---|---|---|
| render_primitives | 0 | ge `draw::{instances,geometry,compose,grid,triangulate,cull::oracle}`, `frame::{ids,damage,camera}`, `text/*`, WGSL consts | 2.2 | bytemuck, earcutr. Single home for colour normalisation; the `layout` facade dissolves |
| gpu_device W | 1 | ge `device/*`; `GpuContext` from `frame/boot.rs:57-112` and `doll/lifecycle_1.rs:123-166`; `instance_descriptor` (`boot.rs:458`) | .7 | One wgpu bootstrap instead of two |
| gpu_frame W | 2 | ge `pipeline`, `draw::{encode,lines,polygons,cull::compute}`, `frame::{packet,present,batch,buffers,atlas,text}`, `loop` | 2.6 | |
| renderer_core W | 3 | new, plus the `frame/bindings.rs` registries | .5 | `LaneSink`, `LayerContext` (holds `&GpuContext` and shared layouts), `RenderStats`, `FrameHook` |
| symbology_layers_gpu W | 4 | `bridge_1/2/3` merged into `slot_bridge`, `instances/lanes`, `atlas/gpu`, `lanes_prefs` | 1.6 | `SlotSymbologyGpu`, `GlyphAtlasGpu`, `IconCullGpu` |
| world_layers_gpu W | 7 | buildings and vegetation `buffers.rs`, `satellite/{textures,quadtree}`, `relief/host.rs`, `los/terrain/overlay.rs` | 1.9 | `BuildingLayerGpu`, `ForestLayerGpu`, `SatelliteTexLayers`, `ViewshedOverlayGpu` |
| map_renderer W | 8 | `frame/*`, `camera/viewport.rs`, `publish_engine` (`statistics.rs:313`) | 2.6 | Owns `RenderEngine` |
| map_render_diagnostics W | 9 | `diagnostics/{readback,bench,probes}` | 2.9 | Free functions over accessors; `frame_1/2` merged |
| map_asset_loading W | 5 | `loaders/{world_loader,occluder_loader}`, world `*/loader.rs`, `bridge/progress` | 2.0 | Placed in `streaming/` |
| map_streaming_host W | 6 | `streaming/{host,memory}`, `bridge/{preferences,toggles,statistics}` | 2.3 | Placed in `streaming/` |
| paper_doll_scene | 2 | `doll/scene` and interaction | .56 | |
| paper_doll_renderer W | 3 | `doll/renderer` (`lifecycle_1/2` merged), `doll.wgsl`, `readback/doll.rs` | .8 | Depends on gpu_device only |

**api/ (sqlx and axum allowed only here)**

| Crate | T | From | kL | Notes |
|---|---|---|---|---|
| api_failpoints | 0 | `core/failpoints` | .55 | A dev-only `failpoints` feature (Q6); the `$crate` path fixed |
| api_foundation | 1 | `core/{error_handling,wire_format,text,http}` | .65 | Adds a `uuid_id!` macro for sqlx-transparent newtypes |
| api_configuration | 2 | `core/{configuration,process_lifecycle}` | .6 | |
| api_database | 3 | `core/database`, `migrations/`, `seeds/` | .25 | Under 300, kept because it owns the schema |
| api_audit_log | 2 | `required_audit`, `audit_writer`, the `audit_log` model | .23 | Breaks the cycles through 6 domains |
| api_mission_vocabulary | 2 | `TerrainType`, the ORBAT template parser | .3 | Breaks missions↔operations and missions↔server_infrastructure |
| api_http_layer | 4 | `core/{authentication_primitives,middleware,observability,realtime_hub,http_client}` | 2.1 | |
| api_discord | 5 | `DiscordService`, `WebhookService` | .5 | `AppState` stops owning domain code |
| api_state | 6 | `core/application_state.rs` | .2 | `Arc<dyn SessionAuthority>`, `Arc<dyn EquipmentDatasets>` |
| api_caller_identity | 7 | `session_authorization`, `identity_ownership`, `machine_credentials`, `MachineCaller` | .5 | |
| api_member_activity | 8 | `user_stats` (with `ATTENDANCE_RATE_SQL`), `participation_attribution`, `reevaluation_queue` | .35 | |
| api_{administration, command_center, community_content, identity_and_access, match_telemetry, missions, operations, server_infrastructure} | 9 | the domain folders | 1.9 / .6 / 7.6 / 3.3 / 2.5 / 7.0 / 11.5 / 3.6 | Each exposes `routes() -> Router<AppState>`. anyhow becomes thiserror |
| api_background_workers | 10 | `background_workers` | .8 | |

**frontend/ (Leptos only here)**
- **foundation/:**
  - `frontend_route_table` (T0, `router.rs` table)
  - `frontend_ui` (T0, `core/{ui,utils}` plus the tokens)
  - `frontend_api_dtos` (T7, 3.6k): DTOs deliberately differ from the API models (D7); their 141 primitive IDs become newtypes
  - `frontend_transport` (T8, 2.6k): defines the `TokenProvider` trait
  - `frontend_session` (T9): auth plus logout hooks
  - `frontend_offline` (T9)
  - `frontend_map_view` W (T9)
- **features/:** `mission_review_record` (removes the administration→mission_hub edge).
- **pages/:** `administration_pages` 15.0, `operations_pages` 5.0, `mission_hub_pages` 4.7, `doctrine_pages` 4.7, `field_tools_pages` 4.0, `command_center_pages` 1.6, `account_pages` 0.6 (kL). There are no page→page edges.
- **workspaces/:**
  - `mission_creator_state`: shared models, contexts and layout tokens lifted out of `ui/outliner`, `ui/inspector` and `docks/toolbelt`
  - `mission_creator_engine_bridge` (bridge plus input, 7.7k)
  - `mission_creator_session` (shell, 5.3k)
  - `mission_creator_arsenal` (5.8k)
  - `mission_creator_workspace` (UI plus page, 26k)
  - `debug_benches` (6.0k)

**tools/**

| Family | Crates (kL) |
|---|---|
| foundation | `repository_layout` T0 (.8: the 7 root finders, 4 layout modules, root walkers, `target/` dirs) · `process_runner` T0 (1.15: vc `proc`, xt `host_execution`, `secure_shell_transport`) · `verification_core` T1 (.8) · `repository_laws` T2 (~3.5: all laws, plus `source_scrub` as a dev helper) |
| tickets | `ticket_model` (1.1, with `commit_subjects`) · `ticket_registry` (5.0: store, registry, ops, sync, validation, verbs; ops↔registry merge) · `ticket_wave_lock` (1.4) · `ticket_metrics` (1.4) · `ticketboard_model` (3.5) |
| commands | `ci_task_catalog` (2.2) · `database_operations` (3.5) · `deployment` (5.5) · `staging_procedures` (14.2) · `api_readiness_checks` (2.2) · `mod_operations` (10.6) · `platform_execution` (9.9, keeps `wprintln!` internal) · `schema_tooling` (3.7, includes `gen_font_table`) · `enfusion_mcp` (2.7, xt daemon plus dt broker) · `ballistics_oracle_tooling` (2.3) · `workstation_setup` (1.95) · `remote_debugging` (1.8) |
| checks | `documentation_checks` (5.2) · `mod_script_checks` (4.9) · `repository_checks` (5.3) |
| enfusion | `enfusion_pak` (.5) · `enfusion_script_index` (2.0) |
| map_assets | `blueprint_compiler` (9.8) · `world_export_pipeline` (7.6) · `map_raster_pipeline` (5.9) · `map_asset_verification` (2.9) |
| browser_testing | `chrome_devtools_protocol` (1.5) · `browser_gate_suites` (9.5, includes `dom_oracle`, the gate server and capture) |
| staging | `staging_load_generator` (2.8) · `acknowledgement_dropping_relay` (1.3) · `staging_fixtures` (a bin crate, from `api/bin`) |

**Thin entry points**
- `tools/xtask` keeps only the CLI and dispatch.
- `tools/developer_tools` keeps 7 one-line bins.
- `apps/api` keeps the router composition and 2 bins. The 154 integration binaries stay, one database per binary (D7).
- `apps/frontend` keeps `main.rs`, `app_routes.rs` and `shell/`.
- `fleet_host_agent` stays whole: it is a single process with no second consumer; only its wire shapes leave.
- `ticketboard` is the egui UI.

**Total:** 142 libraries plus 5 apps.

## B. Tier DAG and laws

**Tier bands**
- **T0:** foundation, contracts, the leaf crates (`wire_safety`, `ballistics_model`, `geometry_primitives`, `map_coordinates`, `world_file_formats`, `render_primitives`), `api_failpoints`, `frontend_ui`, `frontend_route_table`, `repository_layout`, `process_runner`.
- **T1–4:** mission, ballistics, geometry, formats, terrain, world objects, line of sight, overlay, the GPU device/frame/core, API infrastructure and kernel, tool foundations and tickets.
- **T5–9:** streaming, map rendering, mission editing, `api_state`/domains, frontend foundation.
- **T10 and up:** API workers, frontend pages and workspaces, tool command and check libraries.
- **Roots:** `apps/*`, `xtask`, `developer_tools`.

**`verify crate-tiers` (`repository_laws`)**
1. Every member declares `[package.metadata.layout] category`, `tier` and `targets = "any" | "wasm32"`, and `legacy = true` where it applies. Every `Cargo.toml` under `apps/`, `crates/`, `tools/` and `legacy/` is a member. The path is `<category>/<name>` and the package name equals the directory name.
2. Normal and build edges point strictly downward. The declared tier equals 1 + the maximum dependency tier, so the map stays truthful.
3. Allowed category edges:
   - foundation → none
   - contracts → foundation
   - mission and ballistics → foundation, `map_coordinates`
   - geometry, graphics, world_formats → foundation and lower geometry
   - terrain, world_objects, line_of_sight, map_overlay, streaming → those categories, plus `render_primitives` only from graphics
   - map_rendering and paper_doll → any engine crate
   - mission_editing → mission, geometry, world, line_of_sight, map_overlay
   - api → foundation, contracts, mission, ballistics, api
   - frontend → any crate except api
   - tools → CPU crates and tools only (never a W crate, never api or frontend; the one exception is `staging_fixtures` → api)
4. A W crate is reachable only from W crates, or through `cfg(target_arch="wasm32")` dependency tables.
5. Firewalls:
   - wgpu only in `gpu_device`, `gpu_frame`, `renderer_core`, `map_rendering/*` and `paper_doll_renderer`
   - web-sys, js-sys, wasm-bindgen and gloo only in W crates, `time_source` (behind a wasm cfg) and the frontend
   - sqlx and axum only in api
   - leptos only in the frontend
   - tokio, axum, reqwest, resvg and image are never in the dependency closure of `tools/xtask`
   - map nouns are forbidden in `crates/graphics` (engine rule 2 as a source regex)
   - mission_editing has no browser crates (engine rule 5)
6. Strangler rule: only apps, the two tool bins, and other legacy crates may depend on a `legacy/` crate.
7. Dev-dependencies are exempt from the tier order. They may point to `tools/foundation` (for `repository_laws` test helpers), never to apps or legacy.

**`verify crate-anatomy` (D2)**, for every library under `crates/` and `tools/`:
- `lib.rs` is at most 80 lines and holds only docs, attributes, `mod` and `pub use`.
- `pub mod prelude` exists.
- If any public signature returns `Result`, then `error.rs` holds a `thiserror` `pub enum Error` and `pub type Result<T, E = Error>` (Q7).
- No anyhow in `[dependencies]`. Exempt: apps, `xtask`, `developer_tools`, `staging_fixtures`.
- A README with a Contents block.
- `edition`, `rust-version` and `lints` come from the workspace, and every dependency is `workspace = true`.
- `[features]` may hold only `test_fixtures` or `failpoints`, each enabled only from `[dev-dependencies]`.
- No primitive-typed public `id`/`*_id` fields or parameters (generated crates exempt).
- No `pub use` of another workspace crate except inside `prelude.rs`. This is what kills re-export shims.
- The file-length and sibling-test laws apply, plus the law 8 module headers.

**Other new laws**
- `verify frontend-layering`: foundation ↛ features ↛ pages/workspaces ↛ the shell.
- `verify tailwind-sources`: every frontend crate has an `@source` line in `style/aegis.css`.
- `verify strangler`: the shim ledger is empty at commit (§D).

## C. Root-cause refactors

| What | Where (current) | Lands in / stage |
|---|---|---|
| Root discovery ×7 | `te/repository.rs:87`, `dt/repository_paths.rs:13`, `dt/browser_testing/server.rs:446` (20 uses; pulls in tokio), `xt/core/repository_root.rs`, `xt/.../shell_scripts.rs:109`, `xt/.../repository_access.rs:15`, `xt/mcp/call.rs:153`, `ticketboard/.../discovery.rs:17` (becomes `find_ticket_store`) | repository_layout, S4 |
| Layout modules ×4; silent root skip | `xt/core/repository_layout.rs`, `te/repository.rs:24-112`, `dt/repository_layout.rs`, `vc/.../source_roots.rs` | repository_layout; fail-closed in S0 |
| Process runner | vc `proc`, `xt/core/host_execution.rs:354`, 113 raw `Command::new`, te git ×9, dt ×9 | process_runner, S4 and S11 |
| Clocks ×5, timestamps ×2, SHA ×12, SplitMix64 ×2 + LCG | listed in §A | foundation crates, S4–S5 |
| Glyph packing ×2, colour normalisation ×3 | `overlay/.../glyph_math.rs:73-133` vs ge `text/pack.rs`, `scale.rs:19`; `revision.rs:15`, ge `geometry.rs:22`, `compose.rs:37` | render_primitives, S4 |
| Map centre ×4 | `scene.rs:34`, `INITIAL_TARGET`, `apply_faction/library.rs:11,14`, `probes/runner.rs:87` | map_coordinates, S4 |
| wgpu bootstrap ×2 | `frame/boot.rs:57-112`, `doll/lifecycle_1.rs:123-166` | gpu_device `GpuContext`, S8 |
| `evaluate_los` ×3, BVH builders ×2 (+dt) | `blueprint/attribution_1.rs:119`, `los/world/los.rs:24`, `walker.rs:420`; `bvh/traversal.rs:50`, `tlas.rs:51` | One occlusion primitive and one build core in spatial_indexes, pinned by the existing tests first. S6 |
| `RenderEngine` | 76 fields (27 of them counters) and impl blocks in 6 modules | `RenderStats`, `LaneSink`, typed layer fields, `FrameHook`, per-layer `new(&LayerContext)`. S8 |
| Dead JS exports | 148 `#[wasm_bindgen]` sites | Stripped in S8, with a JS-glue export-diff proof |
| Re-export shims ×13 and facades | 07 §5.11 (`dem/sample` is a reverse edge) | Each deleted at its crate's birth; `dem/sample` deleted in S4 |
| Fake splits | `bridge_1/2/3`, `lifecycle_1/2`, `coverage_1/2`, `frame_1/2`, `attribution_1/2`, `cases_1` | Merged, then re-split by responsibility. S6–S8 |
| Dead code | unused `earcutr`; `pick.rs:76` and `picking.rs:26` pass-throughs; `tools/placement`; the `gesture.rs:37` `EngineHandle` re-export; stale doc links; `png` in xtask; `SQLX_OFFLINE` (`ci.yml:85,89,111`); `mk rust-sqlx-prepare` (`shell_word.rs:143`); api's nested `rust-toolchain.toml` | S0, S2, S7 |
| Test oracles shipped in prod | `dda.rs:84` reference algorithm; agreement cases inside the solver | Tests (S6); `ballistics_agreement_cases` (S5) |
| Engine SCC back-edges | the 10 pairs in 01 §1 | Cut waves in S4/S6/S7/S8, per stage |
| API cycles (9 pairs); `AppState` holding 3 domain services | 05 §A2–A3 | Kernel crates and traits, S9 |
| Frontend inversions | api↔auth, auth↔ui (`gates.rs`), core→editor (`slider:19`, `select:15`, `search_box:18`, `store.rs:259`), core→root (`route_guard.rs:28`), pages→apps (`review_workspace`, `format_bytes`), page→page | `TokenProvider`, ui tokens, logout hooks, route table, workspace move, feature crate. S3 and S10 |
| Tool cycles | `commands::ci`↔`verifications::ci`; `core`→`build`; `documentation`→`crate::cli::Cli` (inject a `clap::Command` instead); platform↔ticket glob imports; `generate`→`gen_font_table`; te metrics↔cli and ops↔registry; dt `repository_layout`↔`repository_paths`; MCP split across two crates | S11 |
| Fleet wire shapes in 4 copies | 05 §A8 | fleet_wire_contract, S4 |
| Fixtures used across crates | `apps/website/shared`; `frontend/tests/fixtures/api` read by api tests | http_url_guard (S2); `contracts/fixtures/api_goldens` (S2) |
| Dependency drift | sha2 0.10/0.11, toml 0.8/1.1, png 0.17/0.18 | Unified as each crate is born (toml in S11) |
| Feature matrix | 8 features | Each feature deleted when the last module it gates leaves; gone in S8 |
| Objective kind dispatch spread over 11 `.c` files | mod `Gamemode/Objectives` | `TBD_ObjectiveKindBehaviour` plus Types/, mod lane M3 |

## D. Stage plan

**Strangler versus untangle-then-extract.** The strangler method wins:
- Every crate is born once, already meeting its standards.
- The tier law applies from the first birth; no temporary module-mode law is needed.
- Consumers leave the monolith early: the api in S5, developer_tools in S6.
- Each cut is local and sized to one crate's back-edges, not a crate-wide reshuffle that rewrites imports twice.

The cost is "cut waves": back-edges inside legacy must be severed before a bottom crate can be born (the invariant: a new crate never depends on legacy). `RenderEngine` needs an in-place redesign before any GPU crate exists.

**Shim lifecycle (inside one stage)**
1. **Birth waves.** The agent runs `relocate` to move files to the final path, refactors them to the anatomy, and leaves `pub use new_crate::…` shims at the old paths in legacy. It wires an optional legacy dependency to the existing feature and writes each shim into `<scratchpad>/shim_ledger.tsv`. Items widened from `pub(crate)` are listed for API review.
2. **Switch wave.** One scripted agent runs `relocate --rust-paths <ledger>` over the consumers and over legacy's own `crate::` paths, updates the manifests, and deletes every shim.
3. **Commit.** `verify strangler` requires an empty ledger and zero imports of legacy from new crates. So no shim ever crosses a commit.

**Brief additions**
- Export `CHROME_HEADLESS_SHELL=/opt/pw-browsers/chromium-1194/chrome-linux/chrome`.
- Parallel agents share one `target/`; the cargo lock serialises builds.
- Each agent owns only its own lines in the legacy `lib.rs`, `mod.rs` and `Cargo.toml` files.

**Standard gate set (GS)**, one command per call:
1. `fmt --check`
2. `clippy --workspace --all-targets -D warnings`
3. wasm32 clippy over the frontend and every W crate
4. `ci ci-local`
5. `db up`, then `db test-it`
6. `mk ci-local-leptos`
7. `mk leptos-gates`
8. `ci verify-documentation`
9. `ticket check`
10. `verify crate-tiers / crate-anatomy / strangler / frontend-layering / tailwind-sources`
11. `relocate --verify`
12. Dependency-drift probe: the set equals the baseline plus the stage's declared unifications.
13. Test-name census at or above the baseline, so moved tests are counted, not lost.
14. `cargo check -p api --release` and a wasm release check of the frontend, so dev-only features cannot mask production builds.
15. A wasm dist-size probe.

**Corrections to the first plan carried into this one (from 06)**
- The 139 primitive IDs are frontend DTO fields (141 including integer types). map-engine has 135, and api has 152 `Uuid` IDs, which `uuid_id!` covers.
- `range_slicing` is not shared with the frontend.
- `SQLX_OFFLINE` and `mk rust-sqlx-prepare` are vestigial.
- Chromium needs the env var above, plus a fix so `find_chromium` honours `PLAYWRIGHT_BROWSERS_PATH` and the `chrome-linux/` layout. trunk 0.21.14 and wasm-bindgen-cli 0.2.126 (the `gate-env.json` pins) must be installed. The 141-vs-149 Chrome version difference is only a warning.
- `png` in xtask is unused; `toml` there is test-only and becomes a dev-dependency.
- developer-tools does not depend on verification-core; its comment at `vc/Cargo.toml:1-3` is fixed.
- `mk leptos-gates` is an `mk` target only, not a `ci` task.

| Stage | Goal / agents (budget; what each owns) | Gate extras; checkpoints |
|---|---|---|
| **S0** Tooling | **T1** (L) `relocate`: `git mv`, path spellings, `../` re-relativised by anchor (file, manifest, cwd), `#[path]`, `include*!`, Cargo `path=`, markdown links, Rust path maps via a module walker; unresolvable literals fail `--apply`; frozen backticks skipped. **T2** (M) owns every `Cargo.toml`: `[workspace.package/dependencies/lints]` hoisting identical specs only (lockfile unchanged), the `png`/`toml` fixes, the vc comment. **T3** (M) `target/` consolidation (`cargo_target_directory.rs`, `wave_execution/mod.rs:277-292`, `changed_rs.rs:330`, reclaim globs, rsync, `.gitignore`), trunk and wasm-bindgen install, `find_chromium` fix. **T4** (L) the §B laws and fail-closed roots in vc, wired into the ci table, `ci.yml` and `ci-schema-parity`; ratchet mode where needed. **T5** (S) remove `SQLX_OFFLINE`, the `mk` target, `earcutr`. All five run in parallel. | Perturbation proof for each law and for `--verify`; baselines into the program record |
| **S1** Global renames | **R1** (M, alone, scripted): `assets`, `contracts`, `documentation` (with its mirror folders), `tools/{xtask, developer_tools, verification_core}`, `ticket-engine` → `legacy/ticket_engine`; `refactor_*` → `documentation/archive/refactor_v2/`; the improved_layout docs archived; 757 ticket `spec`/`plan` rewrites. Then **R2** (M) layout modules and xt literals (incl. `editor_orbat_coherency`) ∥ **R3** (S) `.gitattributes`, workflows, editorconfig, `gate-env`, codegen header ∥ **R4** (M) link-check retired spellings, READMEs, CLAUDE.md paths | LFS count equals baseline; `git lfs fsck`; `codegen-fresh` |
| **S2** Apps flattened, `deploy/` | **A1** (M, alone): `apps/{api, frontend, offline_service_worker}`, `legacy/{map_engine, graphics_engine}`, all packages snake_case, `deploy/` (Dockerfile, `compose.dev.yml`, `compose.staging.yml`, Caddyfile, `systemd/`, `deploy.env.example`), api toolchain file removed, api goldens to contracts. Then **A2** (M) xt deploy, staging, db (`cd apps/api`, seeds) and `staging_compose_paths` ∥ **A3** (M) `wave_execution` ∥ **A4** (S) Dockerfile, systemd, `.env.example`, runtime fallbacks (`http_router.rs:109-117`, `configuration`), Caddy mount ∥ **A5** (M) births: `http_url_guard`, `offline_cache_policy` | `docker build -f deploy/Dockerfile .`; `deploy website/staging --dry-run`. **OC-deploy:** move the ignored `.env`, `deploy.env`, `dist` and the server EnvironmentFile |
| **M1, M2** (after S2, before S4) | **M1** (S) `apps/mod/References/` (crf_framework, vanilla_reference, playable_selector) with every tool path; tools fail closed. **M2** (S) `Objectives/Engine/{Model,Registry,Runtime,Tasks}` plus the 7 pinned xt paths | **OC-mod:** move the ignored folders; Workbench `.rdb` regeneration, compile, world boot |
| **S3** Frontend inside its crate (D5) | **F1** (M, alone): `src/v2` → `src/{foundation, pages, workspaces}`, `api` → `transport`, tests and READMEs merged. Then **F2** (M) `src/shell/`, editor `shell` → `session` ∥ **F3** (S) `ui/tokens.rs`, `auth/logout_hooks.rs`, `foundation/routing` (route table) ∥ **F4** (S) `review_workspace` → `workspaces/editor/review_mode`, `format_bytes` → utils, `mission_review` → `features/` | `frontend-layering` becomes hard zero |
| **S4** Tier 0–1 everywhere | **B0** (S) cuts: scene calibration/stress instances → diagnostics, `dem/sample` deleted. Then in parallel: **B1** (L) tool foundations (ticket boundary test allows `repository_layout`) ∥ **B2** (M) the six foundation crates ∥ **B3** (M) geometry_primitives, map_coordinates, camera_math ∥ **B4** (M) render_primitives ∥ **B5** (S) world_file_formats ∥ **B6** (M) the two contract crates. Then **X4** (M, alone) switch. **B7** (S) retarget engine rules 1/2/6 to `crates/graphics` | `cargo tree -p xtask`: no duplicated sha2 |
| **S5** Mission and ballistics | **D1** (L) the five mission authoring crates with newtype IDs ∥ **D2** (M) the five ballistics crates. Then **D3** (L) crdt, document, formation geometry, the two operations crates. Then **X5** (M) switch. **D4** (S) retire rule 4 (and its pins) and the data half of rule 7; delete the `scenario`/`store` features; update the tripwire | `cargo tree -p api` has no map_engine, yrs, wgpu, rkyv or png |
| **S6** World CPU | **P0** (L, alone) cuts: `indexing/world` and `WorldResidency` placement, `footprint.rs` → buffers, lod constants, locations↔labels, `world/mesh` dissolved, `relief/host` and `*/loader` and satellite GPU moved render-side, roads edges, `los/terrain/overlay.rs`. Births by tier: **P1** (M) spatial_indexes ∥ **P2** (M) prefab and chunks ∥ **P3** (L) the 4 overlay crates ∥ **P4** (L) the 5 terrain crates; then **P5** (M) building_interiors ∥ **P6** (M) vegetation, place_names; then **P7** (L) the 3 LOS crates. **X6** (M) switch; developer_tools leaves legacy | Features `bvh`, `io` deleted |
| **S7** Streaming CPU and headless editor | **Q0** (M) cuts: the scheduler↔buffers↔loaders cycle, the `EngineHandle` re-export, the 3-layer forwarding chains, the four modules named "residency" renamed by meaning. **Q1** (M) chunk_scheduler, chunk_draw_buffers ∥ **Q2** (L) session, commands, persistence; then **Q3** (M) map_editing_tools. **X7** (M) switch | `editing` feature deleted; rule 5 now enforced by the tier law |
| **S8** Rendering | **V0** (L, alone, in place): strip `#[wasm_bindgen]` (proof: diff of the JS-glue export list, gates green), `RenderStats`, `LaneSink`, typed layers, `FrameHook`, `GpuContext`, diagnostics over accessors. **V1** (L) gpu_device, gpu_frame, renderer_core; legacy graphics_engine deleted. Then **V2** (M) symbology layers ∥ **V3** (M) the paper doll crates ∥ **V4** (L) map_asset_loading and map_streaming_host; then **V5** (M) world layers; then **V6** (L) map_renderer and diagnostics, which receive the draw-order and `wash_palette` source tests. **X8** (M) switch; legacy map_engine and its features deleted. **V7** (S) `engine_layers` deleted | **OC-web walkthrough:** online, and again with the API stopped behind the proxy (502) |
| **S9** API | **K0** (L, in place): kernel modules, `AppState` traits, `fail_point!` `$crate`, `architecture_rules.rs`. **K1** (M) infrastructure crates. **K2** (M) state and kernel. Then **K3a–h** (M each, parallel): the 8 domains. **K4** (M) workers, thin app, `staging_fixtures` moved, `engineering_laws.rs` | `db test-it` count at or above baseline; Docker build |
| **S10** Frontend | **H0** (L) editor untangle with a module-DAG ratchet. Merge rule: any pair still in a cycle after these cuts (state lift, `eden_chrome` re-exports deleted, `DOCK_BOTTOM_PX`, arsenal panels) is merged and recorded. **H0b** (S) `pins.rs` moved to its owners, `@source` lines. **H1** (L) foundation and feature crates. Then **H2a–g** (S/M) the 7 page crates ∥ **H3** (L) the 5 editor crates. **H4** (M) debug benches and thin app. **H5** (M) edition 2024 (Q3) | Walkthrough |
| **S11** Tools | **J0** (M) the tool cycles. **J1** (L) ticket crates; `legacy/ticket_engine` deleted; toml moves to 1.x. Then **J2a–f** (M each) the command and check libraries, two to three per agent ∥ **J3a–d** (M) the developer-tools libraries. **J4** (S) thin bins. anyhow becomes thiserror in every library | `cargo tree -p xtask` has no tokio, axum, reqwest or resvg; `legacy/` is empty |
| **S12** Close | **G1…** (closing-fix batches); records agent | Final sweep, perturbations, walkthrough |

**M3 (mod lane, any time after M2)**
- **M3a** (S): explorer catalogues the 11 switch sites.
- **M3b** (M): `Types/TBD_ObjectiveKindBehaviour.c`, `Types/{Capture,Destroy,HoldUntil}`, and a lookup; NONE maps to a no-op base.
- **M3c ∥ M3d** (M): M3c rewires Registry; M3d rewires Runtime, Text, `TBD_ZoneVolume.c` and `TBD_MissionWinConditionChecks.c`.
- Frozen names: `TBD_ObjectivesComponent`, `TBD_ObjectiveHud`.
- Gates: `enfusion-comments`, `file-length`, the xtask mod tests. Then OC-mod: a playtest of each kind.

**Documentation per stage (law 10)**
- Each birth agent writes its crate's README and module headers, and its section of `documentation/standards/crate_boundary_rules.md`, which replaces `engine_boundary_rules.md` section by section.
- The records agent updates `documentation/architecture/workspace_layout.md` (a living doc created in S1), the program record (execution record, shim ledger summary, widened-API list), and the CLAUDE.md atlas lines changed by S1–S3, S8 and S11. CLAUDE.md §1.6 and §2 are rewritten in S12.
- The blueprint is archived in S12.

## E. Gate evolution and end state

| Today | After |
|---|---|
| `engine_layers` rules 1–7 with pins | Rules 1, 3a, 3b, 4, 6 and 7 become the manifest firewalls of `crate-tiers`; rule 2 is a regex over `crates/graphics`; rule 5 is a category edge plus a regex. All deleted with legacy in S8 |
| `crate_dependencies.rs` forbidden list | The allowed-edge category matrix plus tier numbers |
| Silent `source_roots` skip | Fail-closed roots derived from the members (S0) |
| `feature_gate_tripwire` | The anatomy features rule; tripwire deleted in S8 |
| No standards gate | `crate-anatomy`, `strangler`, `frontend-layering`, `tailwind-sources` |
| `editor_orbat_coherency` (84 paths) | Rewritten by relocate at each move |

**End-state checks**
- 147 members, all passing tiers and anatomy; `legacy/` and `apps/website/` are gone.
- Only `building_interiors/test_fixtures` and `api_failpoints/failpoints` remain as features.
- `git grep` finds no `_v2`, `src/v2`, `crate::v2`, `website[-_]`, `tools_v2` or `apps/website` in live files.
- Each duplicate is defined exactly once: `ROOT_MARKER`, `SplitMix64`, `now_ms`, sha256, glyph packing.
- `#[wasm_bindgen]` appears only in the frontend app and `browser_platform`.
- The dependency-tree checks hold for api (no yrs, wgpu, rkyv, png, leptos), xtask (no tokio stack), and the frontend's wasm build (no sqlx, tokio).
- The test census is at or above baseline; 154 api test binaries.
- Docker build and both deploy dry-runs pass; `leptos-gates` and the walkthrough (Mission Creator, offline mortar, event slotting) pass.
- OC-mod: compile, world boot, and a playtest of capture, destroy and hold-until.

## F. Risks

| Risk | Mitigation |
|---|---|
| `pub(crate)` widening (frame 122, streaming 85, frontend 732, api 152) | Each birth reports its widened items; they are reviewed as API and go into the prelude |
| Newtype-ID ripple (map-engine 135, DTOs 141, api `Uuid` 152) | Serde-transparent newtypes keep the goldens byte-equal; `#[sqlx(transparent)]` on the api side; converted bottom-up |
| Edition 2024 changes temporary drop order inside Leptos/RefCell code | A separate wave (H5), gated by `leptos-gates` |
| Dev-only features unified under `--all-targets` | GS step 14 |
| Tailwind classes vanish when code moves to a new crate | The `@source` law |
| `include_str!` source tests across crates | Moved with their targets; relocate fixes the paths; caught by `cargo test` |
| Disk and lock contention from parallel agents | One shared target; prune between waves |
| anyhow→thiserror across ~380 tool files | M/L budgets in S11; bins keep anyhow |
| Gitignored state, LFS, `.rdb`, server paths | The S0–S2 gates plus OC-deploy and OC-mod |
| Length (13 stages, ~95 agents) | One green commit per stage; any stage after S4 is a safe resume point |

## Open questions

1. **Crate count is 147, against the 70–110 expected.** Recommended: accept it. The alternative is about −35: tool commands and checks into 6 crates, the 7 page crates into 3, the 3 LOS crates into 1, and the 2 mission operations crates into 1.
2. **Strip the unused `#[wasm_bindgen]` exports instead of adding a facade crate?** Recommended: strip them. A `map_engine_javascript_api` crate would be added only if the S8 export diff finds a JS caller.
3. **Bump the extracted frontend crates to edition 2024 (H5)?** Recommended: yes, in its own wave.
4. **Domain qualifiers in names** (`api_`, `frontend_`, `mission_creator_`). Recommended: allowed, because they describe the crate; D3 bans only organisation or project prefixes such as `tbd_` or `website_`.
5. **Park the dying crates in `legacy/` from S2?** Recommended: yes. `apps/website/` then dies in S2 and later prompts cite final paths.
6. **Allow `failpoints` as a second dev-only feature?** Recommended: yes.
7. **Crates with no fallible API skip `error.rs`?** Recommended: yes, rather than an uninhabited `Error` everywhere.

### Critical Files for Implementation
- /home/user/TBD-reforger/apps/website/map-engine/src/frame/engine.rs
- /home/user/TBD-reforger/apps/website/map-engine/src/frame/mod.rs
- /home/user/TBD-reforger/tools_v2/verification-core/src/repository_laws/source_roots.rs
- /home/user/TBD-reforger/tools_v2/verification-core/src/repository_laws/crate_dependencies.rs
- /home/user/TBD-reforger/tools_v2/xtask/src/core/repository_layout.rs
- /home/user/TBD-reforger/apps/website/api_v2/src/core/application_state.rs
