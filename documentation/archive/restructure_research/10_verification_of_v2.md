**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Adversarial verification of the second design

I checked all 12 items against the code. 3 pass and 9 are partial or fail. Four problems are structural and would block the stage plan as written:
- **Two crate cycles in the tables.** `world_chunks` ↔ `vegetation` (both T2), and `mission_entity_operations` ↔ `mission_document_operations` (both T6).
- **API cycles are not all cut as tabled.** `leaderboard_view` is missing from `api_member_activity`.
- **`chunk_scheduler` → `chunk_draw_buffers`.** Moving `footprint.rs` into the buffers crate leaves an upward edge.
- **Wrong tier for `mission_document`.** It is declared T5 on a dependency that only its tests use.

## Verdicts

| # | Check | Verdict | Evidence |
|---|---|---|---|
| 1 | 148 `#[wasm_bindgen]` sites are unused JS exports | **PARTIAL** | 143 are real attribute lines; the other 5 are mentions in comments (`data/store/crdt/undo_groups/clocks.rs:26`, `overlay/symbology/atlas/gpu.rs:14`, `frame/upload/text.rs:18`, `frame/pump.rs:18,20`). Real split: frame 45, overlay 43, diagnostics 15, camera 13, world 13, doll 12, spatial 2, data 0. The core claim holds. No JS, Trunk hook, tool or gate calls an engine method; the `window.__*` hooks are Rust closures (`fe/apps/editor/bridge/viewport.rs:105-118,184-191`, `arsenal/doll.rs:482`). Removing the attribute breaks nothing: `JsError`, `js_sys::Promise` and `future_to_promise` stay plain Rust types. Caveats below. |
| 2 | Mission chain direction | **PASS** (1 rider) | Every `flatten` import points down: `flatten/mod.rs:8-11` uses payload, kit, validate and wire_safety. `validator/mod.rs:6` uses only `compile::terrain_bounds`; `context.rs` and `cargo.rs` use wire_safety. Payload uses only ast and extensions (`payload/mod.rs:6`, `serialization.rs:177,185`). ast reaches only extensions; wire_safety and slot_line have no crate imports. Rider: `scenario/mod.rs:26` `COMPILER_PACKAGE_VERSION = "website-map-engine <ver>"` is stored in the DB (`api/missions/services/mission_artifacts/artifact_store.rs:170,198`). Re-homing it changes the stored string unless pinned. |
| 3 | Placement purity; ballistics split | **PASS** | `placement/*` imports only `super::`. Every ballistics edge points down: model ← solver ← planning ← agreement_cases, and calibration → model + solver. `battery.rs:12` → agreement_cases is a doc link only. No other `crate::` imports in ballistics. |
| 4 | Document vs operations | **FAIL** | Details below. |
| 5 | Engine CPU tiers | **FAIL** | Details below. |
| 6 | API cycles | **PARTIAL** | All 9 pairs confirmed. Two are not cut as tabled. Details below. |
| 7 | Frontend edges | **PARTIAL** | Production page→page edges: only administration → `mission_hub::mission_review` (7). Pages → apps: only `review_workspace` and `format_bytes`. Core → editor and core → router match the design; apps → pages and debug → editor are zero. Leftovers below. |
| 8 | Editor sub-crate DAG | **PARTIAL** (plausible with more cuts) | Counts below. |
| 9 | Tools | **PARTIAL** | Details below. |
| 10 | Anatomy law vs existing patterns | **PASS** (notes) | api domain `mod.rs` files are 7–11 lines with no fns. The generated crate's top level needs about 8 domain mods, with at most 23 per nested `mod.rs`, so it fits ≤ 80. The law hits the 79 `pub use website_*` re-exports (frontend 46, map-engine 29, api 4), which is intended. The codegen must also emit `prelude`, README and the other anatomy files for `contract_schema_types`. |
| 11 | Totals | **FAIL** (labelling) | Details below. |
| 12 | Spot-check of 25+ citations | **PASS** (minor slips) | Details below. |

### Check 1 caveats
- **`frame/boot.rs:442` is `#[wasm_bindgen(start)]`, not an export.** It installs the panic hook. Stripping it is safe because `fe/main.rs:17-23` sets the same hook. The S8 "JS-glue export diff" will show this start function vanish; that is expected.
- **The end-state claim is false.** §E says `#[wasm_bindgen]` will appear only in the frontend app and `browser_platform`. In fact:
  - `offline_service_worker` exports `on_install`, `on_activate` and `on_fetch`, which `frontend/service_worker.js` calls (`offline-service-worker/src/bin/offline_service_worker/lifecycle_events.rs`, `fetch_handling.rs`).
  - `fe/apps/editor/bridge/host_state/undo_grouped_gestures.rs:62` has a second `start` that would land in `mission_creator_engine_bridge`.
- **Minor:** `window.__mapAssets` is set by map-engine itself (`streaming/bridge/statistics.rs:209`), not by the frontend.

### Check 4: document vs operations
- **Rows are clean.** `rows/*` and `selection.rs` import only crdt (`rows/mod.rs:6-12`); there are no rows → operations edges. crdt does not import rows except in one test (`crdt/id_arrays/mission_doc_tests/mod.rs:8`). store does not import editing.
- **`mission_document` does not need the compiler in production.** Only the tests call `crate::data::scenario::compile::compile_payload` (`rows/tests/cases_5.rs:38…`). That function is in mission_payload, not mission_compiler. The two `RULE4_PIN` tests (`vc/repository_laws/engine_layers/rules.rs:163-174`) are also tests. So "Deps: crdt, compiler" at T5 breaks the design's own rule that tier = 1 + max dependency tier. The true tier is T2.
- **The two operations crates form a cycle.**
  - entity → others: `operations/entity/mod.rs:8-19` imports assets, attrs, compositions, document_index, faction_library, projections, rows and zones; `armed_placement.rs:20` imports cargo.
  - others → entity: `transform.rs:7`, `attrs.rs:6`, `document_index.rs:7` (`use super::entity::*`) and `cargo.rs:7`.
  - attrs, compositions, document_index, faction_library and cargo are all assigned to `mission_document_operations`.
- **Nine operations modules are unassigned:** `assets`, `cargo_rules`, `environment`, `projections`, `reassign`, `rotation`, `rows`, `slot_ids`, `zones`.
- **entity has unlisted deps:** mission_payload (`entity/selection.rs:18`, `clipboard.rs:49`). `tactical_graphics.rs` needs mission_model.

### Check 5: engine CPU tiers
- **`world_chunks` ↔ `vegetation` cycle.** `streaming/loaders/store.rs:19-24` imports `vegetation::regions`, `roads::network` and `roads::airfield`. `vegetation/canopy.rs:7` imports `loaders::chunk::WorldChunk`. Stage S6 also births chunks (P2) before vegetation (P6).
- **`chunk_scheduler` → `chunk_draw_buffers` upward edges remain:**
  - `scheduler/viewport.rs:6` uses `buffers::packer::deinterleave`.
  - `scheduler/viewport.rs:11` uses `footprint::building_visible`. The design moves `footprint.rs` into the buffers crate, which makes this worse.
  - `scheduler/residency/mod.rs:36` re-exports `BUILDING_MIN_ZOOM`.
- **road_network ↔ place_names:** `roads/network.rs:13` → `locations::route_placement::road_class_name`, while `route_placement.rs` → `roads::network`. P0 only says "roads edges" without naming this cut.
- **Claimed edges that are wrong but harmless:**
  - `terrain_relief` really needs elevation plus map_coordinates (`camera::math::shaping::round`). Its claimed draw_lanes and render_primitives deps are used only by `host.rs`.
  - `water/vectors.rs` imports only io, not relief. `water/mesh.rs` (→ sea_band and triangulate) is unassigned.
  - `road_network` also needs elevation (`airfield.rs` → `dem::grid`), render_primitives (`mesh_from_tri`) and map_coordinates (`chunk_math`).
  - `paper_doll_renderer` depends on `doll/scene` and camera_math, not "gpu_device only". T3 is still right.
- **Edges that hold as claimed:**
  - `label_layout`: `text_packing.rs:16-19,69` and `text_metrics.rs:16` import locations. The "locations↔labels" cut covers this.
  - World LOS → `loaders::chunk`, interior walker, compound, bvh and `chunk_math`.
  - Interior LOS → `terrain::viewshed`.
  - Terrain LOS without `overlay.rs` → dem only.

### Check 6: API cycles
- **`leaderboard_view` must join `api_member_activity`.**
  - `identity_and_access/services/identity_linking.rs:7-9` imports `command_center::leaderboard_view::refresh_leaderboard_on_connection`.
  - `match_telemetry/services/match_results_ingest.rs` imports it too.
  - `user_stats.rs:3` itself needs `leaderboard_view::refresh_leaderboard`.
  - If it stays put, identity ↔ command_center (`load_user`) survives, and so does match_telemetry → command_center → operations (`Event`) → match_telemetry (`member_service_record.rs:50`).
  - With it moved, operations → match_telemetry is one-way, because match_telemetry → operations is only `participation_attribution`, which moves.
- **Machine credentials:** `ExecutorKind` is defined at `server_infrastructure/models/machine_credential.rs:19`. The service (`services/machine_credentials.rs:17`) calls `runtime_sessions::end_sessions_of_credential`. Moving the whole service drags runtime_sessions along.
- **`session_authorization`** drags `account_authority`, `session_issuance::arma_id_is_linked` and `UserRole` along with it.
- **Every other pair is cut once the kernels exist:**
  - administration ↔ identity: what remains is administration → identity only (UserRole, grace, resync).
  - administration ↔ operations: cut by reevaluation_queue moving.
  - missions ↔ operations and missions ↔ server_infrastructure: cut by the vocabulary crate.
  - identity ↔ server_infrastructure: cut by caller_identity.
- **Remaining cycles once leaderboard_view moves: none.**
- **Tier numbers:** `api_member_activity` T8 and `api_caller_identity` T7 are not 1 + max dependency tier.

### Check 7: frontend leftovers
- `pages/administration/ballistics_catalogs/tests/ballistics_catalogs.rs:325` uses `pages::navigation::nav_config::NAVIGATION`, which moves to the app shell.
- `app_routes.rs:29` and `main.rs:22` use navigation; that is fine.
- `core/test_support` (cfg(test), 665 lines) is used by editor (104), pages (34) and core (13). Only `pins.rs` is assigned. `fixtures.rs`, `editor_operations.rs` and `class_r_scrub` need a shared dev-dependency crate.
- `apps/aar` and `apps/planner` contain only READMEs.

### Check 8: editor sub-crate DAG
Production back-edges against state < bridge < session < arsenal < workspace:

| Edge | Refs | Symbols |
|---|---|---|
| bridge → arsenal | 10 | `PlacePayload`, `CatalogState`, `CatalogNode`, `build_catalog_tree`, `rules::{CargoRow, CompatFeed}` |
| bridge → mission_editor | 8 | `map_render_slot_soa`, `transform`, `selectable_ids`, marker/comment lane fns, `AssetPickerState` |
| bridge → shell | 7 | `review_mode::writes_mission`, `world_layer_prefs::{load_prefs, load_basemap_view}`, `persist::schedule_edit_persist`, `hydrate`, `eden_chrome` |
| bridge → ui | 15 | outliner builders and tree fns, `OutlinerNode`, `DrawTarget`, `SeamRegistration`, `m_per_px`, `dock_right::{record_placed, marker_icon_is_authorable}` |
| input → shell | 8 | layout px consts |
| input → ui | 3 | `context_menu::{open, resolve_target}`, `install_seam` |
| input → mission_editor | 2 | `WidgetVariant`, `plain_paste_anchor` |
| shell → ui | 17 | mostly the `eden_chrome` re-exports and `DOCK_BOTTOM_PX` (both cut by the design); real residue is `exports.rs:56-59` `publish_compile_findings` |
| shell → mission_editor | 4 | `ConflictInfo` |
| arsenal → ui | 1 | `ui::arsenal::panels` (design moves these) |

- **Covered by the named H0 cuts and the state lift:** about 40 refs (outliner and inspector models, toolbelt, layout tokens, eden_chrome, panels).
- **Still unnamed: about 38 refs.**
  - Types can lift: `CatalogState`, `PlacePayload`, `CatalogNode`, `CargoRow`, `CompatFeed`, `AssetPickerState`, `ConflictInfo`, `WidgetVariant`, review_mode, world_layer_prefs.
  - Behaviour cannot lift: `schedule_edit_persist`, `hydrate`, `build_catalog_tree`, `map_render_slot_soa`, `context_menu::open`, `publish_compile_findings`, `record_placed`. These need callback injection or relocation.

### Check 9: tools
- **staging → api_readiness:** 39 references (not 36). api_readiness → core only, so the edge points down. deploy → staging is a doc link only (`deploy/staging/payloads.rs:10`).
- **db → deploy:** the 12 refs all go to `deploy/database_*` (2.06k lines, self-contained). The design's sizes (database_operations 3.5 ≈ db + verifications/database 3.3k; deployment 5.5 ≈ 7.2k − 2.06k) leave those 2.06k unassigned.
- **documentation_checks:** `command_citations.rs:32,62` needs `Cli::command()`. Injecting a `clap::Command` is feasible, but `verify_link_check` is also called from `commands/ci/task_definitions/verification_dispatch.rs:79-86`, so the command tree must be threaded through `ci_task_catalog` as well. Tests `tests/gate_scope.rs` and `tests/link_check.rs` use `TopCmd` and must move to the xtask bin.
- **Tickets:** metrics → cli is just `commit_subjects::SubjectCommit` (goes to ticket_model). sync ↔ validation stays inside ticket_registry. validation → metrics (6) means ticket_metrics must sit below ticket_registry.

### Check 11: totals
- The tables hold 140 crates: foundation 9, mission/ballistics 15, engine CPU 28, graphics 12, api 20, frontend 21, tools 35.
- Adding xtask and developer_tools gives 142, but both are binaries, and so is `staging_fixtures`. The real figure is 139 libraries + 3 tool binaries + 5 apps = 147.
- The summary says 9 tool families; the table has 8.
- S4 B2 "the six foundation crates": only 5 are left after S2 (newtype_ids, time_source, deterministic_random, content_digest, browser_platform).
- No crate appears in the stages without a table row, or the reverse.

### Check 12: citations
**Correct:** `frame/mod.rs:148`, `doll.rs:35,117`, `gpu.rs:11`, `viewshed_scheduler/host.rs:89`, `viewport.rs:80`, `ingest.rs:59,92`, `agreement_cases.rs:81`, `slot_edits.rs:31`, `sha256_digest.rs:32`, `tlas.rs:51`, `dda.rs:84`, `boot.rs:57,458`, `statistics.rs:313`, `scene.rs:34`, `library.rs:11`, `runner.rs:87`, `node.rs:25`, `traversal.rs:50`, `walker.rs:420`, `attribution_1.rs:119`, `los.rs:24`, `glyph_math.rs:73` vs `scale.rs:19`, `revision.rs:15`, `url_guard.rs:66`, `http_url_guard.rs:62`, `route_guard.rs:28`, `member_service_record.rs:50`, `schema_types.rs:23`, `ci.yml:85,89,111`, `shell_word.rs:143`, `host_execution.rs:354`, `te/repository.rs:87`, `dt/repository_paths.rs:13`, `server.rs:446`, slider/select/search_box/store lines, `fleet_command.rs:40`.

**Slips:**
- `xt/mcp/call.rs` is really `xt/commands/mcp/call.rs:153`.
- `doll/lifecycle_1.rs` is really `doll/renderer/lifecycle_1.rs`.
- There are 10 `include!` sites of `is_http_url_cases.rs`, not 9.
- `earcutr` is unused only in map-engine; graphics-engine uses it (`draw/triangulate.rs:129`), so T5 must touch only map-engine's manifest.
- api has 154 integration binaries, which is correct.

## Required corrections
1. **Merge the two mission operations crates into one `mission_operations` (T3).** It depends on document, payload, model, formation_geometry and map_coordinates, and absorbs the 9 unassigned modules. The alternative is to cut `terrain_bounds_of`, `selected_slot_ids` and `document_index`'s `entity::*` into document, which is more work.
2. **Fix `mission_document`:** deps = crdt (plus newtype_ids), T2. mission_payload and mission_compiler become dev-dependencies for the round-trip and RULE4 tests. Move `crdt/id_arrays/mission_doc_tests` into it. Re-tier the editing crates downstream.
3. **Break vegetation ↔ world_chunks.** Move `streaming/loaders/store.rs` (the world store over chunks, roads and regions) out of world_chunks into chunk_scheduler or a `world_store` crate at T3+. Then world_chunks = {chunk, chunk_bin, manifest, residency} at T2, and vegetation and roads sit above it. Reorder P2 and P6 to match.
4. **Fix the scheduler's buffer edges in Q0.** Move `building_visible` (`footprint.rs:46`) and `BUILDING_MIN_ZOOM` into map_draw_lanes (lod). Move `deinterleave` (`packer.rs:20`) to map_coordinates or the scheduler. Delete the `residency/mod.rs:36` re-export.
5. **Name the roads cut in P0:** move `road_class_name` from `route_placement` into road_network. Correct the tabled deps:
   - road_network: elevation, render_primitives, map_coordinates
   - terrain_relief: map_coordinates
   - water: decide which crate owns `water/mesh.rs`
6. **API:** list `command_center/services/leaderboard_view` in `api_member_activity`. Have `api_caller_identity` take only `MachineCaller`, `authenticate_machine` and the `ExecutorKind` model (`machine_credential.rs:19`, with its contract annotation); issue/list/revoke stay in server_infrastructure because they need runtime_sessions. Also list `account_authority`, `arma_id_is_linked` and `UserRole` as moving with session_authorization. Recompute the kernel tiers.
7. **Rewrite the §E wasm_bindgen end-state check:** allow `offline_service_worker`, and either move the `undo_grouped_gestures.rs:62` start into the app (via time_source) or allow it. Correct the counts to 143 attribute lines, noting the `(start)` at `boot.rs:442`.
8. **Pin `COMPILER_PACKAGE_VERSION`** as a literal (or treat changing it as a deliberate contract change) when mission_compiler is born.
9. **Frontend:** add a `frontend_test_support` dev-only crate for `fixtures`, `editor_operations` and `class_r_scrub`. Put `NAVIGATION` in `frontend_route_table`, or move the ballistics_catalogs test into the app.
10. **Editor (H0):** add the ~38 unnamed back-edge refs from Check 8 to the ratchet list. Lift the arsenal catalog and rules model, `ConflictInfo`, `WidgetVariant`, `AssetPickerState` and review_mode/world_layer_prefs into `mission_creator_state`. Inject `schedule_edit_persist`, `hydrate`, `context_menu::open`, `publish_compile_findings` and `record_placed` as callbacks; otherwise the merge rule triggers.
11. **Tools:** move `deploy/database_*` (2.06k) into `database_operations`, with deployment → database_operations for the `DeployCmd::Db` dispatch. Thread the CLI tree through `ci_task_catalog` and move the `TopCmd` tests to the xtask bin. State that ticket_metrics sits below ticket_registry.
12. **Labels:** "139 libraries + 3 tool binaries + 5 apps = 147"; 8 tool families; "five foundation crates" in S4 B2.
13. **Citation fixes:** `xt/commands/mcp/call.rs:153`, `doll/renderer/lifecycle_1.rs`, 10 `include!` sites, and `earcutr` removed from map-engine only.
