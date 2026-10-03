# TBD Reforger Platform

Platform suite for the "TBD" Arma Reforger milsim community: Discord auth, event / ORBAT scheduling, mission library, the Mission Creator (a top-down 2D mission editor), game-server fleet control and telemetry, leaderboards, doctrine wiki, the TBD game mod, and Enfusion mod tooling.

> **Active program — workspace restructure.** The repository is being rebuilt into standard, fine-grained Rust crates (flat `apps/`, tiered crates, no `_v2` names). Before any work, read [documentation/restructure/README.md](/documentation/restructure/README.md), then [progress.md](/documentation/restructure/progress.md) for the current stage and the next step. Commits land directly on `main` (law 2), one green commit per stage.

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
   - `graphics_engine` (`legacy/graphics_engine/`): Pure GPU rendering primitives (pipelines, shaders, draw batching). Knows **zero** map concepts.
   - `map_engine` (`legacy/map_engine/`): Map graphics, spatial computation, terrain formats, asset streaming, camera math, and the editing seam over the mission crates (the mission domain itself — compilation, validation, Yjs CRDT document model — lives in `crates/mission/`). Speaks graphics engine frame vocabulary; zero UI/Leptos dependencies.
   - `frontend` (`apps/frontend/`): Presentation, navigation, and CAD workspaces, in five layers under `src/` (foundation < features < pages, workspaces < shell); consumes engine crates.
   - `api` (`apps/api/`): Axum REST API and SSE realtime hub.
7. **File Size Limits & Test Placement (Hard Ceilings — Zero Exemptions)**:
   - Production files must stay **at or under 500 lines**.
   - Test files (inside a `tests/` folder or named `*_tests.rs`) must stay **at or under 1000 lines**.
   - The ceilings apply in every language; `cargo xtask verify file-length` enforces them on the Rust source trees and the pinned mod Scripts roots (`apps/mod/tbd-framework/Scripts`, `apps/mod/tbd-emcp/Scripts`) by raw line count.
   - **Zero Exemptions / No Allowlist**: There is NO allowlist file and NO exemption mechanism. Never create an allowlist (`.coding-standards-allowlist.yaml` or any other), use allowlist comments, or bypass these limits. If a file approaches or exceeds 500 lines, you MUST decompose it by responsibility into cohesive submodules.
   - **No inline test modules**: Unit tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
8. **In-Code Documentation Standards (Rust & Enfusion)**:
   - **Present-Tense, Context-Free Invariants (Universal)**:
     Comments and docstrings must describe strictly what the code does *now* and *why* (invariants, mathematical models, engine/hardware constraints, failure modes). Never document historical transitions (no "rewritten from X", "fixed in Y"), ticket references in source comments, or references to retired codebases (no "mirrors old TS file"). Commit history owns history.
   - **Rust Code Standards (`rustdoc` — `apps/`, `crates/`, `legacy/`, `tools/`)**:
     - **Module Headers (`//!`)**: Non-trivial modules must carry a 4-point architectural contract header:
       - `**Role:**` Primary responsibility of this module in the subsystem.
       - `**Position:**` Boundary layer and data flow context (what feeds it, who consumes it).
       - `**Signals & state:**` Mutable state, reactive signals, or thread ownership (or "none; pure functions").
       - `**Invariants:**` Non-negotiable structural guarantees and mathematical boundaries.
     - **Symbol Docs (`///`)**: Public types, functions, methods, and enums must use standard markdown docstrings with validated intra-doc links (e.g. `[`crate::path::Type`]`).
     - **Cross-Boundary Tags**:
       - Axum HTTP handlers MUST declare `/// @route <METHOD> <path>` (machine-checked by `cargo xtask verify route-tags`).
       - Schema-projecting DTOs and models MUST declare `//! @contract <schema>#<pointer>` (machine-checked by `cargo xtask schema citations`).
   - **Enfusion Mod Standards (`Doxygen` — `apps/mod/`)** (banners, headers, member docs and tags machine-checked by `cargo xtask verify enfusion-comments` over the pinned mod Scripts roots):
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
   - Backend Rust models (`apps/api/src/<domain>/models/`) are the snake_case API source of truth.
   - Contract types are generated from `contracts/definitions/*.json` via `cargo xtask ci schema-codegen`.
   - Frontend DTOs (`apps/frontend/src/foundation/transport/dto/`) mirror models with strict R-api golden test parity.
10. **Documentation Ships With the Code**:
    - Documentation lands in the same commit as the code it describes, whichever agent writes that code: the comments of the code it alters, the README.md of every folder whose contents, surface, commands or boundaries change, and the feature docs whose behaviour changes.
    - [documentation/README.md](/documentation/README.md) is the documentation entry (map and authority ladder); [documentation/standards/](/documentation/standards/README.md) holds the documentation, README and coding standards and the templates.
    - Terms follow the [glossary](/documentation/glossary/README.md): the editor is the **Mission Creator**; the authored document is a **mission** (never "scenario" in prose; code identifiers stay quoted as spelled); Enfusion's world plus game-mode config is the **mission header**; an **event** is a scheduled session record; **operations** is its domain.
    - Before committing, run the three documentation gates over what changed (§3).

---

## 2. Monorepo Directory Atlas

The [workspace layout](/documentation/architecture/workspace_layout.md) explains the top-level folders and the workspace members as they stand; the [target file tree](/documentation/restructure/target_file_tree.md) is where the restructure takes them.

```text
apps/
├── api/                                 <-- Axum + sqlx REST API and SSE backend (:8080)
│   └── src/                             <-- Domain-driven backend: core + workers + eight domains
│       ├── core/                        <-- Configuration, database, state, errors, router, middleware, observability, realtime hub, auth primitives
│       ├── background_workers/          <-- Interval tasks the API binary arms at boot
│       ├── bin/                         <-- The `api` server, the `import-registry` tool and the `staging-fixtures` host tool
│       ├── administration/              <-- Member roster, moderation actions, Discord role resync, audit log
│       ├── command_center/              <-- Dashboard, leaderboards, per-player statistics
│       ├── community_content/           <-- Announcements, wiki, vehicle database, modpacks, uploads
│       ├── identity_and_access/         <-- Discord OAuth2, session tokens, profile, Arma link handshake
│       ├── match_telemetry/             <-- Game-runtime heartbeats and finished match results
│       ├── missions/                    <-- Missions, versions, artifacts, reviews, deployments, armory, registries
│       ├── operations/                  <-- Events, ORBAT slotting, reservations, service records, fire missions, ballistics catalogs
│       ├── server_infrastructure/       <-- Game-server registry, live status SSE, machine credentials, fleet commands, runtime sessions
│       └── tests/architecture_rules.rs  <-- Executable layout rules checked against src/
├── frontend/                            <-- Leptos 0.8 CSR single-page app (Trunk/WASM, :3000)
│   └── src/                             <-- Five layers in one crate: foundation < features < pages, workspaces < shell
│       ├── main.rs, app_routes.rs       <-- Entry point (mounts the shell) and the render form of the route table
│       ├── foundation/                  <-- Shared foundations every layer builds on
│       │   ├── transport/               <-- HTTP client, DTOs and the role ladder, endpoints, SSE subscriber
│       │   ├── auth/                    <-- Session storage and refresh, sign-out hooks, route guard, content gates
│       │   ├── route_table/             <-- Every route's path, layout flags and access tier; the sidebar's navigation menu
│       │   ├── map_view/                <-- Shared map mount seam (Mission Creator, mortar map picker), terrain heights
│       │   ├── offline/                 <-- Service worker registration, offline pack download, storage quota, offline state
│       │   ├── ui/                      <-- Reusable design system primitives (dialogs, sheets, selects, toasts)
│       │   ├── utils/                   <-- Time formatting, clipboard, sanitising helpers
│       │   └── test_support/            <-- Source scrubber, captured API responses, source pins (tests only)
│       ├── features/                    <-- Capabilities several pages and workspaces show
│       │   └── mission_review_record/   <-- Shared review record: history, thread, artifact provenance, submit control
│       ├── pages/                       <-- Standard platform document pages, one folder per navigation area
│       │   ├── account/                 <-- User login, OAuth callback, settings
│       │   ├── command_center/          <-- Dashboard, announcements, live server intel
│       │   ├── operations/              <-- Event schedule, event detail, slotting, deployments, leaderboards
│       │   │   ├── schedule/            <-- Upcoming events with the selected event's hub
│       │   │   ├── event_detail/        <-- Event briefing dossier and slot signups
│       │   │   ├── orbat_selection/     <-- Dedicated full-screen slotting view
│       │   │   ├── deployments/         <-- "My Deployments": the viewer's service record and upcoming slots
│       │   │   └── leaderboards/        <-- Community player rankings with a slide-over dossier
│       │   ├── mission_hub/             <-- Mission library, overview dossier, create dialog
│       │   │   ├── library/             <-- Filterable community mission catalog
│       │   │   ├── overview/            <-- Mission dossier: briefing, details, armory, review record
│       │   │   └── create_dialog/       <-- "New Mission" dialog that opens the Mission Creator
│       │   ├── field_tools/             <-- Interactive tactical utilities
│       │   │   └── mortar/              <-- Mortar calculator: catalog-driven on-device firing solutions, map picker, offline pack
│       │   ├── doctrine_and_info/       <-- Knowledgebase and reference catalogs
│       │   │   ├── wiki/                <-- Markdown tactical doctrine and rules articles
│       │   │   ├── vehicles/            <-- Vehicle identification index and dossiers
│       │   │   └── modpacks/            <-- Modpack manifests and Workshop collection links
│       │   └── administration/          <-- Management and administrative control panels
│       │       ├── event_manager/       <-- Event scheduling and operations calendar admin
│       │       ├── server_control/      <-- Game-server state, fleet commands, mission deployment, host credentials
│       │       ├── personnel/           <-- Member roster, rank, and permission management
│       │       ├── approvals/           <-- Mission submission review and approval queue
│       │       ├── content_manager/     <-- Announcement authoring ("Comms Broadcaster")
│       │       ├── audit_logs/          <-- Audit trail of administrative actions
│       │       └── ballistics_catalogs/ <-- Calibrated ballistics catalog uploads and stored versions
│       ├── workspaces/                  <-- Standalone CAD workspaces & interactive tools
│       │   ├── editor/                  <-- Mission Creator: top-down 2D CAD workspace (3D only in the Arsenal paper doll)
│       │   │   ├── mission_editor/      <-- Route component parts: canvas mount, page effects, registry loading, transforms
│       │   │   ├── ui/                  <-- CAD docks, outliner, inspectors, Arsenal tab, modals
│       │   │   ├── input/               <-- DOM pointer/keyboard events -> map engine commands and tools
│       │   │   ├── bridge/              <-- Engine seam: boot, viewport and frame timing, hosted mission document, overlays
│       │   │   ├── session/             <-- Per-tab session: IndexedDB drafts, hydrate, cross-tab lock, review mode, layout prefs
│       │   │   ├── review_workspace/    <-- Mission Creator opened read-only on a submitted version
│       │   │   └── arsenal/             <-- Loadout domain, gear catalog trees, 3D paper doll
│       │   ├── planner/                 <-- Reserved for the mission planner whiteboard (README only, no code)
│       │   ├── aar/                     <-- Reserved for the after-action review replay (README only, no code)
│       │   └── debug/                   <-- URL-only benches: building viewer, building interior, world line of sight, ballistics agreement
│       ├── shell/                       <-- App frame around every route: layout, top nav, sidebar, membership status, not-found page
│       └── tests/doc_audit/             <-- Documentation audit of every production file of the crate
├── offline_service_worker/              <-- Rust/WASM service worker: offline pack caches, Range→206 from cache (no JS policy)
├── fleet_host_agent/                    <-- Agent per game-server instance: claims fleet commands from the API, runs process control, RCON reads and console lines, mission header switches
├── mod/                                 <-- Enfusion engine mod suite (three addons)
│   ├── tbd-framework/                   <-- Shipping game mod (TBD_Framework; depends on vanilla only)
│   │   ├── Configs/                     <-- Menu presets, input contexts, key actions
│   │   ├── Data/                        <-- Alias spawn registry, backend config template
│   │   ├── Missions/                    <-- Mission headers a server boots (TBD Dev POC on Everon)
│   │   ├── Prefabs/                     <-- Game mode with its manager components, player controller
│   │   ├── worlds/                      <-- TBD Dev POC world and the layer that places the game mode
│   │   ├── Scripts/Game/TBD/            <-- Gameplay scripts (compiled into game runtime)
│   │   │   ├── API/                     <-- Platform bridge: game-runtime session, fleet commands, identity links, results
│   │   │   ├── Core/                    <-- Structured log, prefab registry, player chat, SHA-256
│   │   │   ├── Gamemode/                <-- Stage machine, safe start, objectives, tasks, end conditions
│   │   │   ├── Session/                 <-- Mission selection, lobby, briefing, spectator, post-game
│   │   │   ├── Systems/                 <-- Mission load, spawns, loadouts, zones, AI, audio, markers, radio
│   │   │   └── UI/                      <-- Menu framework, components, HUD, mock catalogs
│   │   └── UI/                          <-- UI layout definitions and textures
│   │       ├── Textures/                <-- Rounded-shape disc, hero art, masks, icons
│   │       └── layouts/                 <-- Enfusion widget layout files (.layout)
│   │           ├── Common/              <-- Reusable design system primitives (panels, rows, chips, inputs)
│   │           ├── Hud/                 <-- Objective panel shown during live play
│   │           └── Session/             <-- Pre-game screens, post-game overlays, shared bars and panels
│   ├── tbd-export/                      <-- Workbench export addon (TBD_Export; depends on vanilla + TBD_EMCP, not the framework)
│   │   ├── Missions/                    <-- Export mission header (Everon export world)
│   │   ├── Prefabs/                     <-- Export game mode carrying the road export component
│   │   ├── worlds/                      <-- Standalone export world over vanilla Eden
│   │   └── Scripts/
│   │       ├── Game/TBD/Export/         <-- Runtime road-network export component; ballistics oracle play-mode simulation run
│   │       └── WorkbenchGame/           <-- Workbench export plugins (MapExport, EquipmentExport, EquipmentVehicleExport, VehicleExport, BallisticsOracle, registry)
│   ├── tbd-emcp/                        <-- Enfusion MCP bridge handler scripts (TBD_EMCP)
│   │   └── Scripts/WorkbenchGame/EnfusionMCP/ <-- 19 committed NetAPI automation handlers
│   └── References/                      <-- Licensed upstream reference lanes, gitignored except README
│       ├── crf_framework/               <-- Upstream Coalition Reforger Framework scripts and assets
│       ├── vanilla_reference/           <-- Extracted vanilla Reforger scripts and Script API pages
│       └── playable_selector/           <-- PlayableSelector checkout (design-mirror only)
└── ticketboard/                         <-- Native egui/eframe desktop viewer for .ai/tickets

legacy/                                  <-- Parking folder of the two engine monoliths while their code moves into crates/; no new crate depends on it
├── map_engine/                          <-- World, spatial computation, formats, and the mission editing seam
│   └── src/
│       ├── editing/                     <-- Live mission document, undo history, headless map tools (select, ruler, LOS, viewshed)
│       ├── world/                       <-- Scene calibration and the GPU and loader parts of terrain, buildings, vegetation, labels (CPU in crates/terrain, crates/world_objects)
│       ├── spatial/                     <-- The terrain viewshed's GPU overlay (line of sight is in crates/line_of_sight)
│       ├── camera/                      <-- The browser viewport the map camera fills (the camera math is in crates/geometry)
│       ├── streaming/                   <-- Served map data: fetch, world chunk residency, draw buffers, memory budget
│       ├── overlay/                     <-- Lane preferences and the GPU symbol bridges (the overlay CPU is in crates/map_overlay)
│       │   └── symbology/               <-- GPU symbol atlas and slot, vehicle and icon instance bridges
│       ├── frame/                       <-- Render engine: builds graphics_engine::frame packets, upload belts
│       ├── doll/                        <-- Arsenal 3D mannequin preview: scene, picking, renderer
│       ├── shaders/                     <-- The doll renderer's WGSL program
│       └── diagnostics/                 <-- Readback checks, benchmarks, timing, probes
└── graphics_engine/                     <-- Pure GPU rendering primitives (knows zero map concepts)
    └── src/
        ├── device/                      <-- Pooled per-lane GPU buffers, readback fences
        ├── draw/                        <-- Line and polygon uploads, GPU culling, the frame encoder
        ├── frame/                       <-- Frame packet, batches, present, atlases
        ├── loop/                        <-- Shared requestAnimationFrame pump and its FrameTarget trait
        └── pipeline/                    <-- Render pipeline constructors

crates/                                  <-- Library crates grouped by category (crates/<category>/<crate>); every manifest under it is a workspace member declaring its tier
├── foundation/                          <-- Leaf crates with no workspace dependency
│   ├── http_url_guard/                  <-- The HTTP(S) URL check the API and the single-page app share, with its one case table
│   ├── newtype_ids/                     <-- Macros declaring serde-transparent typed ids (string, integer, uuid; an sqlx form expanded at the call site)
│   ├── time_source/                     <-- Wall-clock and monotonic time sources, RFC 3339 UTC formatting and validation
│   ├── deterministic_random/            <-- The seeded SplitMix64 generator
│   ├── content_digest/                  <-- SHA-256 and SHA-384 hex digests, framed hashing
│   └── browser_platform/                <-- Browser console macros and fetch helpers (wasm32 only)
├── contracts/                           <-- Crates that hold one boundary contract
│   ├── offline_cache_policy/            <-- Offline cache names, request classes, offline pack and network fallback rules the service worker applies
│   ├── fleet_wire_contract/             <-- Fleet-command wire shapes, executor kinds, the machine-credential format and secret-file limits
│   └── contract_schema_types/           <-- Rust types generated from contracts/definitions (`cargo xtask ci schema-codegen`)
├── geometry/                            <-- Engine geometry: vectors, segments, rigid transforms, map coordinates, cameras, spatial indexes
│   ├── geometry_primitives/             <-- 3D vector ops, 2D segment geometry, rigid transforms, axis-aligned boxes
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
│   ├── road_network/                    <-- Road segments and class codec, styling and zoom gates, road meshes, cartographic strips, airfield
│   └── water_bodies/                    <-- Bathymetry water mask, level suffix plan, inland water archive, sea fill mesh
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
├── ballistics/                          <-- Mortar ballistics: catalog and flight model, firing solver, fire-mission planner, calibration
│   ├── ballistics_model/                <-- Ballistics catalog, shell flight model, surface wind, angular units, typed catalog ids
│   ├── ballistics_solver/               <-- High-angle firing solver per charge, wind-corrected aim, crest clearance, impact dispersion
│   ├── fire_mission_planning/           <-- Fire-mission assembler: battery solutions, time fuzes, client/server comparison, wording
│   ├── ballistics_calibration/          <-- Catalog calibration against the game's native tables, wind tables and engine oracle samples
│   └── ballistics_agreement_cases/      <-- Seeded lattice of battery fire problems and their solution bit patterns (native and wasm32 agreement)
└── graphics/                            <-- Map-agnostic CPU rendering primitives
    └── render_primitives/               <-- Instance layouts, geometry, triangulation, CPU cull oracle, frame ids, text atlas, the WGSL shader

deploy/                                  <-- Release Dockerfile (context narrowed by the root .dockerignore), dev and staging compose files, deploy.env.example
├── caddy/                               <-- Caddy site on :3080; the one folder the staging Caddy container mounts
└── systemd/                             <-- User units and timers: API, game-server fleet, host agents, relay, database backups

tools/                                   <-- Every developer tool in the repository; the tool crates plus one npm package
├── foundation/                          <-- The tools' tiered base crates
│   ├── verification_core/               <-- Fail-closed verdicts, pattern scans, gates, the repository verification lock
│   ├── process_runner/                  <-- Process isolation, deadlines, host-bridge execution, the secure shell transport
│   ├── repository_laws/                 <-- Every repository law: crate tiers, anatomy, strangler, engine layers, file length
│   └── repository_layout/               <-- The repository root finder and the paths every tool shares
├── ticket_engine/                       <-- Ticket storage, validation, queue and roadmap sync, wave lock, metrics
├── developer_tools/                     <-- Heavy async CLI suite, blueprint compiler, map verification
│   ├── src/bin/                         <-- Executables: enf, gate, mcpd, world, map, capture, acknowledgement-dropping-relay
│   │   ├── enf                          <-- Symbol indexes, lookups and checks over Enfusion scripts
│   │   ├── gate                         <-- Headless CDP Chrome gates of the single-page app
│   │   ├── mcpd                         <-- Enfusion MCP broker daemon
│   │   ├── world                        <-- World-export pipeline and its verification gates
│   │   ├── map                          <-- Satellite, cartographic, label, water and glyph map assets
│   │   ├── capture                      <-- Mission Creator screenshots, zoom sweeps, crops
│   │   └── acknowledgement-dropping-relay <-- Staging fault injection: withholds one fleet executor answer
│   ├── fixtures/dom_oracle/             <-- DOM goldens, screenshots and route inventories the browser gates compare against
│   └── test_fixtures/blueprint/         <-- Prefab and world-object inputs for the blueprint compiler tests
├── xtask/                               <-- `cargo xtask` command router, repository verifications, platform execution
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
├── apps/ legacy/ mod/ tools/ contracts/ assets/
│                                        <-- Feature docs at the code's path minus src/ (mod docs drop apps/ and Scripts/Game/TBD/)
├── restructure/                         <-- Active workspace restructure program: plan, target tree, crate catalogue, relocation manifests, shared agent brief, progress
├── runbooks/                            <-- Procedures: local development, deployment, gates, playtests
├── standards/                           <-- Documentation, README and coding standards; document templates
├── glossary/                            <-- Project terms, one file per letter range
├── design_system/                       <-- Design tokens, map symbology, interaction patterns
├── known_bugs/                          <-- Live known-bug registry
├── tickets/                             <-- Ticket specs and plans (flat; frozen once the ticket closes)
├── archive/                             <-- Frozen history, one folder per topic (finished program records, superseded layout plans, research, executed restructure agent briefs)
└── product_roadmap.md                   <-- Planned product items and open product questions
.ai/tickets/                             <-- Ticket registry: one TOML per ticket, queue.json, ticket templates
```

---

## 3. Canonical Commands (`cargo xtask`)

Configuration lives in `apps/api/.env`, copied from `apps/api/.env.example` (`APP_ENV=development`, Postgres on port 5434). Step-by-step setup: [local development](/documentation/runbooks/local_development.md).

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
cargo xtask ci ci-local        # Replay full CI check suite locally
cargo xtask mk ci-local-leptos # Frontend checks: fmt, clippy wasm32, test, trunk release build
cargo xtask mk leptos-gates    # Full headless Chrome CDP editor gates (runs gate doctor first)
cargo xtask db test-it         # Rust backend integration tests (requires db up)
cargo xtask mod compile        # Compile check Enfusion mod scripts
cargo xtask verify enfusion-comments  # Enfusion comment card (headers, banners, @authority/@rpc/@replicated, @route/@contract) over the pinned mod Scripts roots

# Documentation Gates (add --path <folder> to narrow)
cargo xtask ci verify-documentation    # All three over the committed tree (a ci-local step; ci.yml language-gates runs them)
cargo xtask verify readme-coverage     # Every README.md Contents block matches its folder's tracked children
cargo xtask verify link-check          # Links, anchors, backticked paths and cited commands resolve
cargo xtask verify markdown-placement  # No Markdown but README.md in code trees; live docs at or under 500 lines

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
