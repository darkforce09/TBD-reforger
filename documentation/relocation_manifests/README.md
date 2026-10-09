**Status:** live

# Relocation manifests

Every manifest `cargo xtask refactor relocate` has run: each one lists the moves and reference
rewrites of one stage or topic, so no path is ever rewritten by hand. The folder is the permanent
retired-spelling registry: every manifest stays committed here after it ran, and
`cargo xtask refactor relocate --verify` reads them all, oldest first, to keep proving that no
live file spells a path a manifest retired.

## Contents

```text
documentation/relocation_manifests/
├── example.tsv               the commented format sample the tests run; never judged as a stage manifest
├── m2_objectives_engine.tsv  stage M2: the mod's four objectives engine folders into Objectives/Engine/
├── m3_objective_types.tsv    stage M3: the destroy target search beside the destroy objective behaviour
├── s10_w02a_h0a_editor_lifts.tsv  stage S10 H0a: the Mission Creator's pure state lifts into workspaces/editor/state/; the Arsenal panels join the Arsenal, the conflict dialog the session, the source-pin test support the editor
├── s10_w04_1_frontend_test_support.tsv  stage S10 H1a, first of four: the frontend's test support becomes the dev-only frontend_test_support crate
├── s10_w04_2_frontend_ui.tsv  stage S10 H1a, second of four: the frontend's interface primitives and helpers become frontend_ui, every module at the crate root
├── s10_w04_3_frontend_api_dtos.tsv  stage S10 H1a, third of four: the frontend's wire types and typed identifiers become frontend_api_dtos
├── s10_w04_4_frontend_transport.tsv  stage S10 H1a, fourth of four: the frontend's HTTP client, endpoint calls and event streams become frontend_transport
├── s10_w04_5_frontend_route_table.tsv  stage S10 H1b, fifth foundation crate: the frontend's route table and the sidebar's menu become frontend_route_table
├── s10_w04_6_frontend_map_view.tsv  stage S10 H1b, sixth foundation crate: the frontend's shared map seam becomes frontend_map_view, its native math tested on every target
├── s10_w04_7_frontend_session.tsv  stage S10 H1b, seventh foundation crate: the frontend's session store, refresh, sign-out hooks, route guard and gates become frontend_session
├── s10_w04_8_frontend_offline.tsv  stage S10 H1b, eighth foundation crate: the frontend's offline worker registration, pack download, saved copies and pack status become frontend_offline
├── s10_w04_9_mission_review_record.tsv  stage S10 H1b, the feature crate: the mission review record becomes mission_review_record; the app's foundation and features folders close
├── s10_w05_1_mission_creator_state.tsv  stage S10 H3a, first Mission Creator crate: the editor's pure state vocabulary becomes mission_creator_state
├── s10_w05_2_mission_creator_engine_bridge.tsv  stage S10 H3a, second Mission Creator crate: the editor's bridge and input layer become mission_creator_engine_bridge
├── s10_w05_3_mission_creator_session.tsv  stage S10 H3a, third Mission Creator crate: the editor's browser session becomes mission_creator_session
├── s10_w05_administration_pages.tsv  stage S10 H2a, a page crate: the event manager, approvals, server control, personnel, content manager, audit logs and ballistics catalogs pages become administration_pages, their feature docs its documentation mirror
├── s10_w05_debug_benches.tsv  stage S10 H3c, a workspace crate: the building viewer, world line-of-sight, equipment data viewer and ballistics agreement benches become debug_benches, their feature docs its documentation mirror
├── s10_w05_mission_hub_pages.tsv  stage S10 H2c, a page crate: the mission library, the mission overview and the New Mission dialog become mission_hub_pages, their feature docs its documentation mirror
├── s10_w05_operations_pages.tsv  stage S10 H2b, a page crate: the event schedule, event hub, ORBAT selection, deployments and leaderboards pages become operations_pages, their feature docs its documentation mirror
├── s10_w06_1_mission_creator_arsenal.tsv  stage S10 H3b, fourth Mission Creator crate: the editor's Arsenal becomes mission_creator_arsenal, its feature docs its documentation mirror
├── s10_w06_2_mission_creator_workspace.tsv  stage S10 H3d, fifth Mission Creator crate: the editor page, docks, outliner, inspectors, dialogs, review workspace and tests become mission_creator_workspace, the editor docs its documentation mirror
├── s10_w06_account_pages.tsv  stage S10 H2g, a page crate: the sign-in, sign-in callback and account settings pages become account_pages, their feature doc its documentation mirror
├── s10_w06_command_center_pages.tsv  stage S10 H2f, a page crate: the dashboard, server intel and announcements pages become command_center_pages, their feature docs its documentation mirror
├── s10_w06_doctrine_pages.tsv  stage S10 H2d, a page crate: the doctrine wiki, vehicle database and modpacks pages become doctrine_pages, their feature docs its documentation mirror
├── s10_w06_field_tools_pages.tsv  stage S10 H2e, a page crate: the mortar calculator with its map picker, saved fire missions and offline line becomes field_tools_pages, its feature docs its documentation mirror
├── s11_w1_j0a_xtask_core.tsv  stage S11 J0a: xtask's core folder empties into deploy_settings, tool_test_support, process_runner, repository_layout and the ci group
├── s11_w1_j0b_developer_tools_cuts.tsv  stage S11 J0b: developer_tools' layout module keeps the map pipelines' paths; xtask imports the shared ones from repository_layout
├── s11_w1_j0b_staging_load_plan.tsv  stage S11 J0b: the staging load's plan and report types into the tokio-free load_plan module
├── s11_w1_j1_ticket_crates.tsv  stage S11 J1: the ticket engine dissolves into ticket_model, ticket_metrics, ticket_wave_lock and ticket_registry
├── s11_w2_j1b_ticketboard_model.tsv  stage S11 J1b: the ticketboard's models, services, process helpers and egui-free application state become ticketboard_model
├── s11_w2_j2a_check_crates.tsv  stage S11 J2a: the architecture, language-ban, licensing and registry checks and the tooling tests become repository_checks; the mod script checks become mod_script_checks
├── s11_w2_j2b_schema_and_ballistics.tsv  stage S11 J2b: the contract codegen, the schema gates and the flattening become schema_tooling; the ballistics trim becomes ballistics_oracle_tooling
├── s11_w2_j2c_readiness_and_setup.tsv  stage S11 J2c: the API readiness verification and its property-test seed become api_readiness_checks; the setup group becomes workstation_setup
├── s11_w2_j2d_enfusion_mcp.tsv  stage S11 J2d: xtask's mcp group and developer_tools' server entrypoint become enfusion_mcp; the mcpd broker becomes enfusion_mcp_broker
├── s11_w2_j2e_repository_relocation.tsv  stage S11 J2e: the relocation tool leaves xtask as the repository_relocation crate under tools/commands
├── s11_w3_j2f_database_and_deployment.tsv  stage S11 J2f: the db group, the deploy-side database verbs and the database checks become database_operations; the deploy group and the compose-path check become deployment
├── s11_w4a_j2g1_staging_procedures.tsv  stage S11 J2g1: xtask's staging acceptance harness becomes staging_procedures under tools/commands
├── s11_w4a_j2g2_remote_debugging.tsv  stage S11 J2g2: xtask's debug and repro groups and the remote log verdict become remote_debugging under tools/commands
├── s11_w4a_j2h1_documentation_checks.tsv  stage S11 J2h1: the readme-coverage, markdown-placement and link-check gates become documentation_checks under tools/checks
├── s11_w4b_j2h2_ci_task_catalog.tsv  stage S11 J2h2: xtask's ci and mk lanes, the cargo target pin, the CI workflow checks and the map asset checks become ci_task_catalog under tools/commands
├── s11_w5_j2i_platform_execution.tsv  stage S11 J2i: xtask's platform group (the wave driver, slice runs, slice worktrees, the preflight) becomes platform_execution under tools/commands
├── s11_w6_j2j_mod_operations.tsv  stage S11 J2j: xtask's mod group (the compile gate, world boot, playtest server, equipment export, website API client, mod wave driver) becomes mod_operations under tools/commands
├── s11_w7a_j3f_ballistics_gate_suites.tsv  stage S11 J3f: the ballistics agreement and offline mortar gates and the `gate` and `capture` command lines leave developer_tools for browser_gate_suites
├── s11_w3_j3a_enfusion_crates.tsv  stage S11 J3a: developer_tools' pak reader becomes enfusion_pak; its script oracle and xtask's vanilla page mirrors become enfusion_script_index
├── s11_w3_j3b_browser_testing.tsv  stage S11 J3b: developer_tools' DevTools protocol client becomes chrome_devtools_protocol; its gate suites, DOM oracle fixtures and gate pin become browser_gate_suites
├── s11_w3_j3c_staging_tools.tsv  stage S11 J3c: developer_tools' staging verification engines become staging_load_plan, staging_load_generator and acknowledgement_dropping_relay under tools/staging
├── s11_w7a_j3d1_blueprint_compiler.tsv  stage S11 J3d1: developer_tools' blueprint module and its test fixtures become blueprint_compiler under tools/map_assets
├── s11_w7a_j3e1_world_export_pipeline.tsv  stage S11 J3e1: developer_tools' world-export module becomes world_export_pipeline under tools/map_assets; its census kind test follows INSTANCE_KINDS into prefab_catalog
├── s11_w7b_j3d2_map_asset_verification.tsv  stage S11 J3d2: developer_tools' map verification becomes map_asset_verification under tools/map_assets; the CI task catalogue's map asset checks and xtask's world line-of-sight adapter call it
├── s11_w7b_j3e2_map_raster_pipeline.tsv  stage S11 J3e2: developer_tools' map raster module and its decision-record locations become map_raster_pipeline under tools/map_assets
├── s11_w8_g-s11b_xtask_map_logic.tsv  stage S11 G-S11b: xtask's terrain export driver and map tile index writer become world_export_pipeline modules; the map group keeps only its command line and dispatch
├── s12_w01_relocation_manifests.tsv  stage S12 G1: this folder leaves the restructure program's folder to stay live as the retired-spelling registry
├── s12_w02_blueprint_compiler_names.tsv  stage S12 G6: the blueprint compiler's files and modules named for their job; `bvh` becomes `occlusion_sidecars`
├── s12_w02_map_asset_verification_names.tsv  stage S12 G6: the map asset gates' JSON-helper files named for their gates and loaders; test files take `_tests`
├── s12_w02_map_raster_pipeline_names.tsv  stage S12 G6: the map raster pipeline's files named for their job, `sap` spelled out as supertexture; test files flattened
├── s12_w02_repository_root.tsv  stage S12 G7: the checkout-root walk, its error and its tests leave repository_layout for the foundation crate repository_root
├── s12_w02_tool_test_checkout_root.tsv  stage S12 G7: tool_test_support's checkout-root module becomes test_checkout_root, no longer spelling the crate name repository_root
├── s12_w03_agent_context_crate.tsv  stage S12 G13: xtask's agent context guards become agent_context_guards under tools/commands; the `ai` group keeps only its command line and dispatch
├── s12_w03_mod_documentation_mirror.tsv  stage S12 G8: the mod's documentation mirror moves under documentation/apps/, at the mod's code path like every other application's
├── s12_w04_restructure_program_archive.tsv  stage S12 G10b: the closed workspace restructure program's records move into the archive as one topic folder
├── s13_w01_1_api_server.tsv  stage S13 W1, first of four: the API application becomes the api_server crate under crates/api, its binaries api-server and import-item-registry, its documentation mirror under documentation/crates/api/
├── s13_w01_2_frontend_shell.tsv  stage S13 W1, second of four: the single-page app becomes the frontend_application crate and the offline service worker joins it in the new shell layer crates/frontend/shell; the app's documentation hub becomes its mirror, the planned workspaces join the workspace crate documentation
├── s13_w01_3_game_server_host_agent.tsv  stage S13 W1, third of four: the host agent becomes the game_server_host_agent crate in the new category crates/fleet, its documentation mirror under documentation/crates/fleet/, its systemd template game_server_host_agent@.service
├── s13_w01_4_ticketboard_desktop.tsv  stage S13 W1, fourth of four: the ticketboard becomes the ticketboard_desktop tool crate under tools/tickets, its package, binary and eframe app id ticketboard_desktop, its documentation mirror under documentation/tools/tickets/
├── s1_global_renames.tsv     stage S1: top-level folder and tool package renames, archived records
├── s2_apps_and_deploy.tsv    stage S2: website crates to apps/ and legacy/, snake_case packages, deploy/
├── s2_brief_archive.tsv      stage S2: the executed S1 agent briefs into the archive
├── s2_caddy_folder.tsv       stage S2: the Caddyfile into deploy/caddy/, the one folder the Caddy container mounts
├── s2_crate_births.tsv       stage S2: the API's URL guard and the worker's cache policy become crates/
├── s3_byte_formatting.tsv    stage S3: the byte-count formatter's consumers, from the Mission Creator to foundation utilities
├── s3_foundation_cycles.tsv  stage S3: the role ladder, the single flight into transport; the content gates into auth
├── s3_frontend_layers.tsv    stage S3: the frontend's src/v2 into foundation, features, pages, workspaces, shell
├── s3_outliner_module.tsv    stage S3: the Editor Layers outliner's node model, from outliner::outliner to node_model
├── s4_b0_dem_sample_tests.tsv  stage S4 B0: the dem/sample re-export module's tests beside the code they test
├── s4_b1a_tool_foundations.tsv  stage S4 B1a: verification_core splits into the tools/foundation crates verification_core, process_runner and repository_laws
├── s4_b2b_browser_platform.tsv  stage S4 B2b: the map engine's console macros and fetch helpers become browser_platform
├── s4_b2b_content_digest.tsv  stage S4 B2b: developer_tools' SHA-384 helper becomes the content_digest crate
├── s4_b3_geometry.tsv  stage S4 B3: vector, segment, transform and box helpers, map coordinates and the cameras become the geometry crates
├── s4_b4_render_primitives.tsv  stage S4 B4: the graphics engine's GPU-free layouts, geometry, glyphs and shader become render_primitives
├── s4_b5_world_file_formats.tsv  stage S4 B5: the map engine's on-disk formats become the world_file_formats crate
├── s4_b6b_contract_schema_types.tsv  stage S4 B6b: the API's importers of its generated contract types switch to the contract_schema_types crate
├── s4_x4a_switch.tsv  stage S4 X4a: every consumer of the S4a crates imports them directly, and the legacy re-export shims go
├── s5_ballistics.tsv  stage S5 D2, second of three: the ballistics files into the five ballistics crates
├── s5_ballistics_folders.tsv  stage S5 D2, first of three: the four ballistics module folders and the ballistics docs
├── s5_ballistics_paths.tsv  stage S5 D2, third of three: the Rust paths of the moved ballistics code and its consumers
├── s5_formation_geometry.tsv  stage S5 D3a, first of three: the placement point math of the arrange commands becomes formation_geometry
├── s5_mission_crdt.tsv  stage S5 D3a, second of three: the document's id arrays, slot columns and undo clocks become mission_crdt
├── s5_mission_document.tsv  stage S5 D3a, third of three: the document's rows, selection policy and whole-document tests become mission_document
├── s5_mission_model_payload.tsv  stage S5: the compiled rows, ORBAT, authored blocks and slot line become mission_model; the payload compiler and kit aliases become mission_payload
├── s5_mission_operations.tsv  stage S5 D3b: the document operations become mission_operations; the store's re-export pins become its prelude tests
├── s5_mission_validation_compiler.tsv  stage S5: the validator becomes mission_validation; the game-document compiler and its editor-input structs become mission_compiler
├── s5_mission_wire_safety.tsv  stage S5: the map engine's wire-safety scans become crates/mission/mission_wire_safety
├── s5_switch.tsv  stage S5 X5: every consumer of the mission crates imports them directly, and the legacy data and placement re-export shims go
├── s6_p0c_cuts.tsv          stage S6 P0c: chunk ingest, object index, footprint buffers, bind pins move
├── s6_p1_spatial_prefab_chunks.tsv  stage S6 P1: the BVH, point indexes, prefab catalogue and world chunks become spatial_indexes, prefab_catalog and world_chunks
├── s6_p2_overlay.tsv  stage S6 P2: the overlay's lanes, zoom gates, labels, symbology and instance packers become the map_overlay crates
├── s6_p3_satellite_crate_root.tsv  stage S6 P3: the satellite container reader's module root and tests take their crate names
├── s6_p3_terrain.tsv  stage S6 P3: the elevation model, relief, satellite container reader and water data become the terrain crates
├── s6_p4_roads_vegetation_interiors.tsv  stage S6 P4: the road network, vegetation data and building interiors become road_network, vegetation and building_interiors
├── s6_p5_place_names_store.tsv  stage S6 P5: the spot heights, town and road names and the headless world store become place_names and world_store
├── s6_p6_line_of_sight.tsv  stage S6 P6: line of sight over the elevation model, inside one building and through the streamed world becomes terrain_line_of_sight, interior_line_of_sight and world_line_of_sight
├── s6_x6_switch.tsv  stage S6 X6: every consumer of the S6 crates imports them directly, and the legacy re-export shims go
├── s7_q0a_streaming_cut.tsv  stage S7 Q0a: the layer toggles and residency statistics into the draw buffers; the residency tests beside the composed owner, under subject names
├── s7_q1_streaming.tsv  stage S7 Q1: the chunk scheduler and the draw buffers become the chunk_scheduler and chunk_draw_buffers crates
├── s7_q2a_operation_suites.tsv  stage S7 Q2a: the map engine's three document integration suites become mission_operations integration suites
├── s7_q2a_session_persistence.tsv  stage S7 Q2a: the editing host, undo, grouping, routing, selection, picks and lanes become mission_editing_session; local draft decisions become mission_persistence
├── s7_q2b_commands.tsv  stage S7 Q2b: the hosted editing commands and the pure export, report and selection texts become mission_editing_commands
├── s7_q3_tools.tsv  stage S7 Q3: the selection, ruler, line-of-sight and viewshed scheduler tools, with the source scrub their guards read, become map_editing_tools
├── s7_x7a_switch.tsv  stage S7 X7a: every consumer of the S7 streaming and mission editing crates imports them directly, and the map engine's re-export shims go
├── s7_x7b_documentation.tsv  stage S7 X7b, first of two: the editing layer and draft persistence feature docs into the mirror of the mission editing crates
├── s7_x7b_engine_layer_results.tsv  stage S7 X7b, second of two: with engine rule 5 retired, the engine-layer report results tests take that subject's name
├── s8_w00_v0a_frontend_pin.tsv  stage S8 V0a: the source pin over the frontend's document history host leaves the map engine for the frontend's bridge tests
├── s8_w00_v1_gpu_crates.tsv  stage S8 V1: the graphics engine's device code becomes gpu_device and its frame, draw, pipeline and loop code gpu_frame; its documentation moves to the graphics crates' mirror
├── s8_w01_v0s_streaming_model.tsv  stage S8 V0s: the streaming preferences, boot progress and the pure memory budget with its tests become map_streaming_model
├── s8_w01_v1c_renderer_core.tsv  stage S8 V1c: the render engine's statistics report leaves the diagnostics bench for the frame; the GPU frame's building pipeline module becomes oriented_quad
├── s8_w01_v3_paper_doll.tsv  stage S8 V3: the map engine's doll becomes paper_doll_scene (scene, picking) and paper_doll_renderer (renderer, shader, readback self-check)
├── s8_w02_v4_streaming_crates.tsv  stage S8 V4: the map engine's loaders, live memory budget, asset statistics and world loaders become map_asset_loading; its host becomes map_streaming_host; the streaming doc moves to the streaming crates' mirror
├── s8_w03_v2_symbology_layers.tsv  stage S8 V2: the map engine's lane preferences, glyph atlas layer, icon lane cull, icon uniform layout, world icon lanes and slot symbology become symbology_layers_gpu
├── s8_w04_v5_world_layers.tsv  stage S8 V5: the map engine's textured lane record, building layer, forest layer, terrain texture layer and terrain line of sight overlay become world_layers_gpu
├── s8_w05_v6a_map_renderer.tsv  stage S8 V6a: the map engine's frame and camera viewport become map_renderer (engine, boot, frame path, statistics, diagnostic views, asset sink, lane sinks, typed layer doors, upload belts, tests); the map engine overview becomes the map rendering overview
├── s8_w06_v6b_render_diagnostics.tsv  stage S8 V6b: the map engine's diagnostics become map_render_diagnostics (readback checks with the calibration check beside them, scene readback, frame benchmark, stress pool, stress scene); the frontend's browser hooks import them from the crate
├── s8_w07_x8_switch.tsv  stage S8 X8: the single-page app imports the map renderer, the GPU frame pump and the streaming crates directly, and the map engine's re-export shims go with both legacy crates
├── s8_w09_x8b_crate_boundary_rules.tsv  stage S8 X8b: the boundary rules standard, which holds the crate-level laws, takes the name crate_boundary_rules.md
├── s9_w01_k0a_kernel_cuts.tsv  stage S9 K0a: the audit writers, member activity and caller identity files into the API's kernel staging folders
├── s9_w02_k0b_kernel_cuts.tsv  stage S9 K0b: the Discord clients and the equipment datasets into the API's kernel staging folders
├── s9_w05_1_api_foundation.tsv  stage S9 K1a, first of five: the handler error, wire formats, text and request primitives become api_foundation
├── s9_w05_2_api_configuration.tsv  stage S9 K1a, second of five: the configuration and the shutdown signal become api_configuration
├── s9_w05_3_api_failpoints.tsv  stage S9 K1a, third of five: the fault injection becomes api_failpoints
├── s9_w05_4_api_database.tsv  stage S9 K1a, fourth of five: the connection lifecycle, migrations and seeds become api_database
├── s9_w05_5_api_property_evidence.tsv  stage S9 K1a, fifth of five: the property run recorder becomes the dev-only api_property_evidence
├── s9_w06_k1b_http_layer.tsv  stage S9 K1b: the access tokens, middleware, observability, realtime hub and outbound retry become api_http_layer
├── s9_w07_1_api_audit_log.tsv  stage S9 K2a, first of four: the audit severity and the audit line appends become api_audit_log
├── s9_w07_2_api_mission_vocabulary.tsv  stage S9 K2a, second of four: the terrain and game mode enums become api_mission_vocabulary
├── s9_w07_3_api_discord.tsv  stage S9 K2a, third of four: the Discord OAuth2, guild-member and webhook clients become api_discord
├── s9_w07_4_api_equipment_datasets.tsv  stage S9 K2a, fourth of four: the equipment dataset imports, index and read queries become api_equipment_datasets
├── s9_w08_1_api_caller_identity.tsv  stage S9 K2b, first of three: the role ladder, the session and account authority, the identity locks and the machine caller become api_caller_identity
├── s9_w08_2_api_member_activity.tsv  stage S9 K2b, second of three: the member statistics, the leaderboard refresh, the attendance attribution and the re-evaluation queue become api_member_activity
├── s9_w08_3_api_state.tsv  stage S9 K2b, third of three: the application state and its sub-state projections become api_state
├── s9_w09_community_content.tsv  stage S9 K3a: the announcements, the wiki, the vehicle database, modpacks, uploads and the equipment data viewer routes become api_community_content
├── s9_w09_identity_and_access.tsv  stage S9 K3b: Discord sign-in, session tokens, the caller's profile, the Arma link handshake and Discord membership become api_identity_and_access
├── s9_w10_administration.tsv  stage S9 K3c: the member roster and its moderation, the Discord role resync, the membership grace extension and the audit log console become api_administration
├── s9_w10_server_infrastructure.tsv  stage S9 K3d: the server registry, the live status feed, the machine credentials, the fleet command ledger and the runtime sessions become api_server_infrastructure
├── s9_w11_match_telemetry.tsv  stage S9 K3e: the session-fenced heartbeat, the match registration, the results revisions and the detailed event batches become api_match_telemetry
├── s9_w11_missions.tsv  stage S9 K3f: the mission library, versions, artifacts, reviews and approvals, deployments, the armory, factions and registries become api_missions
├── s9_w12_operations.tsv  stage S9 K3g: the event calendar and its access control, ORBAT slotting and reservations, service records, leave requests, fire missions and ballistics catalogs become api_operations
├── s9_w13_command_center.tsv  stage S9 K3h: the members' dashboard with its fleet overview, the community leaderboards and the per-player statistics card become api_command_center
├── s9_w14_k4a_thin_app.tsv  stage S9 K4a: the background workers become api_background_workers, and the API application becomes the thin app (the router at `src/router.rs`, no `core/`)
└── s9_w14_k4b_staging_fixtures.tsv  stage S9 K4b: the API's `staging-fixtures` host tool and its four integration suites become the staging_fixtures bin crate under tools/staging
```

## How it works

### What a manifest is

A manifest is the input of one relocation run: a list of rows, each a `path` move or a reference
rewrite (`rust_path`, `text`). The tool applies every row of one manifest in one run, moves with
`git mv` and rewrites every reference to what moved, and refuses the whole manifest when anything
is unresolved. Once committed, a manifest is a record: its `path` rows' `from` columns are the
retired spellings the verification keeps out of every live file, and its `rust_path` rows' prefixes
the ones it keeps out of their scopes.

### Format

A manifest is a UTF-8, tab-separated `.tsv` file. Blank lines and lines starting with `#` are
skipped; the first other line is the header `kind`, `from`, `to`, `scope` (tab-separated).
Every row has three or four columns, and a manifest with any invalid row is refused as a whole,
every error named with its line. Paths are repository-relative, `/`-separated, and name the tree
as it stands before the manifest's moves.

| Kind | `from` and `to` | `scope` | What it does |
|---|---|---|---|
| `path` | a tracked file or folder, and where it goes | empty | `git mv` (parent folders created), then rewrites every reference to it in every tracked text file |
| `rust_path` | a Rust path prefix ending in `::`, such as `crate::v2::core::` | a folder whose `.rs` files are rewritten; empty means every tracked `.rs` | rewrites the prefix in `use` trees, code, attributes, doc links, comments and string literals |
| `text` | an identifier-like token, such as a package name | a folder, a glob holding `*` or `?`, or empty for every live text file | rewrites the token where neither neighbour is a letter, a digit, `_` or `-` |

`example.tsv` is the commented format sample with one row of each kind; the relocation tests run
it on a throwaway checkout, and `--verify` never judges it.

### File names

A manifest is named `<stage or topic>_w<NN>_<subject>.tsv`: the stage or topic that ran it, the
two-digit wave it ran in, and what it moves, in lowercase snake_case (for example
`s12_w01_relocation_manifests.tsv`). When one stage lands several manifests in one commit, their
names sort in the order they were applied, since manifests one commit adds are ordered by name.
Manifests from before this rule keep their names (`s1_global_renames.tsv`, `s11_w2_j1b_…`).

### What a run rewrites

A `path` row rewrites repository-root spellings (`from/…`, the `/from/…` of repository-root Markdown
links, and `from` behind a deployment prefix in strings, TOML, YAML, systemd units, `.gitattributes`
and `.gitignore`) and relative references (`include_str!` and its kin, `#[path]`, literals built on
`CARGO_MANIFEST_DIR`, Cargo `path = "…"`, Markdown link destinations and any `./` or `../` token). A
relative reference is read from the file's folder, the owning crate's folder or the repository root,
and rewritten so it names the moved target from the same anchor; one that cannot be re-read, or that
two anchors read differently, makes `--apply` refuse the whole manifest with its `path:line` before
anything is written. A moved file's owning crate (the anchor of its `CARGO_MANIFEST_DIR` joins) is
the nearest folder holding a `Cargo.toml` in the tree the manifest leaves, counting a `Cargo.toml` the
same manifest moves there and an untracked one already on disk, so a crate being born takes its
files' joins; where no folder below the repository root holds one (the root manifest is the
workspace's), the join is unresolved, never re-anchored at the root. A climbing token `<seg>/../…` is a relative reference only under an anchor
where its named lead (the segments before its first `..`) is a tracked folder, so a datum such as `"7/../.."` is left as
written and never makes an apply refuse. A token of `.` and `..` segments alone (`../`, `./`) is a
reference only where its syntax fixes the anchor (a link destination, an `include!` or `#[path]`
argument, a Cargo `path` value): in prose, a comment or a plain string literal it speaks of a parent
folder in general and is never rewritten. A literal whose syntax does not fix its anchor (a token in
a Rust literal or comment, a climbing token in prose) is rewritten only when its spelling pins it to
one reading; one every crate spells for its own files (`src/lib.rs`), a fixture path relative to a
temporary checkout (`../../engine/map` in a test's synthetic `Cargo.toml`) or an example path
whose tail names nothing stays as written and is listed as ambiguous with its `path:line`, for
review, without stopping the run. When a moved module's code reaches outside the moved subtree
through `self::` or `super::` chains, a `rust_path` row turns those chains into absolute `crate::`
paths first. Rows of one manifest compose: a path moves by the longest `from` that contains it, and
each row lands exactly at its `to` in any manifest order, so folder rows may share a destination
parent (one folder becomes `crate/src`, two others `crate/src/ortho` and `crate/src/orbit`). Rows
that would put two things in one place are refused by the dry run with both lines: two rows with the
same `to`, a `to` inside its own `from`, a file landing where another row's moved files already lie,
and rows no order of moves can make (two folders that swap names).

Frozen records change as little as their checks need: Markdown under the archive and the ticket
documents gets only its link destinations rewritten, prose and backticks staying as history; a
`README.md` there is a live index of its folder, so the tree part of its Contents block's lines (the
root folder and each entry's name, not the roles) is rewritten and verified as a live file is; a
ticket record whose status is shipped or cancelled gets only its `spec`, `plan` and `owns` entries
rewritten. The relocation tool's own test sources
(`tools/commands/repository_relocation/src/tests/`) spell paths of throwaway checkouts, so only their
code follows a manifest, never a string literal or a comment. A file takes the treatment of the place it lands, so a file moved into the archive is
frozen from that move on.
Binary files and Git LFS pointers move with their folders and are never edited. SQL migrations (a
`.sql` file directly in a `migrations` folder, whose checksum `sqlx` pins once a database applies
it) move byte-identical and are never edited or verified.

### Adding a manifest

Write `<stage or topic>_w<NN>_<subject>.tsv` here with a `#` comment saying what it moves and why,
then run it in three steps and commit it with the moves it made. It is never edited afterwards; a
correction is a new manifest.

1. `cargo xtask refactor relocate --manifest <path.tsv> --dry-run` prints, per row, the tracked
   files moved and the references rewritten by file kind, then every unresolved literal and every
   ambiguous literal left as written, and runs the verification below over the tree the plan
   would leave; it writes nothing and exits 1 on any unresolved literal or finding (ambiguous
   literals are for review and do not fail it).
2. `cargo xtask refactor relocate --manifest <path.tsv> --apply` refuses, writing nothing, unless
   that dry run passes; otherwise it makes the moves and rewrites and then verifies the manifest.
   A failure at any step undoes every step, leaving the index and the working tree byte-identical
   to before.
3. `cargo xtask refactor relocate --verify` judges every manifest in this folder except
   `example.tsv`, in the order below: no live tracked file spells a `path` row's retired `from` on segment boundaries
   (frozen records and the manifests themselves excluded), and no `rust_path` prefix is left in its
   scope. Exit 0 pass, 1 findings, 2 did not run.

### The registry and its order

Committed manifests are never edited, so `--verify` judges an earlier manifest's `rust_path`
scope where the later manifests left it. The manifests are ordered by the first commit of the
checkout's history that added each one under any path it has had: one `git log --topo-order
--reverse --find-renames=100% --diff-filter=AR` over every `.tsv` path follows each exact rename
back to the file's first addition, and one `git diff --cached` against `HEAD` does the same for a
move staged but not committed yet, so moving this folder leaves the order unchanged. Manifests one
commit added sort by file name, and manifests no commit added yet (untracked, or only added to the
index) come last, by file name; a shallow clone is a did-not-run. Each scope, relocated by its own
manifest's `path` rows, then follows every later manifest's `path` rows in that order: a scope a
row moved, or moved with a parent, is judged at its new folder; a scope that is gone because a row
took files out of it, its own manifest's rows file by file or a later manifest's, holds nothing to
judge, so its row holds and the output prints a `note:` line naming the first such row; a scope
gone with no row to explain it is still a did-not-run. A manifest named with `--manifest` composes
the same way when it is a stage manifest of this folder; a dry run and an apply judge their own
manifest alone, its own rows composed.
`text` rows rewrite at apply time only and are not judged by `--verify`.

## Code

- [Repository relocation](/tools/commands/repository_relocation/) — the parser, the passes, the moves
  and the verification that read these files; its constant `MANIFESTS_BELOW_DOCUMENTATION` names
  this folder.

## Boundaries

- Depends on: the relocation tool's manifest parser, which defines the format above.
- Used by: `cargo xtask refactor relocate --verify`, which judges every stage manifest here; the
  relocation tests, which run `example.tsv` on a throwaway checkout.
- Rules: `example.tsv` stays the format sample with one row of each kind
  (`relocate_example_manifest_parses_and_applies`); a committed manifest is never edited, since its
  rows are the retired spellings the verification keeps out of the tree; the order survives a move
  of this folder (`relocate_manifest_order_survives_a_move_of_the_manifests_folder`).

## Related documentation

- [Documentation standards](/documentation/standards/documentation_standards.md) — the mirror moves
  a relocation manifest carries for the documentation tree.
- [Path coupling research](/documentation/archive/restructure_research/02_path_coupling.md) —
  every kind of path reference a move breaks.
