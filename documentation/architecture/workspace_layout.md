**Status:** live

# Workspace layout

How the repository is laid out: the top-level folders, the members of the Cargo workspace, and
where code, contracts, assets, documents and agent configuration live.

## Where it lives

- Code: the root [`Cargo.toml`](/Cargo.toml) (the workspace members),
  [`rust-toolchain.toml`](/rust-toolchain.toml) (Rust 1.95.0 with rustfmt, clippy and the
  `wasm32-unknown-unknown` target) and [`.cargo/config.toml`](/.cargo/config.toml), which makes
  `cargo xtask` run the `xtask` package.
- Entry: the [root README](/README.md) for a first look, the
  [directory atlas](/CLAUDE.md#2-monorepo-directory-atlas) for the folders one level deeper.

## Top-level folders

```text
TBD-reforger/
├── apps/            the products: API, single-page app, service worker, Enfusion mod suite,
│                    fleet host agent, ticketboard
├── crates/          the tiered library crates, grouped by category (foundation/, contracts/, geometry/, world_formats/, streaming/, map_rendering/, graphics/, api/, …)
├── deploy/          the release Dockerfile, compose files, Caddy site (caddy/), deploy settings,
│                    systemd units
├── tools/           the developer tools: xtask, developer_tools, the crates by category
│                    (foundation/, tickets/, commands/, checks/, enfusion/, browser_testing/,
│                    staging/, map_assets/), and the pinned Enfusion MCP npm package
├── contracts/       JSON Schemas, rules, catalogs and fixtures of every shape that crosses a boundary
├── assets/          terrain datasets (Git LFS), the world-object glyph set, the storage specification
├── documentation/   every document: feature docs, runbooks, standards, glossary, tickets, archive
├── .ai/             the ticket registry (tickets/) and agent artifacts of past waves
├── .github/         the five GitHub Actions workflows
├── .cursor/         the Cursor agent rules
├── .claude/         the Claude Code project settings
└── .cargo/          the cargo alias for xtask
```

The root files are the workspace manifest and lockfile, the toolchain pin, `clippy.toml`, the
build context of the API's release image (`.dockerignore`), the editor and checker settings (`.editorconfig`, `.editorconfig-checker.json`), the Git LFS patterns
(`.gitattributes`), the ignore rules (`.gitignore`), the per-mission warning budget of the mod
world-boot gate (`.world-boot-warning-baseline`), the root `README.md`, and `CLAUDE.md` with
`AGENTS.md` as a symlink to it. Build output is never tracked: `target/` and every `target-*/`
folder are ignored.

## Workspace members

One Cargo workspace (resolver 3) holds every Rust crate: 161 members, which are 5 apps under
`apps/`, 113 library crates under `crates/`, 41 tool crates under `tools/<category>/` and the two
tool binaries `tools/xtask` and `tools/developer_tools`. The crate-tier law judges the 154 crates
and leaves out only the apps and the two binaries. Members inherit edition 2024 and rust-version
1.95 from `[workspace.package]`.

| Folder | Package | What it is |
|---|---|---|
| [`apps/api/`](/apps/api/README.md) | `api` | the Axum and sqlx REST API and SSE hub: the thin application (router, composition root, the `api` server and the `import-registry` tool) the 23 crates under `crates/api/` are assembled into, and its 150 integration binaries |
| [`apps/frontend/`](/apps/frontend/README.md) | `frontend` | the Leptos single-page app, compiled to WebAssembly and served by Trunk: the thin app (entry point, route rendering, the platform frame in `shell/`, the stylesheet) the 22 crates under `crates/frontend/` are assembled into |
| [`apps/offline_service_worker/`](/apps/offline_service_worker/README.md) | `offline_service_worker` | the WebAssembly service worker behind offline packs |
| [`apps/fleet_host_agent/`](/apps/fleet_host_agent/README.md) | `fleet_host_agent` | the agent beside each game-server instance that carries out fleet commands |
| [`apps/ticketboard/`](/apps/ticketboard/README.md) | `ticketboard` | the egui desktop viewer of the ticket registry; its headless models are `ticketboard_model` in `tools/tickets/` |
| [`crates/foundation/http_url_guard/`](/crates/foundation/http_url_guard/README.md) | `http_url_guard` | the HTTP(S) URL check the API and the single-page app share |
| [`crates/contracts/offline_cache_policy/`](/crates/contracts/offline_cache_policy/README.md) | `offline_cache_policy` | the offline cache names, request classes and fallback rules the service worker applies |
| [`crates/foundation/newtype_ids/`](/crates/foundation/newtype_ids/README.md) | `newtype_ids` | macros declaring serde-transparent typed ids |
| [`crates/foundation/time_source/`](/crates/foundation/time_source/README.md) | `time_source` | wall-clock and monotonic time sources, RFC 3339 UTC formatting and validation |
| [`crates/foundation/deterministic_random/`](/crates/foundation/deterministic_random/README.md) | `deterministic_random` | the seeded SplitMix64 generator |
| [`crates/foundation/content_digest/`](/crates/foundation/content_digest/README.md) | `content_digest` | SHA-256 and SHA-384 hex digests and framed hashing |
| [`crates/foundation/browser_platform/`](/crates/foundation/browser_platform/README.md) | `browser_platform` | browser console macros and fetch helpers (wasm32 only) |
| [`crates/foundation/repository_root/`](/crates/foundation/repository_root/README.md) | `repository_root` | the one checkout-root finder: the walk up to the `.ai/tickets/ROOT` marker the tools and the tests share |
| [`crates/foundation/orbat_slot_ids/`](/crates/foundation/orbat_slot_ids/README.md) | `orbat_slot_ids` | an ORBAT slot's two ids: `SlotUid`, the durable editor id, and `SlotId`, the derived wire id |
| [`crates/geometry/geometry_primitives/`](/crates/geometry/geometry_primitives/README.md) | `geometry_primitives` | vector ops, segment geometry, rigid transforms, axis-aligned boxes |
| [`crates/geometry/grid_rasterization/`](/crates/geometry/grid_rasterization/README.md) | `grid_rasterization` | half-up rounding, the uniform Catmull-Rom spline, polygon scanline spans and anti-aliased disc stamps over a sample grid |
| [`crates/geometry/map_coordinates/`](/crates/geometry/map_coordinates/README.md) | `map_coordinates` | terrain frames, chunk math, rounding, grid references |
| [`crates/geometry/camera_math/`](/crates/geometry/camera_math/README.md) | `camera_math` | the orthographic map camera, the orbit camera, 4x4 matrices |
| [`crates/geometry/spatial_indexes/`](/crates/geometry/spatial_indexes/README.md) | `spatial_indexes` | the triangle BVH and its `.bvh` sidecar, the flat-tree build core, the point grid, picks and clusters |
| [`crates/world_formats/world_file_formats/`](/crates/world_formats/world_file_formats/README.md) | `world_file_formats` | the on-disk world formats and their typed ids |
| [`crates/world_formats/prefab_catalog/`](/crates/world_formats/prefab_catalog/README.md) | `prefab_catalog` | prefab rows, render classes, footprint lookups, prefab tables and world payload decoding |
| [`crates/world_formats/world_chunks/`](/crates/world_formats/world_chunks/README.md) | `world_chunks` | chunk JSON and TBDC container decoding, chunk ids and the terrain manifest |
| [`crates/world_formats/world_store/`](/crates/world_formats/world_store/README.md) | `world_store` | the headless world store: manifest, prefab table, roads, regions, one chunk at a time |
| [`crates/terrain/terrain_elevation/`](/crates/terrain/terrain_elevation/README.md) | `terrain_elevation` | raster placement, PNG and raw grid decoding, bilinear sampling and the vector grid |
| [`crates/terrain/terrain_relief/`](/crates/terrain/terrain_relief/README.md) | `terrain_relief` | the hillshade image, contour rings with summit picks and the sea band fills |
| [`crates/terrain/satellite_imagery/`](/crates/terrain/satellite_imagery/README.md) | `satellite_imagery` | the `.tbd-sat` container reader: header, both index versions, checks and level picks |
| [`crates/terrain/road_network/`](/crates/terrain/road_network/README.md) | `road_network` | road segments and the class codec, styling and zoom gates, the export image styles, road meshes, cartographic strips, the airfield |
| [`crates/terrain/water_bodies/`](/crates/terrain/water_bodies/README.md) | `water_bodies` | the bathymetry water mask and palette, the level suffix plan, the inland water archive and the sea fill mesh |
| [`crates/world_objects/vegetation/`](/crates/world_objects/vegetation/README.md) | `vegetation` | forest regions, the canopy mass outline, tree counts and island density bins |
| [`crates/world_objects/building_interiors/`](/crates/world_objects/building_interiors/README.md) | `building_interiors` | building blueprints and sight-line attribution, compounds with doors, section cuts |
| [`crates/world_objects/place_names/`](/crates/world_objects/place_names/README.md) | `place_names` | spot heights, town and road names, their declutter, the labels archive and glyph packing |
| [`crates/line_of_sight/terrain_line_of_sight/`](/crates/line_of_sight/terrain_line_of_sight/README.md) | `terrain_line_of_sight` | elevation profiles along a sight line and viewsheds, whole or a ray at a time |
| [`crates/line_of_sight/interior_line_of_sight/`](/crates/line_of_sight/interior_line_of_sight/README.md) | `interior_line_of_sight` | compound traces, the one sight-line evaluation, floor washes whole or in batches |
| [`crates/line_of_sight/world_line_of_sight/`](/crates/line_of_sight/world_line_of_sight/README.md) | `world_line_of_sight` | the world occluder: chunk box trees, the prefab occluder library, verdicts with coverage |
| [`crates/map_overlay/map_draw_lanes/`](/crates/map_overlay/map_draw_lanes/README.md) | `map_draw_lanes` | the 48 lane roles, their paint order and wire ids, and the zoom gates |
| [`crates/map_overlay/label_layout/`](/crates/map_overlay/label_layout/README.md) | `label_layout` | label declutter, town importance, world glyph sizing and label glyph packing |
| [`crates/map_overlay/unit_symbology/`](/crates/map_overlay/unit_symbology/README.md) | `unit_symbology` | side tints, role and vehicle classes, the symbol atlas, markers and squad links |
| [`crates/map_overlay/overlay_instances/`](/crates/map_overlay/overlay_instances/README.md) | `overlay_instances` | slot, vehicle, comment and cluster icon instances and fire-mission marks |
| [`crates/graphics/render_primitives/`](/crates/graphics/render_primitives/README.md) | `render_primitives` | map-agnostic CPU rendering primitives: instances, geometry, triangulation, cull oracle, text, the WGSL shader |
| [`crates/graphics/gpu_device/`](/crates/graphics/gpu_device/README.md) | `gpu_device` | the GPU context of a canvas (create, resize, acquire), pooled lane buffers, readback guards, the frame timer (wasm32) |
| [`crates/graphics/gpu_frame/`](/crates/graphics/gpu_frame/README.md) | `gpu_frame` | the frame vocabulary, draw encoding, compute sprite cull, render pipelines and the animation-frame pump (wasm32) |
| [`crates/graphics/renderer_core/`](/crates/graphics/renderer_core/README.md) | `renderer_core` | the renderer contracts: lane sink, layer context, frame hooks, render statistics and their JSON, packet binding ids (wasm32) |
| [`crates/streaming/chunk_scheduler/`](/crates/streaming/chunk_scheduler/README.md) | `chunk_scheduler` | the viewport pin, in-flight marks, LRU eviction, chunk and prefab ingest, the object index and rebuild requests |
| [`crates/streaming/chunk_draw_buffers/`](/crates/streaming/chunk_draw_buffers/README.md) | `chunk_draw_buffers` | the draw set, glyph, strip and footprint buffers, layer toggles and the world residency owner |
| [`crates/streaming/map_streaming_model/`](/crates/streaming/map_streaming_model/README.md) | `map_streaming_model` | world layer and host preferences, boot progress, the memory budget model and the `MapAssetSink` contract |
| [`crates/streaming/map_asset_loading/`](/crates/streaming/map_asset_loading/README.md) | `map_asset_loading` | the browser loaders (world, occluder, terrain, satellite, forest, labels), mesh composition, the live memory budget and asset statistics (wasm32) |
| [`crates/streaming/map_streaming_host/`](/crates/streaming/map_streaming_host/README.md) | `map_streaming_host` | the browser map host: terrain and world boot, camera settle, view preferences, map queries (wasm32) |
| [`crates/map_rendering/symbology_layers_gpu/`](/crates/map_rendering/symbology_layers_gpu/README.md) | `symbology_layers_gpu` | the slot symbology, glyph atlas, icon lane cull and world icon lane GPU layers (wasm32) |
| [`crates/map_rendering/world_layers_gpu/`](/crates/map_rendering/world_layers_gpu/README.md) | `world_layers_gpu` | the building, forest density, terrain texture and terrain line of sight overlay GPU layers (wasm32) |
| [`crates/map_rendering/map_renderer/`](/crates/map_rendering/map_renderer/README.md) | `map_renderer` | the render engine: GPU context, pipelines, camera, batch list, lane sinks, upload belts, statistics, asset sink (wasm32) |
| [`crates/map_rendering/map_render_diagnostics/`](/crates/map_rendering/map_render_diagnostics/README.md) | `map_render_diagnostics` | the renderer's readback self-checks, scene readback, frame benchmark and stress pool (wasm32) |
| [`crates/paper_doll/paper_doll_scene/`](/crates/paper_doll/paper_doll_scene/README.md) | `paper_doll_scene` | the Arsenal paper doll's parts, equipment regions, state colours, meshes, picks and callout anchors |
| [`crates/paper_doll/paper_doll_renderer/`](/crates/paper_doll/paper_doll_renderer/README.md) | `paper_doll_renderer` | the wgpu renderer of the paper doll on its own canvas, with its readback self-check (wasm32) |
| [`crates/contracts/fleet_wire_contract/`](/crates/contracts/fleet_wire_contract/README.md) | `fleet_wire_contract` | the fleet-command wire shapes the API and the fleet host agent share |
| [`crates/contracts/contract_schema_types/`](/crates/contracts/contract_schema_types/README.md) | `contract_schema_types` | the Rust types generated from the JSON Schemas |
| [`crates/mission/mission_wire_safety/`](/crates/mission/mission_wire_safety/README.md) | `mission_wire_safety` | the control-character scan of authored names and the cargo capacity scan of slot loadouts |
| [`crates/mission/mission_model/`](/crates/mission/mission_model/README.md) | `mission_model` | the compiled rows, ORBAT projection, authored extension blocks, slot line and typed ids of a mission |
| [`crates/mission/mission_crdt/`](/crates/mission/mission_crdt/README.md) | `mission_crdt` | the native yrs id arrays, row-aligned slot columns and undo grouping clocks of the mission document |
| [`crates/mission/formation_geometry/`](/crates/mission/formation_geometry/README.md) | `formation_geometry` | the placement patterns, align, space, orient and garrison positions of the arrange commands |
| [`crates/mission/mission_payload/`](/crates/mission/mission_payload/README.md) | `mission_payload` | the editor payload, export envelope and version body compiler, with the kit alias table |
| [`crates/mission/mission_validation/`](/crates/mission/mission_validation/README.md) | `mission_validation` | the ordered validation rules of an editor payload, their findings, facts and self-check |
| [`crates/mission/mission_compiler/`](/crates/mission/mission_compiler/README.md) | `mission_compiler` | the game-document compiler, its compile findings and the compiler identity |
| [`crates/mission/mission_document/`](/crates/mission/mission_document/README.md) | `mission_document` | the mergeable mission document: rows, hydrate and export, merge, selection and undo |
| [`crates/mission/mission_operations/`](/crates/mission/mission_operations/README.md) | `mission_operations` | the authoring commands and row projections the Mission Creator applies to the mission document |
| [`crates/mission_editing/map_editing_tools/`](/crates/mission_editing/map_editing_tools/README.md) | `map_editing_tools` | the headless map tools: the selection gesture and picks, the ruler, line of sight and the viewshed job scheduler |
| [`crates/mission_editing/mission_editing_commands/`](/crates/mission_editing/mission_editing_commands/README.md) | `mission_editing_commands` | the hosted document commands (ORBAT, layers, markers, zones, triggers and more) and the pure export, report and selection texts |
| [`crates/mission_editing/mission_editing_session/`](/crates/mission_editing/mission_editing_session/README.md) | `mission_editing_session` | the hosted mission document and its borrow chain, the undo drive, grouping, routing, selection, picks and overlay lanes |
| [`crates/mission_editing/mission_persistence/`](/crates/mission_editing/mission_persistence/README.md) | `mission_persistence` | the local draft decisions: record keys, blob verdicts, merge, local versus server, adoption and snapshots |
| [`crates/ballistics/ballistics_model/`](/crates/ballistics/ballistics_model/README.md) | `ballistics_model` | the ballistics catalog, shell flight model, surface wind, angular units and typed catalog ids |
| [`crates/ballistics/ballistics_solver/`](/crates/ballistics/ballistics_solver/README.md) | `ballistics_solver` | the high-angle firing solver per charge, wind-corrected aim, crest clearance and impact dispersion |
| [`crates/ballistics/fire_mission_planning/`](/crates/ballistics/fire_mission_planning/README.md) | `fire_mission_planning` | the fire-mission assembler: battery solutions, time fuzes, the comparison rule and the wording |
| [`crates/ballistics/ballistics_calibration/`](/crates/ballistics/ballistics_calibration/README.md) | `ballistics_calibration` | a ballistics catalog judged against the game's native tables, wind tables and engine oracle samples |
| [`crates/ballistics/ballistics_agreement_cases/`](/crates/ballistics/ballistics_agreement_cases/README.md) | `ballistics_agreement_cases` | the seeded lattice of battery fire problems and the bit patterns of their solutions |
| [`crates/api/api_identifiers/`](/crates/api/api_identifiers/README.md) | `api_identifiers` | the serde- and sqlx-transparent typed ids of every API table key, Discord snowflake and game runtime key |
| [`crates/api/api_foundation/`](/crates/api/api_foundation/README.md) | `api_foundation` | the handler error envelope, JSON wire formats, text policies, request parameters |
| [`crates/api/api_failpoints/`](/crates/api/api_failpoints/README.md) | `api_failpoints` | the `fail_point!` macro and, in test builds, the failpoint catalogue and arming registry |
| [`crates/api/api_configuration/`](/crates/api/api_configuration/README.md) | `api_configuration` | the environment configuration read at boot, trusted proxy networks, the process shutdown signal |
| [`crates/api/api_database/`](/crates/api/api_database/README.md) | `api_database` | the Postgres pool, the embedded migrations, the development seeds, SQLSTATE predicates |
| [`crates/api/api_http_layer/`](/crates/api/api_http_layer/README.md) | `api_http_layer` | access tokens, the middleware chain and extractors, rate limiters, metrics and health, the realtime hub |
| [`crates/api/api_property_evidence/`](/crates/api/api_property_evidence/README.md) | `api_property_evidence` | the dev-only property run recorder of the API's property tests |
| [`crates/api/api_mission_vocabulary/`](/crates/api/api_mission_vocabulary/README.md) | `api_mission_vocabulary` | the terrain and game mode enums several API domains name |
| [`crates/api/api_audit_log/`](/crates/api/api_audit_log/README.md) | `api_audit_log` | the audit severity and the best-effort and transactional audit line appends |
| [`crates/api/api_equipment_datasets/`](/crates/api/api_equipment_datasets/README.md) | `api_equipment_datasets` | the equipment dataset imports, their SQLite navigation index and the generation-pinned read queries |
| [`crates/api/api_member_activity/`](/crates/api/api_member_activity/README.md) | `api_member_activity` | member statistics, the leaderboard refresh, attendance attribution, the re-evaluation queue |
| [`crates/api/api_discord/`](/crates/api/api_discord/README.md) | `api_discord` | the Discord OAuth2, guild-member and announcement webhook clients and their typed failures |
| [`crates/api/api_caller_identity/`](/crates/api/api_caller_identity/README.md) | `api_caller_identity` | the role ladder, session and account authority, the identity lock order, the machine caller |
| [`crates/api/api_state/`](/crates/api/api_state/README.md) | `api_state` | the application state and its `FromRef` sub-state projections |
| [`crates/api/api_community_content/`](/crates/api/api_community_content/README.md) | `api_community_content` | the announcements, the wiki, the vehicle database, modpacks, uploads and the equipment data viewer routes |
| [`crates/api/api_identity_and_access/`](/crates/api/api_identity_and_access/README.md) | `api_identity_and_access` | Discord sign-in, session tokens, the caller's profile, the Arma link handshake, Discord membership |
| [`crates/api/api_administration/`](/crates/api/api_administration/README.md) | `api_administration` | the member roster and its moderation, the Discord role resync, the membership grace extension, the audit log console |
| [`crates/api/api_server_infrastructure/`](/crates/api/api_server_infrastructure/README.md) | `api_server_infrastructure` | the server registry, the live status feed, machine credentials, the fleet command ledger, runtime sessions |
| [`crates/api/api_missions/`](/crates/api/api_missions/README.md) | `api_missions` | the mission library, versions, artifacts, reviews and approvals, deployments, the armory, factions and registries |
| [`crates/api/api_match_telemetry/`](/crates/api/api_match_telemetry/README.md) | `api_match_telemetry` | the game runtime's session-fenced heartbeat, match registration, results revisions and detailed event batches |
| [`crates/api/api_operations/`](/crates/api/api_operations/README.md) | `api_operations` | the event calendar and its access control, ORBAT slotting and reservations, service records, leave requests, fire missions, ballistics catalogs |
| [`crates/api/api_command_center/`](/crates/api/api_command_center/README.md) | `api_command_center` | the members' dashboard with its fleet overview, the community leaderboards, the per-player statistics card |
| [`crates/api/api_background_workers/`](/crates/api/api_background_workers/README.md) | `api_background_workers` | the interval tasks the API binary arms at boot, each calling a domain service |
| [`crates/frontend/foundation/frontend_ui/`](/crates/frontend/foundation/frontend_ui/README.md) | `frontend_ui` | the design-system primitives every page and panel is assembled from, and the helpers with no domain of their own (time formatting, clipboard, sanitising) |
| [`crates/frontend/foundation/frontend_api_dtos/`](/crates/frontend/foundation/frontend_api_dtos/README.md) | `frontend_api_dtos` | the single-page app's wire types, one Rust shape per JSON body of the API, the role ladder and the frontend's typed ids |
| [`crates/frontend/foundation/frontend_transport/`](/crates/frontend/foundation/frontend_transport/README.md) | `frontend_transport` | the HTTP client, the request functions per endpoint, the SSE subscriber and the token provider seam |
| [`crates/frontend/foundation/frontend_route_table/`](/crates/frontend/foundation/frontend_route_table/README.md) | `frontend_route_table` | every route's path, layout flags and access tier, and the sidebar's navigation menu |
| [`crates/frontend/foundation/frontend_map_view/`](/crates/frontend/foundation/frontend_map_view/README.md) | `frontend_map_view` | the shared seam that puts a live terrain map on a canvas (Mission Creator, mortar map picker), and terrain heights |
| [`crates/frontend/foundation/frontend_session/`](/crates/frontend/foundation/frontend_session/README.md) | `frontend_session` | the browser side of signing in: the session store, token refresh, sign-out hooks, the route guard and content gates |
| [`crates/frontend/foundation/frontend_offline/`](/crates/frontend/foundation/frontend_offline/README.md) | `frontend_offline` | the page's half of offline use: service worker registration, the offline pack download, storage quota, offline state |
| [`crates/frontend/foundation/frontend_test_support/`](/crates/frontend/foundation/frontend_test_support/README.md) | `frontend_test_support` | the helpers the tests of every frontend crate share: repository file reads from the checkout root, captured API responses, the source scrubber (dev-only) |
| [`crates/frontend/features/mission_review_record/`](/crates/frontend/features/mission_review_record/README.md) | `mission_review_record` | one rendering of a mission's review, shared by every page that shows it: history, thread, artifact provenance, submit control |
| [`crates/frontend/pages/account_pages/`](/crates/frontend/pages/account_pages/README.md) | `account_pages` | the viewer's own session pages: sign-in, the OAuth callback, settings |
| [`crates/frontend/pages/command_center_pages/`](/crates/frontend/pages/command_center_pages/README.md) | `command_center_pages` | the dashboard, announcements and live server intel |
| [`crates/frontend/pages/operations_pages/`](/crates/frontend/pages/operations_pages/README.md) | `operations_pages` | the event schedule, event detail, ORBAT slotting, "My Deployments" and the leaderboards |
| [`crates/frontend/pages/mission_hub_pages/`](/crates/frontend/pages/mission_hub_pages/README.md) | `mission_hub_pages` | the mission library, the mission dossier and the "New Mission" dialog |
| [`crates/frontend/pages/field_tools_pages/`](/crates/frontend/pages/field_tools_pages/README.md) | `field_tools_pages` | the mortar calculator: on-device firing solutions, map picker, offline pack |
| [`crates/frontend/pages/doctrine_pages/`](/crates/frontend/pages/doctrine_pages/README.md) | `doctrine_pages` | the doctrine wiki, the vehicle identification index and the modpack manifests |
| [`crates/frontend/pages/administration_pages/`](/crates/frontend/pages/administration_pages/README.md) | `administration_pages` | the administration panels: event manager, server control, personnel, approvals, content manager, audit logs, ballistics catalogs |
| [`crates/frontend/workspaces/mission_creator_state/`](/crates/frontend/workspaces/mission_creator_state/README.md) | `mission_creator_state` | the Mission Creator's lowest crate: layout tokens, review mode, world-layer preferences, the asset catalog and rules model, the outliner model, zones, transforms |
| [`crates/frontend/workspaces/mission_creator_engine_bridge/`](/crates/frontend/workspaces/mission_creator_engine_bridge/README.md) | `mission_creator_engine_bridge` | the Mission Creator's engine seam and input: boot, viewport and frame timing, the hosted mission document, overlays, pointer and keyboard tools |
| [`crates/frontend/workspaces/mission_creator_session/`](/crates/frontend/workspaces/mission_creator_session/README.md) | `mission_creator_session` | the Mission Creator's per-tab session: IndexedDB drafts, hydrate, the cross-tab lock, save status, the conflict dialog |
| [`crates/frontend/workspaces/mission_creator_arsenal/`](/crates/frontend/workspaces/mission_creator_arsenal/README.md) | `mission_creator_arsenal` | the Mission Creator's Arsenal: the loadout domain, gear catalog trees, the Arsenal tab and the 3D paper doll |
| [`crates/frontend/workspaces/mission_creator_workspace/`](/crates/frontend/workspaces/mission_creator_workspace/README.md) | `mission_creator_workspace` | the Mission Creator page, the top-down 2D CAD workspace: docks, outliner, inspectors, modals, the canvas mount, the read-only review workspace |
| [`crates/frontend/workspaces/debug_benches/`](/crates/frontend/workspaces/debug_benches/README.md) | `debug_benches` | the URL-only benches: building viewer, building interior, world line of sight, ballistics agreement, data viewer |
| [`tools/xtask/`](/tools/xtask/README.md) | `xtask` | the `cargo xtask` command line and dispatch onto the tool crates, plus the `ai`, `fetch`, `map`, `refactor`, `schema`, `ticket`, `verify` and `wave` command groups |
| [`tools/foundation/verification_core/`](/tools/foundation/verification_core/README.md) | `verification_core` | fail-closed verdicts, pattern scans, gates and the verification lock |
| [`tools/foundation/process_runner/`](/tools/foundation/process_runner/README.md) | `process_runner` | process isolation, deadlines, host-bridge execution and the secure shell transport |
| [`tools/foundation/repository_laws/`](/tools/foundation/repository_laws/README.md) | `repository_laws` | every repository law: the workspace laws (crate tiers, anatomy, test-file reachability, frontend layering, Tailwind sources), file length, test placement, the apps' dependency directions |
| [`tools/foundation/repository_layout/`](/tools/foundation/repository_layout/README.md) | `repository_layout` | the paths every tool shares, relative to the checkout root; its prelude re-exports `repository_root`'s finder |
| [`tools/foundation/deploy_settings/`](/tools/foundation/deploy_settings/README.md) | `deploy_settings` | the one reader of `deploy/deploy.env` and its precedence rule over exported variables |
| [`tools/foundation/tool_test_support/`](/tools/foundation/tool_test_support/README.md) | `tool_test_support` | the test locks (process variables, working directory) and the checkout root the tool crates' tests share (dev-only) |
| [`tools/tickets/ticket_model/`](/tools/tickets/ticket_model/README.md) | `ticket_model` | the typed ticket, its canonical TOML encoding, the corpus store and the ticket-domain paths |
| [`tools/tickets/ticket_metrics/`](/tools/tickets/ticket_metrics/README.md) | `ticket_metrics` | slice-run receipts and token estimates |
| [`tools/tickets/ticket_wave_lock/`](/tools/tickets/ticket_wave_lock/README.md) | `ticket_wave_lock` | the wave lock compiler, reader and checker |
| [`tools/tickets/ticket_registry/`](/tools/tickets/ticket_registry/README.md) | `ticket_registry` | ticket operations, validation, queue and roadmap sync, the `cargo xtask ticket` verbs |
| [`tools/tickets/ticketboard_model/`](/tools/tickets/ticketboard_model/README.md) | `ticketboard_model` | the ticketboard's headless half: registry, wave lock and metrics models, events and the egui-free application state |
| [`tools/commands/repository_relocation/`](/tools/commands/repository_relocation/README.md) | `repository_relocation` | manifest-driven moves of tracked paths and the retired-spelling verification behind `cargo xtask refactor relocate` |
| [`tools/commands/agent_context_guards/`](/tools/commands/agent_context_guards/README.md) | `agent_context_guards` | the AI agent tool-call guard behind `cargo xtask ai` (Bash and Read rules, the session read set) and the filtered command runner that never hides a failure |
| [`tools/commands/schema_tooling/`](/tools/commands/schema_tooling/README.md) | `schema_tooling` | the contract codegen, the contract schema gates, the ORBAT slot flattening and the font-table generator behind `cargo xtask schema` and `cargo xtask gen` |
| [`tools/commands/ballistics_oracle_tooling/`](/tools/commands/ballistics_oracle_tooling/README.md) | `ballistics_oracle_tooling` | the ballistics catalog and calibration fixtures behind `cargo xtask ballistics trim-export` |
| [`tools/commands/enfusion_mcp/`](/tools/commands/enfusion_mcp/README.md) | `enfusion_mcp` | the Enfusion MCP client behind `cargo xtask mcp`: daemon control, tool calls, the offline selftest, Workbench NET API calls, log verdicts |
| [`tools/commands/api_readiness_checks/`](/tools/commands/api_readiness_checks/README.md) | `api_readiness_checks` | the API readiness judge behind `cargo xtask verify api-readiness`: acceptance register, evidence receipts, fingerprints, the staging recorder, the property-test seed |
| [`tools/commands/workstation_setup/`](/tools/commands/workstation_setup/README.md) | `workstation_setup` | the `cargo xtask setup` commands and the staging host check of `cargo xtask mod bootstrap-staging` |
| [`tools/commands/database_operations/`](/tools/commands/database_operations/README.md) | `database_operations` | the local database lane behind `cargo xtask db`, the database container layer and the verified backup, guarded restore and restore drill behind `cargo xtask deploy db`, the milestone announcement seed, and the seed and SQL-shape checks behind `cargo xtask verify` |
| [`tools/commands/deployment/`](/tools/commands/deployment/README.md) | `deployment` | the website and staging fleet deploys behind `cargo xtask deploy website` and `cargo xtask deploy staging`, and the staging compose-path check behind `cargo xtask verify` |
| [`tools/commands/remote_debugging/`](/tools/commands/remote_debugging/README.md) | `remote_debugging` | the staging server-join probes and the direct-join report behind `cargo xtask debug`, the remote console log verdict behind `cargo xtask mod remote-logs`, and the mission-version upload reproduction behind `cargo xtask repro` |
| [`tools/commands/staging_procedures/`](/tools/commands/staging_procedures/README.md) | `staging_procedures` | the staging acceptance harness behind `cargo xtask staging`: the fleet, Discord and load procedures and their receipts, the confirmed host actions and the read-only commands around them |
| [`tools/commands/ci_task_catalog/`](/tools/commands/ci_task_catalog/README.md) | `ci_task_catalog` | the CI task table and its runner behind `cargo xtask ci` and `cargo xtask help`, the build lane recipes behind `cargo xtask mk`, the shared cargo target pin and its checks, the `verify ci-shell` and `verify ci-schema-parity` gates, and the map asset checks |
| [`tools/commands/platform_execution/`](/tools/commands/platform_execution/README.md) | `platform_execution` | the platform factory behind `cargo xtask platform`: the wave driver, slice runs and their receipts, the slice worktree lifecycle and the unattended-run preflight |
| [`tools/commands/mod_operations/`](/tools/commands/mod_operations/README.md) | `mod_operations` | the game mod's operations behind `cargo xtask mod`: the headless compile gate, the world boot, the playtest server, the equipment and vehicle export publication, the website API client and the mod wave driver |
| [`tools/checks/repository_checks/`](/tools/checks/repository_checks/README.md) | `repository_checks` | the workspace-law, route-tag, ORBAT coherency, language-ban, file-length, upstream code-leak and registry alias checks behind `cargo xtask verify`, and the tooling rules over every tool crate |
| [`tools/checks/mod_script_checks/`](/tools/checks/mod_script_checks/README.md) | `mod_script_checks` | the Enfusion comment card, the mod script pins, the UI layout gate and the Workbench spawn runs behind `cargo xtask verify` and `cargo xtask mod` |
| [`tools/checks/documentation_checks/`](/tools/checks/documentation_checks/README.md) | `documentation_checks` | the README coverage, Markdown placement and link-check gates behind `cargo xtask verify` and `cargo xtask ci verify-documentation` |
| [`tools/enfusion/enfusion_mcp_broker/`](/tools/enfusion/enfusion_mcp_broker/README.md) | `enfusion_mcp_broker` | the `mcpd` broker over one enfusion-mcp server behind a Unix socket, and its offline stub |
| [`tools/enfusion/enfusion_pak/`](/tools/enfusion/enfusion_pak/README.md) | `enfusion_pak` | the Enfusion `.pak` archive reader: the merged virtual file system under the blueprint and world policies, the loose and layered sources |
| [`tools/enfusion/enfusion_script_index/`](/tools/enfusion/enfusion_script_index/README.md) | `enfusion_script_index` | the Enfusion script oracle behind `enf` (symbol indexes, lookups, citation and capability checks, vanilla extraction) and the vanilla page mirrors behind `cargo xtask fetch` |
| [`tools/map_assets/blueprint_compiler/`](/tools/map_assets/blueprint_compiler/README.md) | `blueprint_compiler` | the building-blueprint compiler: voxel dumps and game models to blueprints, occlusion sidecars and the blueprint archive, behind `cargo xtask map` |
| [`tools/map_assets/map_asset_verification/`](/tools/map_assets/map_asset_verification/README.md) | `map_asset_verification` | the gates over a terrain's committed map assets and the map golden fixtures behind `cargo xtask schema` and `verify blas-manifest`, and the world line-of-sight probe behind `cargo xtask map world-los` |
| [`tools/map_assets/map_raster_pipeline/`](/tools/map_assets/map_raster_pipeline/README.md) | `map_raster_pipeline` | the map raster pipeline behind the `map` binary: a terrain's satellite container, tile pyramids, cartographic render, labels, water archives, the world-glyph atlas, and PNG images of the Workbench water and road exports |
| [`tools/map_assets/world_export_pipeline/`](/tools/map_assets/world_export_pipeline/README.md) | `world_export_pipeline` | the world-export pipeline behind the `world` binary: a terrain's object chunks, catalogue, census, density tiles, forest regions, roads and elevation from a Workbench export, and the gates that prove them |
| [`tools/browser_testing/chrome_devtools_protocol/`](/tools/browser_testing/chrome_devtools_protocol/README.md) | `chrome_devtools_protocol` | the Chrome DevTools Protocol client of the browser gates: Chromium discovery and launch, pages over WebSockets, the gate font cache |
| [`tools/browser_testing/browser_gate_suites/`](/tools/browser_testing/browser_gate_suites/README.md) | `browser_gate_suites` | the browser gates of the single-page app and the `gate` and `capture` command lines: the static server, the DOM oracle, route drift, the Mission Creator smokes, the data viewer gate, the ballistics agreement and offline mortar gates, the capture rig, the doctor |
| [`tools/staging/staging_load_plan/`](/tools/staging/staging_load_plan/README.md) | `staging_load_plan` | the staging member load's plan, request catalog, pacing, records, report and their JSON codec, without tokio |
| [`tools/staging/staging_load_generator/`](/tools/staging/staging_load_generator/README.md) | `staging_load_generator` | the staging member load's virtual clients and the `staging-load` executable's command line |
| [`tools/staging/acknowledgement_dropping_relay/`](/tools/staging/acknowledgement_dropping_relay/README.md) | `acknowledgement_dropping_relay` | the loopback relay that withholds one fleet executor answer, and the `acknowledgement-dropping-relay` command line |
| [`tools/staging/staging_fixtures/`](/tools/staging/staging_fixtures/README.md) | `staging_fixtures` | the `staging-fixtures` host tool that stages a staging run's fixtures through the API crates' services, with its 4 database suites |
| [`tools/developer_tools/`](/tools/developer_tools/README.md) | `developer_tools` | the eight tool binaries, each a one-line `main` over one tool crate (script index, browser gates, MCP broker, world export, map assets, capture, the staging load and relay); no library |

The API is 24 packages: the `api` application and the 23 crates under `crates/api/`, grouped as
infrastructure (`api_identifiers`, `api_foundation`, `api_failpoints`, `api_configuration`,
`api_database`, `api_http_layer`, the dev-only `api_property_evidence`), kernel
(`api_mission_vocabulary`, `api_audit_log`, `api_equipment_datasets`, `api_member_activity`,
`api_discord`, `api_caller_identity`, `api_state`), the eight domain crates and
`api_background_workers`. Its 154 integration binaries are 150 in `apps/api/tests/` and the 4
suites of `staging_fixtures`.

Every package is named after its folder, in snake_case. A crate under `crates/` sits in the
folder of its category and declares its tier in its manifest (`cargo xtask verify crate-tiers`).
The map's streaming crates (`crates/streaming/`), its renderer and typed GPU layers
(`crates/map_rendering/`), the Arsenal's paper doll (`crates/paper_doll/`) and the map-agnostic
GPU crates (`crates/graphics/`) are crates like any other; the
[crates README](/crates/README.md) lists every category. The two tool binaries, `tools/xtask` and
`tools/developer_tools`, depend only on tool crates (the checkout-root finder
`crates/foundation/repository_root` comes through `repository_layout`'s prelude), and no tokio, axum, reqwest, resvg or image
enters xtask's dependency closure. The mod suite under `apps/mod/` is not
Cargo code: its three Enfusion addons are built by Workbench and checked by `cargo xtask mod compile`.

## Where things live

```text
code ─────────── apps/<product>/            products, one folder each
                 crates/<category>/<crate>/ library crates, by category
                 tools/<category>/<crate>/  repository tooling (xtask and developer_tools directly under tools/)
deploy ───────── deploy/                    release image, compose files, Caddy, systemd units
shapes ───────── contracts/definitions/     JSON Schemas, the source of generated contract types
                 contracts/fixtures/        golden test data, positive and negative
data ─────────── assets/terrains/           built-in islands, served at /map-assets (Git LFS)
                 assets/glyphs/             world-object glyph atlas and its SVG sources
documents ────── documentation/<code path>/ feature docs mirroring the code
                 documentation/runbooks/    procedures; standards/, glossary/, archive/ beside it
work tracking ── .ai/tickets/               one TOML per ticket, the queue and the templates
```

- **Code.** A product's code and its README sit in its folder under `apps/`; every folder carries
  a README.md built to the [README standard](/documentation/standards/readme_standard.md), and
  the code trees hold no other Markdown. Engine and layer boundaries are in the
  [crate boundary rules](/documentation/standards/crate_boundary_rules.md).
- **Contracts.** Every shape that crosses a network, process or language boundary is a schema in
  `contracts/definitions/`; `cargo xtask ci schema-codegen` generates the Rust contract types
  from it, and the backend models stay the snake_case source of truth of the API.
- **Assets.** Terrain datasets are Git LFS objects matched by `.gitattributes`; export scratch,
  equipment exports and terrain tiles are ignored and rebuilt by the export tools.
- **Documentation.** Every document lives under `documentation/`. A feature doc sits at the
  documentation root plus its code path without `src/`: `documentation/apps/api/` for `apps/api/`,
  `documentation/crates/streaming/` for `crates/streaming/`,
  `documentation/crates/frontend/workspaces/mission_creator_workspace/` for
  `crates/frontend/workspaces/mission_creator_workspace/src/`. The mod's documents sit in
  `documentation/apps/mod/` and leave out the framework's `Scripts/Game/TBD/`, as the
  [documentation standards](/documentation/standards/documentation_standards.md) set out.
- **Deployment.** `deploy/` holds what runs the platform outside a developer machine: the API's
  release `Dockerfile` (its build context narrowed by the root `.dockerignore`), the development
  and staging compose files, the Caddy site in `deploy/caddy/` (the one folder the staging Caddy
  container mounts, so a host's `deploy.env` stays outside it), the `deploy.env.example` that
  `cargo xtask deploy` reads, and the systemd units and timers in `deploy/systemd/`.
- **Agent configuration.** `CLAUDE.md` holds the project laws, the atlas and the canonical
  commands; `.cursor/rules/` the Cursor rules; `.claude/settings.json` the Claude Code settings.

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the crate-tier law,
  its category matrix and firewalls, and the other workspace laws.
- [Documentation entry](/documentation/README.md) — the map of every document.
- [Where does X go?](/documentation/standards/where_does_x_go.md) — where a new file belongs.
