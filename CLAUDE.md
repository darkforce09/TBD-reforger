# TBD Reforger Platform

Platform suite for the "TBD" Arma Reforger milsim community: Discord auth, event / ORBAT scheduling, mission library, the Mission Creator (a top-down 2D mission editor), game-server fleet control and telemetry, leaderboards, doctrine wiki, the TBD game mod, and Enfusion mod tooling.

---

## 1. Core Project Laws

1. **Hard Gate — No Silent Deferrals**:
   Do the whole ask. Never invent "out of scope", "deferred", or ship an MVP and call the task done unless the operator explicitly specifies to defer that piece ("defer X", "skip X"). Rule: `.cursor/rules/no-silent-deferrals.mdc`.
2. **Git Discipline — Direct to Main**:
   Never create git branches (`git checkout -b` is forbidden). All commits land directly on `main`. Merge and delete any stray branch immediately. The one exception: the `slice/<id>` branches that the slice and wave tooling (`cargo xtask platform slice-worktree`, `cargo xtask platform wave`, `cargo xtask mod wave`) creates, merges and deletes itself.
3. **Fundamentals & Clean Architecture Over Hacks**:
   Never tack on ad-hoc code to "just make it work". If a clean solution requires an architectural adjustment or structural refactoring, plan and execute that refactor cleanly. Understandability, simplicity, and long-term maintainability strictly supersede quick patches.
4. **Zero Context Needed for Directory & File Names**:
   Every folder, file, module, and symbol name must be so clear and self-describing that anyone can immediately and unmistakably understand its purpose without needing *any* prior project or historical context. Avoid cryptic abbreviations, project-specific jargon, and overloaded names.
5. **Categorize Variants & Primitives (Avoid Flat Dumps)**:
   Avoid flat dumping of dozens of files or variant variations into a single folder. Related variants, numerical sets (e.g. column counts, rounded radius variants), and functional primitives should be grouped into dedicated, well-named subfolders to maintain clean directory comprehension.
6. **Strict Boundary Layers**:
   - Every product crate sits at `crates/<category>/<crate>` (the frontend's at `crates/frontend/<layer>/<crate>`) and every tool crate at `tools/<category>/<crate>`, declaring its `category`, `tier` and `targets`; the applications are crates like any other (`api_server`, `frontend_application`, `offline_service_worker`, `game_server_host_agent`, `ticketboard_desktop`), and no member depends on one of them. The crate-tier law (`cargo xtask verify crate-tiers`) holds every dependency edge: tiers point strictly down, the category matrix allows the edge, and the external-crate firewalls hold. The only members outside the layout are the two tool binaries `tools/xtask` and `tools/developer_tools`; the game mod at `mod/` holds no crate, and the crate-tier law sweeps the whole checkout for manifests, so a `Cargo.toml` placed there is a finding. The [crate boundary rules](/documentation/standards/crate_boundary_rules.md) state every rule as the code enforces it.
   - Foundation and contracts (`crates/foundation/`, `crates/contracts/`): Leaf crates and one-boundary contract crates. Foundation depends on foundation only; contracts on foundation and contracts.
   - Graphics crates (`crates/graphics/`: `render_primitives`, `gpu_device`, `gpu_frame`, `renderer_core`): Map-agnostic rendering — byte layouts, the GPU context, the frame vocabulary, pipelines, draw encoding, the renderer contracts. Knows **zero** map concepts (no map noun in a declared name); depends on foundation and graphics crates only.
   - Map engine crates (the engine categories `crates/geometry/`, `crates/world_formats/`, `crates/terrain/`, `crates/world_objects/`, `crates/line_of_sight/`, `crates/map_overlay/`, `crates/streaming/` over graphics, and the rendering categories `crates/map_rendering/` and `crates/paper_doll/` above them): spatial computation, world and terrain formats, streaming, the render engine and its typed GPU layers. Streaming crates never depend on rendering crates; `wgpu` lives only in `crates/map_rendering/` and the GPU packages. Zero UI/Leptos dependencies.
   - Mission domain (`crates/mission/`, over foundation and geometry crates) and ballistics (`crates/ballistics/`, over foundation): no map, GPU or browser code. The Mission Creator's editing layer (`crates/mission_editing/`) adds the static-world engine categories and names no browser crate or browser token.
   - Frontend (`crates/frontend/`): Presentation, navigation, and CAD workspaces as Leptos crates (leptos only here) in the layers foundation < features < pages and workspaces < shell; pages and workspaces are peers that never depend on each other. The shell layer `crates/frontend/shell/` holds the single-page app `frontend_application` and the `offline_service_worker`, two peers that never name each other. Consumes every crate but the api, fleet and tool crates.
   - API (`crates/api/`): Axum REST API and SSE realtime hub. The domain code lives in the api crates (infrastructure < kernel < domains < workers < the server), which depend on foundation, contracts, mission, ballistics and api crates; the server crate `crates/api/api_server` assembles them into the `api-server` binary. sqlx and axum live only in api crates, with axum also in the `tools/browser_testing` and `tools/staging` harness servers and sqlx also in `tools/staging/staging_fixtures`.
   - Fleet (`crates/fleet/`): The `game_server_host_agent` beside each game-server instance, which carries out the API's fleet commands; depends on foundation crates built for every target and contracts crates only.
   - Tools (`tools/<category>/`): Depend on foundation, contracts, mission, ballistics and tool crates and on engine crates built for every target; never on a wasm-only or frontend crate, nor an api crate outside `staging_fixtures`. The ticketboard desktop viewer `ticketboard_desktop` is a `tools/tickets` crate. The binaries `tools/xtask` and `tools/developer_tools` depend on tool crates only, and the dependency closure of xtask holds no tokio, axum, reqwest, resvg or image.
7. **File Size Limits & Test Placement (Hard Ceilings — Zero Exemptions)**:
   - Production files must stay **at or under 500 lines**.
   - Test files (inside a `tests/` folder or named `*_tests.rs`) must stay **at or under 1000 lines**.
   - The ceilings apply in every language; `cargo xtask verify file-length` enforces them on the Rust source trees and the pinned mod Scripts roots (`mod/tbd-framework/Scripts`, `mod/tbd-emcp/Scripts`) by raw line count.
   - **Zero Exemptions / No Allowlist**: There is NO allowlist file and NO exemption mechanism. Never create an allowlist (`.coding-standards-allowlist.yaml` or any other), use allowlist comments, or bypass these limits. If a file approaches or exceeds 500 lines, you MUST decompose it by responsibility into cohesive submodules.
   - **No inline test modules**: Unit tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
8. **In-Code Documentation Standards (Rust & Enfusion)**:
   - **Present-Tense, Context-Free Invariants (Universal)**:
     Comments and docstrings must describe strictly what the code does *now* and *why* (invariants, mathematical models, engine/hardware constraints, failure modes). Never document historical transitions (no "rewritten from X", "fixed in Y"), ticket references in source comments, or references to retired codebases (no "mirrors old TS file"). Commit history owns history.
   - **Rust Code Standards (`rustdoc` — `crates/`, `tools/`)**:
     - **Module Headers (`//!`)** (recommended style): Non-trivial modules should carry a 4-point architectural contract header:
       - `**Role:**` Primary responsibility of this module in the subsystem.
       - `**Position:**` Boundary layer and data flow context (what feeds it, who consumes it).
       - `**Signals & state:**` Mutable state, reactive signals, or thread ownership (or "none; pure functions").
       - `**Invariants:**` Non-negotiable structural guarantees and mathematical boundaries.
     - **Symbol Docs (`///`)**: Public types, functions, methods, and enums must use standard markdown docstrings with validated intra-doc links (e.g. `[`crate::path::Type`]`).
     - **Cross-Boundary Tags** (recommended style, not machine-checked):
       - Axum HTTP handlers should declare `/// @route <METHOD> <path>`.
       - Schema-projecting DTOs and models should declare `//! @contract <schema>#<pointer>`.
   - **Enfusion Mod Standards (`Doxygen` — `mod/`)** (recommended style for banners, headers, member docs and tags; not machine-checked):
     - **Class & Method Banners**: `//!` single-line banners describing class purpose and method contracts.
     - **Field & Enum Members**: `//!<` trailing doc comments documenting units, default values, and JSON key bindings.
     - **File/Plugin Headers**: `/** ... */` multi-line block headers for top-level scripts and Workbench plugins.
     - **Network Authority (Mandatory)**:
       - `//! @authority server|client|owner` on every method whose correctness depends on execution context.
       - `//! @rpc <Reliable|Unreliable> <Server|Owner|Broadcast>` directly above every `[RplRpc]` attribute.
       - `//! @replicated <prop>` directly above every `[RplProp]` field specifying its replication hook.
     - **Cross-Boundary Tags**:
       - Hand-written JSON DTO structs MUST declare `//! @contract <schema>#<pointer>`.
       - REST API call sites MUST declare `//! @route <METHOD> <path>`.
9. **API & Contract Parity**:
   - Backend Rust models (`crates/api/api_<domain>/src/models/`) are the snake_case API source of truth.
   - Contract types are generated from `contracts/definitions/*.json` via `cargo xtask ci schema-codegen`.
   - Frontend DTOs (`crates/frontend/foundation/frontend_api_dtos/src/`) mirror models with strict R-api golden test parity.
10. **Keep Documentation Truthful**:
    - When you change a folder's surface, commands or boundaries, update its README.md and the feature docs whose behaviour changed, and keep the comments of the code you alter current. This is advice, not a commit gate.
    - [documentation/README.md](/documentation/README.md) is the documentation entry (map and authority ladder); [documentation/standards/](/documentation/standards/README.md) holds the documentation, README and coding standards and the templates.
    - Terms follow the [glossary](/documentation/glossary/README.md): the editor is the **Mission Creator**; the authored document is a **mission** (never "scenario" in prose; code identifiers stay quoted as spelled); Enfusion's world plus game-mode config is the **mission header**; an **event** is a scheduled session record; **operations** is its domain.
11. **Pre-alpha Test Policy**:
    - Test core logic only: math and geometry, file and wire formats, CRDT merge and undo, auth and permissions, mission compile and validation, data integrity, and guards on destructive operations.
    - Never write tests that pin source text, prose or wording, CSS classes, constants, file layout, or `Debug`/`Display` output.
    - The API keeps one integration test binary per domain, each provisioning its database once.
    - No failpoint suites and no property-evidence suites.
    - Slow suites (headless browser gates, mod world boot) run nightly or on demand, never on every commit.
    - Before committing: `cargo xtask mk rust-fmt`, `cargo xtask mk rust-clippy`, and the tests of the crates you touched.

---

## 2. Monorepo Directory Atlas

The [workspace layout](/documentation/architecture/workspace_layout.md) explains the top-level folders and every workspace member.

```text
mod/                                     <-- Enfusion game mod suite: three addons, the reference lanes and its README; no Rust crate lives here
├── tbd-framework/                       <-- Shipping game mod (TBD_Framework; depends on vanilla only)
│   ├── Configs/                         <-- Menu presets, input contexts, key actions
│   ├── Data/                            <-- Alias spawn registry, backend config template
│   ├── Missions/                        <-- Mission headers a server boots (TBD Dev POC on Everon)
│   ├── Prefabs/                         <-- Game mode with its manager components, player controller
│   ├── worlds/                          <-- TBD Dev POC world and the layer that places the game mode
│   ├── Scripts/Game/TBD/                <-- Gameplay scripts (compiled into game runtime)
│   │   ├── API/                         <-- Platform bridge: game-runtime session, fleet commands, identity links, results
│   │   ├── Core/                        <-- Structured log, prefab registry, player chat, SHA-256
│   │   ├── Gamemode/                    <-- Stage machine, safe start, objectives, tasks, end conditions
│   │   ├── Session/                     <-- Mission selection, lobby, briefing, spectator, post-game
│   │   ├── Systems/                     <-- Mission load, spawns, loadouts, zones, AI, audio, markers, radio
│   │   └── UI/                          <-- Menu framework, components, HUD, mock catalogs
│   └── UI/                              <-- UI layout definitions and textures
│       ├── Textures/                    <-- Rounded-shape disc, hero art, masks, icons
│       └── layouts/                     <-- Enfusion widget layout files (.layout)
│           ├── Common/                  <-- Reusable design system primitives (panels, rows, chips, inputs)
│           ├── Hud/                     <-- Objective panel shown during live play
│           └── Session/                 <-- Pre-game screens, post-game overlays, shared bars and panels
├── tbd-export/                          <-- Workbench export addon (TBD_Export; depends on vanilla + TBD_EMCP, not the framework)
│   ├── Missions/                        <-- Export mission header (Everon export world)
│   ├── Prefabs/                         <-- Export game mode carrying the road export component
│   ├── worlds/                          <-- Standalone export world over vanilla Eden
│   └── Scripts/
│       ├── Game/TBD/Export/             <-- Runtime road-network export component; ballistics oracle play-mode simulation run
│       └── WorkbenchGame/               <-- Workbench export plugins (MapExport, EquipmentExport, EquipmentVehicleExport, VehicleExport, BallisticsOracle, registry)
├── tbd-emcp/                            <-- Enfusion MCP bridge handler scripts (TBD_EMCP)
│   └── Scripts/WorkbenchGame/EnfusionMCP/ <-- 19 committed NetAPI automation handlers
└── References/                          <-- Licensed upstream reference lanes, gitignored except README
    ├── crf_framework/                   <-- Upstream Coalition Reforger Framework scripts and assets
    ├── vanilla_reference/               <-- Extracted vanilla Reforger scripts and Script API pages
    └── playable_selector/               <-- PlayableSelector checkout (design-mirror only)

crates/                                  <-- Product crates grouped by category (crates/<category>/<crate>), the applications among them; every manifest under it is a workspace member declaring its tier
├── foundation/                          <-- Base crates: tier 0 leaves, plus orbat_slot_ids on newtype_ids
│   ├── http_url_guard/                  <-- The HTTP(S) URL check the API and the single-page app share, with its one case table
│   ├── newtype_ids/                     <-- Macros declaring serde-transparent typed ids (string, integer, uuid; an sqlx form expanded at the call site)
│   ├── orbat_slot_ids/                  <-- An ORBAT slot's two ids: SlotUid (durable editor id) and SlotId (derived wire id)
│   ├── time_source/                     <-- Wall-clock and monotonic time sources, `wall_clock_ms()`, RFC 3339 UTC formatting and validation
│   ├── deterministic_random/            <-- The seeded generators: SplitMix64, LinearCongruential64 (MMIX constants), LinearCongruential32 (C rand constants)
│   ├── content_digest/                  <-- SHA-256 and SHA-384 hex digests, framed hashing
│   ├── repository_root/                 <-- The one checkout-root finder: the walk up to the `.ai/tickets/ROOT` marker
│   └── browser_platform/                <-- Browser console macros and fetch helpers (wasm32 only)
├── contracts/                           <-- Crates that hold one boundary contract
│   ├── offline_cache_policy/            <-- Offline cache names, request classes, offline pack and network fallback rules the service worker applies
│   ├── fleet_wire_contract/             <-- Fleet-command wire shapes, executor kinds, the machine-credential format and secret-file limits
│   └── contract_schema_types/           <-- Rust types generated from contracts/definitions (`cargo xtask ci schema-codegen`)
├── geometry/                            <-- Engine geometry: vectors, segments, rigid transforms, map coordinates, cameras, spatial indexes, grid rasterization
│   ├── geometry_primitives/             <-- 3D vector ops, 2D segment geometry, rigid transforms, axis-aligned boxes
│   ├── grid_rasterization/              <-- Half-up rounding, Catmull-Rom spline, polygon scanline spans, anti-aliased disc stamps
│   ├── map_coordinates/                 <-- Terrain frames (map centres, bounds), chunk math, rounding, grid references
│   ├── camera_math/                     <-- Orthographic map camera, orbit camera, 4x4 matrices
│   └── spatial_indexes/                 <-- Triangle BVH and its .bvh sidecar, the flat-tree build core, point grid, picks, clusters
├── world_formats/                       <-- On-disk world formats and their readers
│   ├── world_file_formats/              <-- rkyv archives, containers, density grids, POD layouts, their typed ids
│   ├── prefab_catalog/                  <-- Prefab rows, render classes, footprint lookups, prefab tables, world payload decoding
│   ├── world_chunks/                    <-- Chunk JSON and TBDC container decoding, chunk ids, the terrain manifest
│   └── world_store/                     <-- Headless world store: manifest, prefab table, roads, regions, one chunk at a time
├── terrain/                             <-- The ground the map reads: elevation, relief, satellite container, water, roads
│   ├── terrain_elevation/               <-- Raster placement, PNG and raw grid decoding, bilinear sampling, the vector grid
│   ├── terrain_relief/                  <-- Hillshade image, contour rings with summit picks, sea band fills
│   ├── satellite_imagery/               <-- The .tbd-sat container reader: header, both index versions, checks, level picks
│   ├── road_network/                    <-- Road segments and class codec, styling and zoom gates, export image styles, road meshes, cartographic strips, airfield
│   └── water_bodies/                    <-- Bathymetry water mask and palette, level suffix plan, inland water archive, sea fill mesh
├── map_overlay/                         <-- What the map draws on the terrain and in what order
│   ├── map_draw_lanes/                  <-- The 48 lane roles, their paint order and wire ids, the zoom gates
│   ├── label_layout/                    <-- Label declutter, town importance, world glyph sizing, label glyph packing
│   ├── unit_symbology/                  <-- Side tints, role and vehicle classes, symbol atlas, markers, squad links
│   └── overlay_instances/               <-- Slot, vehicle, comment and cluster icon instances, fire-mission marks
├── world_objects/                       <-- What stands on the ground: vegetation, building interiors, place names
│   ├── vegetation/                      <-- Forest regions, canopy mass outline, tree counts, island density bins
│   ├── building_interiors/              <-- Building blueprints and sight-line attribution, compounds with doors, section cuts
│   └── place_names/                     <-- Spot heights, town and road names, their declutter, the labels archive, glyph packing
├── line_of_sight/                       <-- Visibility at three scales: the bare ground, inside one building, through the placed world
│   ├── terrain_line_of_sight/           <-- Elevation profiles along a sight line, viewsheds whole or a ray at a time
│   ├── interior_line_of_sight/          <-- Compound traces, the one sight-line evaluation, floor washes whole or in batches
│   └── world_line_of_sight/             <-- The world occluder: chunk box trees, the prefab occluder library, verdicts with coverage
├── streaming/                           <-- The streamed world's CPU half: chunk residency and the draw buffers composed over it
│   ├── chunk_scheduler/                 <-- Viewport pin, in-flight marks, LRU eviction, chunk and prefab ingest, object index, rebuild requests
│   ├── chunk_draw_buffers/              <-- Draw set, glyph, strip and footprint buffers, layer toggles, the world residency owner
│   ├── map_asset_loading/               <-- Browser loaders (world, occluder, terrain, satellite, forest, labels), mesh composition, live memory budget, asset statistics
│   ├── map_streaming_host/              <-- Browser map host: terrain and world boot, camera settle, view preferences, map queries
│   └── map_streaming_model/             <-- World-layer and host preferences, boot progress, the memory budget model, the MapAssetSink contract
├── mission/                             <-- The mission domain: model, editor payload, validation, compilation, mergeable document, authoring commands
│   ├── mission_wire_safety/             <-- Control-character scan of authored names, cargo capacity scan of slot loadouts
│   ├── mission_model/                   <-- Compiled rows, ORBAT projection, authored extension blocks, slot line, typed mission ids
│   ├── mission_crdt/                    <-- Native yrs id arrays, row-aligned slot columns, undo grouping clocks
│   ├── formation_geometry/              <-- Placement patterns, align, space, orient and garrison positions of the arrange commands
│   ├── mission_payload/                 <-- Editor payload, export envelope, version body compiler, kit alias table
│   ├── mission_validation/              <-- Ordered validation rules of an editor payload, their findings, facts and self-check
│   ├── mission_compiler/                <-- Game-document compiler, its compile findings and the compiler identity
│   ├── mission_document/                <-- Mergeable Yjs mission document: rows, hydrate and export, merge, selection, undo
│   └── mission_operations/              <-- Authoring commands and row projections the Mission Creator applies to the document
├── mission_editing/                     <-- The Mission Creator's editing layer over the mission document (no browser code)
│   ├── map_editing_tools/               <-- Headless map tools: selection gesture and picks, ruler, line of sight, viewshed job scheduler
│   ├── mission_editing_commands/        <-- Hosted document commands (ORBAT, layers, markers, zones, triggers, ...) and pure export, report and selection texts
│   ├── mission_editing_session/         <-- Hosted document and its borrow chain, undo drive, grouping, routing, selection, picks, overlay lanes
│   └── mission_persistence/             <-- Local draft decisions: record keys, blob verdicts, merge, local-versus-server, adoption, snapshots
├── ballistics/                          <-- Mortar ballistics: catalog and flight model, firing solver, fire-mission planner, calibration
│   ├── ballistics_model/                <-- Ballistics catalog, shell flight model, surface wind, angular units, typed catalog ids
│   ├── ballistics_solver/               <-- High-angle firing solver per charge, wind-corrected aim, crest clearance, impact dispersion
│   ├── fire_mission_planning/           <-- Fire-mission assembler: battery solutions, time fuzes, client/server comparison, wording
│   ├── ballistics_calibration/          <-- Catalog calibration against the game's native tables, wind tables and engine oracle samples
│   └── ballistics_agreement_cases/      <-- Seeded lattice of battery fire problems and their solution bit patterns (native and wasm32 agreement)
├── api/                                 <-- The API's crates (the product's only sqlx and axum); infrastructure < kernel < domains < workers < the server
│   ├── api_identifiers/                 <-- Infrastructure: serde- and sqlx-transparent typed ids of every table key, Discord snowflake, game runtime key
│   ├── api_foundation/                  <-- Infrastructure: handler error envelope, JSON wire formats, text policies, request parameters
│   ├── api_configuration/               <-- Infrastructure: environment configuration read at boot, trusted proxies, process shutdown signal
│   ├── api_database/                    <-- Infrastructure: Postgres pool, embedded migrations/, development seeds/, SQLSTATE predicates
│   ├── api_http_layer/                  <-- Infrastructure: access tokens, middleware and extractors, rate limiters, metrics and health, realtime hub
│   ├── api_mission_vocabulary/          <-- Kernel: terrain and game mode enums several domains name
│   ├── api_audit_log/                   <-- Kernel: audit severity, best-effort and transactional audit appends
│   ├── api_equipment_datasets/          <-- Kernel: equipment dataset imports, SQLite navigation index, generation-pinned reads
│   ├── api_member_activity/             <-- Kernel: member statistics, leaderboard refresh, attendance attribution, re-evaluation queue
│   ├── api_discord/                     <-- Kernel: Discord OAuth2, guild-member and announcement webhook clients
│   ├── api_caller_identity/             <-- Kernel: role ladder, session and account authority, identity lock order, machine caller
│   ├── api_state/                       <-- Kernel: AppState with its concrete services and FromRef projections
│   ├── api_community_content/           <-- Domain: announcements, wiki, vehicle database, modpacks, uploads, equipment data viewer
│   ├── api_identity_and_access/         <-- Domain: Discord sign-in, session tokens, profile, Arma link handshake, Discord membership
│   ├── api_administration/              <-- Domain: member roster, moderation actions, Discord role resync, audit log console
│   ├── api_server_infrastructure/       <-- Domain: game-server registry, live status SSE, machine credentials, fleet commands, runtime sessions
│   ├── api_missions/                    <-- Domain: missions, versions, artifacts, reviews, deployments, armory, factions, registries
│   ├── api_match_telemetry/             <-- Domain: game-runtime heartbeats, match registration, results revisions, event batches
│   ├── api_operations/                  <-- Domain: events, ORBAT slotting, reservations, service records, fire missions, ballistics catalogs
│   ├── api_command_center/              <-- Domain: dashboard, leaderboards, per-player statistics
│   ├── api_background_workers/          <-- Workers: the interval tasks the API binary arms at boot
│   └── api_server/                      <-- Server: the Axum + sqlx REST API and SSE backend (:8080) the api crates assemble into
│       ├── src/                         <-- lib.rs · router.rs (route tables, mounts, middleware chain) · composition.rs (state with concrete services)
│       │   ├── bin/                     <-- The `api-server` server and the `import-item-registry` tool
│       │   └── tests/                   <-- Executable layout rules (crate graph, domain graph) and prose rules, router pins
│       ├── tests/                       <-- 150 integration binaries, each on its own database
│       └── .env.example                 <-- Local configuration template, copied to the untracked .env
├── fleet/                               <-- The game-server fleet crates over the fleet wire contract
│   └── game_server_host_agent/          <-- Game server host agent per game-server instance: claims fleet commands from the API, runs process control, RCON reads and console lines, mission header switches
├── frontend/                            <-- The single-page app's crates (Leptos only here): foundation < features < pages, workspaces < shell
│   ├── foundation/                      <-- Shared foundations every layer builds on, lowest crate first
│   │   ├── frontend_ui/                 <-- Reusable design system primitives (dialogs, sheets, selects, toasts); time formatting, clipboard, sanitising helpers
│   │   ├── frontend_api_dtos/           <-- API request and response DTOs, the role ladder, the frontend's typed ids
│   │   ├── frontend_transport/          <-- HTTP client, endpoints, SSE subscriber, the token provider seam
│   │   ├── frontend_route_table/        <-- Every route's path, layout flags and access tier; the sidebar's navigation menu
│   │   ├── frontend_map_view/           <-- Shared map mount seam (Mission Creator, mortar map picker), terrain heights
│   │   ├── frontend_session/            <-- Session storage and refresh, sign-out hooks, route guard, content gates
│   │   ├── frontend_offline/            <-- Service worker registration, offline pack download, storage quota, offline state
│   │   └── frontend_test_support/       <-- Source scrubber, captured API responses, repository reads from repository_root's checkout root (dev-dependency only)
│   ├── features/                        <-- Capabilities several pages and workspaces show
│   │   └── mission_review_record/       <-- Shared review record: history, thread, artifact provenance, submit control
│   ├── pages/                           <-- Standard platform document pages, one crate per navigation area
│   │   ├── account_pages/               <-- User login, OAuth callback, settings
│   │   ├── command_center_pages/        <-- Dashboard, announcements, live server intel
│   │   ├── operations_pages/            <-- Event schedule, event detail, full-screen ORBAT slotting, "My Deployments", leaderboards
│   │   ├── mission_hub_pages/           <-- Mission library, overview dossier (briefing, details, armory, review record), "New Mission" dialog
│   │   ├── field_tools_pages/           <-- Mortar calculator: catalog-driven on-device firing solutions, map picker, offline pack
│   │   ├── doctrine_pages/              <-- Markdown doctrine wiki, vehicle identification index, modpack manifests
│   │   └── administration_pages/        <-- Event manager, server control, personnel, approvals, content manager, audit logs, ballistics catalogs
│   ├── workspaces/                      <-- Standalone CAD workspaces & interactive tools
│   │   ├── mission_creator_state/       <-- Mission Creator state: layout tokens, review mode, world-layer prefs, asset catalog and rules, outliner model, zones
│   │   ├── mission_creator_engine_bridge/ <-- Engine seam: boot, viewport and frame timing, hosted mission document, overlays; pointer/keyboard input -> map tools
│   │   ├── mission_creator_session/     <-- Per-tab session: IndexedDB drafts, hydrate, cross-tab lock, save status, conflict dialog
│   │   ├── mission_creator_arsenal/     <-- Loadout domain, gear catalog trees, Arsenal tab, 3D paper doll
│   │   ├── mission_creator_workspace/   <-- Mission Creator page (top-down 2D CAD): docks, outliner, inspectors, modals, canvas mount, read-only review workspace
│   │   └── debug_benches/               <-- URL-only benches: building viewer, building interior, world line of sight, ballistics agreement, data viewer
│   └── shell/                           <-- The shell layer: the single-page app and the offline service worker, two peers
│       ├── frontend_application/        <-- Leptos 0.8 CSR single-page app (Trunk/WASM, :3000) the frontend crates assemble into
│       │   ├── src/                     <-- main.rs (entry point, mounts the shell) · app_routes.rs (the render form of the route table)
│       │   │   ├── shell/               <-- App frame around every route: layout, top nav, sidebar, membership status, not-found page
│       │   │   └── tests/doc_audit/     <-- Documentation audit of every production file of the app and of every frontend crate
│       │   └── style/                   <-- Tailwind v4 stylesheet: one @source line for the app and one per leptos crate
│       └── offline_service_worker/      <-- Rust/WASM service worker: offline pack caches, Range→206 from cache (no JS policy)
├── map_rendering/                       <-- The map's typed GPU layers and its renderer (may name map things; wgpu)
│   ├── map_render_diagnostics/          <-- The renderer's byte-exact readback self-checks, scene readback, frame benchmark, stress pool
│   ├── map_renderer/                    <-- The render engine: GPU context, pipelines, camera, batch list, lane sinks, upload belts, statistics, asset sink
│   ├── symbology_layers_gpu/            <-- Slot symbology (atlas, binds, selection, drag, clusters), glyph atlas, icon lane cull, world icon lanes, lane preferences
│   └── world_layers_gpu/                <-- Building, forest density, satellite and hillshade texture, terrain line of sight overlay layers; textured lane record
├── paper_doll/                          <-- The Arsenal's 3D paper doll: its scene and its renderer
│   ├── paper_doll_scene/                <-- Soldier parts, 14 equipment regions, state colours, unit meshes, orbit-camera picks and callout anchors
│   └── paper_doll_renderer/             <-- wgpu renderer of the doll on its own canvas: damage-driven frames, instance packing, readback self-check
└── graphics/                            <-- Map-agnostic rendering: CPU primitives, the GPU device, the GPU frame and the renderer contracts
    ├── gpu_device/                      <-- GPU context of a canvas (create, resize, acquire), pooled lane buffers, readback guards, frame timer
    ├── gpu_frame/                       <-- Frame vocabulary, draw encoding, compute sprite cull, render pipelines, animation-frame pump
    ├── renderer_core/                   <-- Renderer contracts: lane sink, layer context, frame hooks, render statistics and their JSON, packet binding ids
    └── render_primitives/               <-- Instance layouts, geometry, triangulation, CPU cull oracle, frame ids, text atlas, the WGSL shader

deploy/                                  <-- Release Dockerfile (context narrowed by the root .dockerignore), dev and staging compose files, deploy.env.example
├── caddy/                               <-- Caddy site on :3080; the one folder the staging Caddy container mounts
└── systemd/                             <-- User units and timers: API, game-server fleet, host agents, relay, database backups

tools/                                   <-- Every developer tool in the repository; the tool crates by category plus one npm package
├── foundation/                          <-- The tools' tiered base crates
│   ├── verification_core/               <-- Fail-closed verdicts, pattern scans, gates, the repository verification lock
│   ├── process_runner/                  <-- Child processes (deadlines; terminal, binary, file, detached and streaming modes), host-bridge execution, the secure shell transport
│   ├── repository_laws/                 <-- Every repository law: crate tiers, anatomy, test-file reachability, frontend layering, Tailwind sources, file length, test placement, test-only features
│   ├── repository_layout/               <-- The paths every tool shares; its prelude re-exports repository_root's checkout-root finder
│   ├── deploy_settings/                 <-- The one reader of deploy/deploy.env and its precedence over exported variables
│   └── tool_test_support/               <-- Test locks and the checkout root the tool crates' tests share (dev-only)
├── tickets/                             <-- Ticket crates
│   ├── ticket_model/                    <-- Typed ticket, its canonical TOML encoding, the corpus store
│   ├── ticket_metrics/                  <-- Slice-run receipts and token estimates
│   ├── ticket_wave_lock/                <-- Wave lock compiler, reader and checker
│   ├── ticket_registry/                 <-- Ticket operations, validation, queue and roadmap sync, the `ticket` verbs
│   ├── ticketboard_model/               <-- The ticketboard's headless half: models, events, egui-free application state
│   └── ticketboard_desktop/             <-- Native egui/eframe desktop viewer for .ai/tickets over ticketboard_model
├── commands/                            <-- Command crates behind the xtask groups
│   ├── agent_context_guards/            <-- AI agent tool-call guard (Bash and Read rules, the session read set) and the filtered command runner (`ai`)
│   ├── ci_task_catalog/                 <-- CI task table and runner (`ci`, `help`), build lane recipes (`mk`), cargo target pin, CI workflow checks, map asset checks
│   ├── platform_execution/              <-- Platform factory (`platform`): wave driver, slice runs, slice worktrees, preflight
│   ├── mod_operations/                  <-- Game mod operations (`mod`): compile gate, world boot, playtest server, equipment export, mod wave driver
│   ├── database_operations/             <-- Local database lane (`db`), database container layer, verified backup, guarded restore, restore drill (`deploy db`), seed checks
│   ├── deployment/                      <-- Website and staging fleet deploys (`deploy website`, `deploy staging`), staging compose-path check
│   ├── staging_procedures/              <-- Staging acceptance harness (`staging`): fleet, Discord and load procedures, their receipts, host actions
│   ├── api_readiness_checks/            <-- API readiness judge (`verify api-readiness`): acceptance register, evidence receipts, fingerprints, property-test seed
│   ├── schema_tooling/                  <-- Contract codegen, schema gates, ORBAT slot flattening, font table (`schema`, `gen`)
│   ├── repository_relocation/           <-- Manifest-driven moves and the retired-spelling verification (`refactor relocate`)
│   ├── enfusion_mcp/                    <-- Enfusion MCP client (`mcp`): daemon control, tool calls, offline selftest, Workbench NET API calls
│   ├── ballistics_oracle_tooling/       <-- Ballistics catalog and calibration fixtures (`ballistics`)
│   ├── workstation_setup/               <-- Workstation setup (`setup`) and the staging host check
│   └── remote_debugging/                <-- Staging join probes (`debug`), remote console log verdict, upload reproduction (`repro`)
├── checks/                              <-- Check crates behind `cargo xtask verify`
│   ├── repository_checks/               <-- Workspace laws, route tags, editor ORBAT coherency, language bans, licensing, registry aliases, tooling rules
│   ├── mod_script_checks/               <-- Enfusion comment card, mod script pins, UI layout gate, Workbench spawn runs
│   └── documentation_checks/            <-- README coverage, Markdown placement and link-check gates
├── enfusion/                            <-- Enfusion tool crates
│   ├── enfusion_pak/                    <-- The `.pak` archive reader and its merged virtual file system
│   ├── enfusion_script_index/           <-- The script oracle behind `enf` and the vanilla page mirrors behind `fetch`
│   └── enfusion_mcp_broker/             <-- The `mcpd` broker over one enfusion-mcp server
├── browser_testing/                     <-- Browser gate crates
│   ├── chrome_devtools_protocol/        <-- DevTools protocol client: Chromium discovery and launch, pages, the gate font cache
│   └── browser_gate_suites/             <-- Gate suites and the `gate` and `capture` command lines: static server, DOM oracle, route drift, editor smokes, ballistics and offline mortar gates
│       └── fixtures/dom_oracle/         <-- DOM goldens, screenshots and route inventories the browser gates compare against
├── staging/                             <-- Staging crates
│   ├── staging_load_plan/               <-- The member load's plan, request catalog, pacing and report, without tokio
│   ├── staging_load_generator/          <-- The member load's virtual clients behind `staging-load`
│   ├── staging_fixtures/                <-- The `staging-fixtures` host tool: staging accounts, fleet credentials, fixture events and its database suites
│   └── acknowledgement_dropping_relay/  <-- Relay that withholds one fleet executor answer
├── map_assets/                          <-- Map asset crates
│   ├── blueprint_compiler/              <-- Building blueprints from voxel dumps and game models, occlusion sidecars, the blueprint archive
│   ├── map_asset_verification/          <-- Gates over a terrain's committed map assets and the map goldens; the world line-of-sight probe
│   ├── map_raster_pipeline/             <-- The `map` pipeline: a terrain's satellite container, tile pyramids, cartographic render, labels, water archives, the glyph atlas, Workbench water and road export images
│   └── world_export_pipeline/           <-- The `world` pipeline: a terrain's chunks, catalogue, census, density, regions, roads and elevation from a Workbench export, and their gates
├── developer_tools/                     <-- The eight tool binaries, each a one-line `main` over one tool crate (no library)
│   └── src/bin/                         <-- Executables: enf, gate, mcpd, world, map, capture, acknowledgement-dropping-relay, staging-load
│       ├── enf                          <-- Symbol indexes, lookups and checks over Enfusion scripts
│       ├── gate                         <-- Headless CDP Chrome gates of the single-page app
│       ├── mcpd                         <-- Enfusion MCP broker daemon
│       ├── world                        <-- World-export pipeline and its verification gates
│       ├── map                          <-- Satellite, cartographic, label, water and glyph map assets
│       ├── capture                      <-- Mission Creator screenshots, zoom sweeps, crops
│       ├── acknowledgement-dropping-relay <-- Staging fault injection: withholds one fleet executor answer
│       └── staging-load                 <-- Staging member load: a plan as JSON in, its report as JSON out
├── xtask/                               <-- `cargo xtask` command line and dispatch, plus the `ai`, `fetch`, `map`, `refactor`, `schema`, `ticket`, `verify` and `wave` groups; no tokio, axum, reqwest, resvg or image in its dependency closure
│   ├── dedicated_server_profiles/       <-- Dedicated-server profile the local mod servers start from
│   ├── fixtures/mcp/                    <-- Recorded MCP transcripts `cargo xtask mcp selftest` replays
│   └── staging/                         <-- Committed load workload and population of the staging load receipt
└── enfusion_mcp_node_package/           <-- Pinned enfusion-mcp npm package (node_modules gitignored)

contracts/                               <-- Every shape that crosses a network, process, or language boundary
├── definitions/                         <-- Authoritative JSON Schemas (missions, events, registry, loadouts, map objects, terrain, fleet)
├── rules/                               <-- Prefab classification and mission kit aliases
├── catalogs/                            <-- Live Workbench exports the platform ingests; ballistics catalogs
└── fixtures/                            <-- Golden test data, positive and negative; recorded API responses in api_goldens/

assets/                                  <-- Terrain datasets and the world-object glyph set
├── terrains/                            <-- Built-in islands (Everon, Arland) and the terrain registry, served at /map-assets
├── glyphs/                              <-- World-object glyph atlas and SVG sources
├── scratch/                             <-- Local export intermediates (gitignored)
└── storage_spec/                        <-- Production persistent volume specification (not yet built)

documentation/                           <-- All documentation; entry, map and authority ladder: README.md
├── architecture/                        <-- Workspace layout as it stands: top-level folders, members, where code, contracts, assets and docs live
├── crates/ mod/ tools/ contracts/ assets/
│                                        <-- Feature docs at the code's path minus src/ (mod docs drop Scripts/Game/TBD/)
├── relocation_manifests/                <-- Manifests of every tracked move (`cargo xtask refactor relocate`), the format and the retired-spelling registry
├── runbooks/                            <-- Procedures: local development, deployment, gates, playtests
├── standards/                           <-- Documentation, README and coding standards; document templates
├── glossary/                            <-- Project terms, one file per letter range
├── design_system/                       <-- Design tokens, map symbology, interaction patterns
├── known_bugs/                          <-- Live known-bug registry
├── tickets/                             <-- Ticket specs and plans (flat; frozen once the ticket closes)
├── archive/                             <-- Frozen history, one folder per topic (finished program records such as the workspace restructure, superseded layout plans, research, executed agent briefs)
└── product_roadmap.md                   <-- Planned product items and open product questions
.ai/tickets/                             <-- Ticket registry: one TOML per ticket, queue.json, ticket templates
```

---

## 3. Canonical Commands (`cargo xtask`)

Configuration lives in `crates/api/api_server/.env`, copied from `crates/api/api_server/.env.example` (`APP_ENV=development`, Postgres on port 5434). Step-by-step setup: [local development](/documentation/runbooks/local_development.md).

```bash
# Local stack, in this order (Postgres :5434)
cargo xtask db up              # Start local Postgres container
cargo xtask mk rust-api        # Axum API on :8080 (applies pending migrations on boot)
cargo xtask db seed            # Apply the five development SQL seeds (needs the migrated tables)
cargo xtask mk leptos          # Leptos SPA on :3000 (trunk serve --release; proxies /api and /map-assets to :8080)

# Database
cargo xtask db down            # Stop local Postgres container (keeps volume)
cargo xtask db repair-migration-checksum [--version N]  # Repoint the checksums of comments-only edits to applied migrations

# Quality Gates & Testing
cargo xtask mk rust-fmt        # Format check (pre-commit)
cargo xtask mk rust-clippy     # Clippy with warnings denied (pre-commit)
cargo xtask ci ci-local        # Replay the CI check suite locally
cargo xtask mk ci-local-leptos # Frontend checks: fmt, clippy wasm32, test, trunk release build
cargo xtask db test-it         # Rust backend integration tests (requires db up)
cargo xtask mod compile        # Compile check Enfusion mod scripts

# On demand / nightly (not pre-commit)
cargo xtask mk leptos-gates    # Full headless Chrome CDP editor gates (runs gate doctor first)
cargo xtask verify link-check  # Links, anchors, backticked paths and cited commands resolve (add --path <folder> to narrow)

# Ticket Registry
cargo xtask ticket check       # Validate ticket registry structure
cargo xtask ticket next        # Show the active slice and the next five ready or queued tickets
cargo xtask ticket sync        # Regenerate queue.json and the roadmap next-work block (the gap-analysis ticket column is kept by hand)

# Deployment (deploy/deploy.env)
cargo xtask deploy website --dry-run  # Print the plan: asset preflight, rsync excludes, remote steps
cargo xtask deploy website     # Rsync, build the API + SPA on the server, restart the unit
cargo xtask deploy staging     # Five game-server instances, their host agents and the relay on the staging host

# Staging verification (documentation/runbooks/staging_verification/)
cargo xtask staging preflight  # Read-only: every precondition of the fleet, Discord and load procedures
cargo xtask staging action-list <fleet|discord|load>  # The numbered real actions a procedure run asks approval for
cargo xtask staging load --rehearse-local  # The load path against the local stack; records nothing
cargo xtask staging <fleet|discord|load> --record  # Run a procedure and write its operational receipt
```

### Dev Login (No Discord Required)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=guest|enlisted|leader|mission_maker|admin` (any other or missing role signs in as `admin`). Open in browser or read `access_token` from the 302 `Location` fragment (`/auth/callback#…`) for API testing.
