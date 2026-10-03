**Status:** live

# Target file tree

The exact end state of the repository after the program's last stage: 148 workspace members
(5 apps, 106 crates under the crates folder, 37 under the tools folder). The close stage diffs the
real tree against this document and fixes or records every difference. Crate details are in
[crate_catalogue.md](/documentation/restructure/crate_catalogue.md).

## Crate anatomy

Every crate folder, in every category, has this shape:

```text
<category>/<crate_name>/
├── Cargo.toml        package.name = "<crate_name>"; edition, rust-version, lints and every dependency
│                     taken from the workspace; [package.metadata.layout] category, tier, targets
├── README.md         Contents block, role and boundaries, per the README standard
├── src/
│   ├── lib.rs        at most 80 lines: module header, attributes, mod and pub use lines only
│   ├── prelude.rs    the types and traits callers import
│   ├── error.rs      thiserror Error and Result, when the public API is fallible
│   └── <modules>/    production files ≤ 500 lines; sibling tests/ folders, test files ≤ 1000 lines
└── tests/            integration tests, where the crate has them
```

`targets` is `any` or `wasm32`. A wasm-only crate gates its `lib.rs` on `wasm32` and keeps its
dependencies in target tables, so native workspace builds stay green.

## Repository tree

```text
TBD-reforger/
├── Cargo.toml                 workspace: member globs per category, [workspace.package],
│                              [workspace.dependencies], [workspace.lints]
├── Cargo.lock · rust-toolchain.toml (1.95.0 + wasm32) · README.md · CLAUDE.md · AGENTS.md
├── .cargo/config.toml         the xtask alias
├── .github/workflows/         ci.yml · contracts.yml · editor-gates.yml · mod-gates.yml · schema.yml
├── .ai/tickets/               ticket registry
├── .gitattributes · .gitignore · .editorconfig · .editorconfig-checker.json
├── .dockerignore              the API release image's build context: the workspace crates and the
│                              contract folders they embed
├── clippy.toml                the clippy settings every crate reads (tests may call unwrap())
├── target/                    all build output, one subfolder per purpose (gitignored)
│
├── apps/
│   ├── README.md
│   ├── api/                   Axum server app: router composition, binaries, integration tests
│   │   ├── Cargo.toml · README.md · .env.example
│   │   ├── src/               lib.rs · router.rs · bin/api.rs · bin/import_registry.rs
│   │   └── tests/             154 integration binaries and their support folders
│   ├── frontend/              Leptos client-side app, the Trunk entry
│   │   ├── Cargo.toml · README.md · Trunk.toml · index.html · manifest.webmanifest · service_worker.js
│   │   ├── style/             Tailwind v4; one @source line per frontend crate
│   │   └── src/               main.rs · app_routes.rs · shell/ (layout, sidebar, top_nav,
│   │                          membership_status, not_found)
│   ├── offline_service_worker/ the service-worker binary (main.rs and its modules, range slicing)
│   ├── fleet_host_agent/      agent binary; its wire shapes come from fleet_wire_contract
│   ├── ticketboard/           egui viewer over ticketboard_model
│   └── mod/
│       ├── README.md
│       ├── References/        gitignored crf_framework/, vanilla_reference/, playable_selector/;
│       │                      a tracked README.md
│       ├── tbd-framework/     Configs/ Data/ Missions/ Prefabs/ UI/ worlds/ addon.gproj resourceDatabase.rdb
│       │   └── Scripts/Game/TBD/   API/ Core/ Gamemode/ Session/ Systems/ UI/
│       │       └── Gamemode/Objectives/
│       │           ├── Engine/     Model/ Registry/ Runtime/ Tasks/
│       │           └── Types/      TBD_ObjectiveKindBehaviour.c · Capture/ · Destroy/ · HoldUntil/
│       ├── tbd-export/
│       └── tbd-emcp/
│
├── crates/
│   ├── README.md              category index and the tier DAG
│   ├── foundation/            newtype_ids · time_source · deterministic_random · content_digest
│   │                          http_url_guard · browser_platform
│   ├── contracts/             fleet_wire_contract · contract_schema_types · offline_cache_policy
│   ├── mission/               mission_wire_safety · mission_model · mission_payload · mission_validation
│   │                          mission_compiler · formation_geometry · mission_crdt · mission_document
│   │                          mission_operations
│   ├── ballistics/            ballistics_model · ballistics_solver · fire_mission_planning
│   │                          ballistics_calibration · ballistics_agreement_cases
│   ├── geometry/              geometry_primitives · map_coordinates · camera_math · spatial_indexes
│   ├── world_formats/         world_file_formats · prefab_catalog · world_chunks · world_store
│   ├── terrain/               terrain_elevation · terrain_relief · water_bodies · road_network
│   │                          satellite_imagery
│   ├── world_objects/         vegetation · place_names · building_interiors
│   ├── line_of_sight/         terrain_line_of_sight · interior_line_of_sight · world_line_of_sight
│   ├── map_overlay/           map_draw_lanes · label_layout · unit_symbology · overlay_instances
│   ├── streaming/             chunk_scheduler · chunk_draw_buffers · map_asset_loading
│   │                          map_streaming_host
│   ├── graphics/              render_primitives · gpu_device · gpu_frame · renderer_core
│   ├── map_rendering/         symbology_layers_gpu · world_layers_gpu · map_renderer
│   │                          map_render_diagnostics
│   ├── paper_doll/            paper_doll_scene · paper_doll_renderer
│   ├── mission_editing/       mission_editing_session · mission_editing_commands · mission_persistence
│   │                          map_editing_tools
│   ├── api/                   api_failpoints · api_foundation · api_configuration
│   │                          api_database (with migrations/ and seeds/) · api_audit_log
│   │                          api_mission_vocabulary · api_http_layer · api_discord · api_state
│   │                          api_caller_identity · api_member_activity · api_administration
│   │                          api_command_center · api_community_content · api_identity_and_access
│   │                          api_match_telemetry · api_missions · api_operations
│   │                          api_server_infrastructure · api_background_workers
│   └── frontend/
│       ├── foundation/        frontend_route_table · frontend_ui · frontend_api_dtos · frontend_transport
│       │                      frontend_session · frontend_offline · frontend_map_view
│       │                      frontend_test_support
│       ├── features/          mission_review_record
│       ├── pages/             administration_pages · operations_pages · mission_hub_pages · doctrine_pages
│       │                      field_tools_pages · command_center_pages · account_pages
│       └── workspaces/        mission_creator_state · mission_creator_engine_bridge
│                              mission_creator_session · mission_creator_arsenal
│                              mission_creator_workspace · debug_benches
│
├── tools/
│   ├── README.md
│   ├── xtask/                 binary: command line and dispatch only; keeps the data folders
│   │                          dedicated_server_profiles/, fixtures/mcp/ and staging/
│   ├── developer_tools/       one-line binaries: enf · gate · mcpd · world · map · capture ·
│   │                          acknowledgement_dropping_relay
│   ├── foundation/            repository_layout · process_runner · verification_core · repository_laws
│   ├── tickets/               ticket_model · ticket_metrics · ticket_registry · ticket_wave_lock
│   │                          ticketboard_model
│   ├── commands/              ci_task_catalog · database_operations · deployment · staging_procedures
│   │                          api_readiness_checks · mod_operations · platform_execution
│   │                          schema_tooling · enfusion_mcp · ballistics_oracle_tooling
│   │                          workstation_setup · remote_debugging
│   ├── checks/                documentation_checks · mod_script_checks · repository_checks
│   ├── enfusion/              enfusion_pak · enfusion_script_index
│   ├── map_assets/            blueprint_compiler · world_export_pipeline · map_raster_pipeline
│   │                          map_asset_verification
│   ├── browser_testing/       chrome_devtools_protocol · browser_gate_suites (with fixtures/dom_oracle/)
│   ├── staging/               staging_load_generator · acknowledgement_dropping_relay ·
│   │                          staging_fixtures (binary)
│   └── enfusion_mcp_node_package/   pinned npm package
│
├── deploy/
│   ├── README.md · Dockerfile · compose.dev.yml · compose.staging.yml · deploy.env.example
│   ├── caddy/                 Caddyfile: the one folder the staging Caddy container mounts
│   └── systemd/               the API service unit, the game server, host agent and relay unit
│                              templates, and the backup units and timers
│
├── assets/                    terrains/ (LFS) · glyphs/ · storage_spec/ · scratch/ (gitignored)
├── contracts/                 definitions/ · rules/ · catalogs/ · fixtures/ (including api_goldens/)
│
└── documentation/
    ├── README.md · product_roadmap.md
    ├── architecture/          README.md · workspace_layout.md (the living successor of the blueprint)
    ├── apps/                  feature docs mirroring apps/: api/ · frontend/ · fleet_host_agent/ ·
    │                          ticketboard/ · mod/
    ├── crates/                feature docs mirroring crates/<category>/<crate>/, where a crate needs
    │                          more than its README
    ├── tools/ · contracts/ · assets/    feature docs mirroring those trees
    ├── design_system/ · glossary/ · known_bugs/ · runbooks/
    ├── standards/             crate_boundary_rules.md replaces engine_boundary_rules.md
    ├── tickets/               frozen ticket specs and plans
    └── archive/               … plus refactor_v2/ · improved_layout/ · restructure/ ·
                               restructure_research/ (with the blueprint draft) ·
                               restructure_agent_briefs/ (each stage's executed agent briefs)
```

## The tree between stages

Each stage builds a part of this tree; until the close stage the live tree also holds what the
later stages have not yet dissolved. After S3 the live tree differs from the end state in these
places:

- `crates/` holds the first two library crates, both tier 0: `http_url_guard` in
  `crates/foundation/` (the one URL predicate the API and the single-page app link, with its case
  table as the `cases` module) and `offline_cache_policy` in `crates/contracts/` (the cache names,
  request classes, network fallback and offline pack list the service worker and the page share).
- `legacy/` parks the two engine crates, `legacy/map_engine/` and `legacy/graphics_engine/`
  (decision D12), until S8 deletes them; no crate under `crates/` depends on them.
- `apps/` holds the end-state app folders; the frontend's `src/` holds the `foundation/`,
  `features/`, `pages/`, `workspaces/` and `shell/` layers in one crate until S10 splits them into
  crates, and `apps/offline_service_worker/` is already the binary-only crate of the end state.
- `deploy/` is in its end state, `caddy/` included.
- `tools/` keeps its four single crates (`xtask`, `verification_core`, `ticket_engine`,
  `developer_tools`) until S4 and S11 split them.

## Documentation mirror rule

S1 updates the documentation standards: a feature doc lives at the documentation root plus its
code path without `src/`. Every relocation manifest pairs a code move with the move of its
documentation mirror. The old website, tools, contracts and assets doc folders therefore end up
under the apps, crates, tools, contracts and assets mirrors.

## What disappears

- The `_v2` folders: assets, contracts, documentation, tools, and the API folder.
- The website folder.
- The frontend's `src/v2` layer.
- The map engine, graphics engine and ticket engine crates, and the legacy parking folder.
- The map engine's nine Cargo features.
- Every root-level `target-*` and `dist-*` build folder.
- The xtask deploy folder.
- The API's nested toolchain and rustfmt files.
- The improved_layout anchor folders.
- The website's shared test-table folder.
