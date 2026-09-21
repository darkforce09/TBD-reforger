# TBD Reforger Platform

Platform suite for the "TBD" Arma Reforger milsim community: Discord auth, event / ORBAT scheduling, mission library, 2D/3D CAD scenario editor, server telemetry, leaderboards, doctrine wiki, and Enfusion mod tooling.

---

## 1. Core Project Laws

1. **Hard Gate — No Silent Deferrals**:
   Do the whole ask. Never invent "out of scope", "deferred", or ship an MVP and call the task done unless the operator explicitly specifies to defer that piece ("defer X", "skip X"). Rule: `.cursor/rules/no-silent-deferrals.mdc`.
2. **Git Discipline — Direct to Main**:
   Never create git branches (`git checkout -b` is forbidden). All commits land directly on `main`. Merge and delete any stray branch immediately.
3. **Fundamentals & Clean Architecture Over Hacks**:
   Never tack on ad-hoc code to "just make it work". If a clean solution requires an architectural adjustment or structural refactoring, plan and execute that refactor cleanly. Understandability, simplicity, and long-term maintainability strictly supersede quick patches.
4. **Zero Context Needed for Directory & File Names**:
   Every folder, file, module, and symbol name must be so clear and self-describing that anyone can immediately and unmistakably understand its purpose without needing *any* prior project or historical context. Avoid cryptic abbreviations, project-specific jargon, and overloaded names.
5. **Categorize Variants & Primitives (Avoid Flat Dumps)**:
   Avoid flat dumping of dozens of files or variant variations into a single folder. Related variants, numerical sets (e.g. column counts, rounded radius variants), and functional primitives should be grouped into dedicated, well-named subfolders to maintain clean directory comprehension.
6. **Strict Boundary Layers**:
   - `website-graphics-engine`: Pure GPU rendering primitives (pipelines, shaders, draw batching). Knows **zero** map concepts.
   - `website-map-engine`: Map graphics, spatial computation, terrain formats, asset streaming, camera math, and the mission domain (compilation, validation, Yjs CRDT document model). Speaks graphics engine frame vocabulary; zero UI/Leptos dependencies.
   - `website-frontend`: Presentation, navigation, and CAD workspaces (`src/v2/`); consumes engine crates.
   - `website-api`: Axum REST API and SSE realtime hub.
7. **File Size Limits & Test Placement**:
   - Production files must stay **under 500 lines of code**.
   - Test files must stay **under 1000 lines of code**.
   - **No inline test modules**: Unit tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
8. **Present-Tense, Context-Free Code Documentation**:
   Comments and docstrings must describe strictly what the code does *now* and *why* (invariants, mathematical models, hardware/engine constraints). Never document historical transitions (no "rewritten from X", "fixed in Y"). Commit history owns history.
9. **API & Contract Parity**:
   - Backend Rust models (`apps/website/api_v2/src/<domain>/models/`) are the snake_case API source of truth.
   - Contract types are generated from `contracts_v2/definitions/*.json` via `cargo xtask ci schema-codegen`.
   - Frontend DTOs (`apps/website/frontend/src/v2/core/api/dto/`) mirror models with strict R-api golden test parity.

---

## 2. Monorepo Directory Atlas

```text
apps/
├── ticketboard/                         <-- Native egui/eframe desktop viewer for .ai/tickets
├── mod/                                 <-- Enfusion engine mod suite
│   ├── tbd-framework/                   <-- Shipping game mod (TBD_Framework)
│   │   ├── Configs/                     <-- System configs, entity catalogs, voice frequencies
│   │   ├── Data/                        <-- Game mode data, sound presets
│   │   ├── Missions/                    <-- Mission header configs and world definitions
│   │   ├── Prefabs/                     <-- Character, weapon, vehicle, and UI entity prefabs
│   │   ├── Scripts/Game/TBD/            <-- Gameplay scripts (compiled into game runtime)
│   │   │   ├── API/                     <-- Backend REST & event streaming bridge
│   │   │   ├── Core/                    <-- Foundation classes, event dispatcher, logging
│   │   │   ├── Gamemode/                <-- Phase manager, game state lifecycle, safe-start
│   │   │   ├── Session/                 <-- Player state, slot reservation tokens
│   │   │   ├── Systems/                 <-- Radios, gear/loadouts, objectives, markers, play areas
│   │   │   └── UI/                      <-- HUD widgets, menu controllers, briefing/slotting screens
│   │   └── UI/                          <-- UI layout definitions and visual assets
│   │       ├── Textures/                <-- UI icons, discs, custom textures
│   │       └── layouts/                 <-- Enfusion widget layout files (.layout)
│   │           ├── Common/              <-- Reusable design system primitives (buttons, panels, cards)
│   │           ├── Hud/                 <-- Tactical in-game HUD and compass widgets
│   │           └── Session/             <-- Pre-game lobby, slotting screen, briefing dock layouts
│   ├── tbd-export/                      <-- Tooling addon (TBD_Export, depends on framework + emcp)
│   │   ├── Missions/                    <-- Export world mission configurations (Everon export)
│   │   ├── Prefabs/                     <-- Export game mode and road export components
│   │   ├── tools/                       <-- Export helper scripts
│   │   └── Scripts/
│   │       ├── Game/TBD/Export/         <-- Runtime export components (roads, terrain features)
│   │       └── WorkbenchGame/           <-- Workbench export plugins (MapExport, Equipment, Vehicles, Registry)
│   ├── tbd-emcp/                        <-- Enfusion MCP bridge handler scripts (TBD_EMCP)
│   │   └── Scripts/WorkbenchGame/EnfusionMCP/ <-- 19 committed NetAPI automation handlers
│   ├── crf_framework/                   <-- Reference: Upstream Coalition Reforger Framework scripts
│   └── vanilla_reference/               <-- Reference: Extracted vanilla Reforger scripts and API docs
└── website/                             <-- Web platform applications and engines
    ├── api_v2/                          <-- Axum + sqlx REST API and SSE backend (:8080)
    │   └── src/                         <-- Domain-driven backend: core + workers + eight domains
    │       ├── core/                    <-- Composition root, config, database, middleware, observability, realtime hub, auth primitives
    │       ├── background_workers/      <-- Interval tasks the API binary arms at boot
    │       ├── administration/          <-- Member roster, moderation actions, audit log
    │       ├── command_center/          <-- Dashboard, leaderboards, per-player statistics
    │       ├── community_content/       <-- Announcements, wiki, vehicle database, modpacks, uploads
    │       ├── identity_and_access/     <-- Discord OAuth2, session tokens, profile, Arma link handshake
    │       ├── match_telemetry/         <-- Game-server ingest: status heartbeat and match results
    │       ├── missions/                <-- Scenario library, versions, armory, registries, approvals
    │       ├── operations/              <-- Event calendar, ORBAT slotting, service records, fire missions
    │       ├── server_infrastructure/   <-- Dedicated-server registry, live status SSE, RCON console
    │       └── tests/architecture_rules.rs <-- Executable layout rules checked against src/
    ├── frontend/                        <-- Leptos 0.8 CSR single-page app (Trunk/WASM, :3000)
    │   └── src/v2/                      <-- Domain-driven frontend architecture
    │       ├── core/                    <-- Shared foundations across the frontend
    │       │   ├── api/                 <-- HTTP client, DTOs, SSE subscriber, error handling
    │       │   ├── auth/                <-- Session storage, role hierarchy, route guards
    │       │   ├── ui/                  <-- Reusable design system primitives (buttons, inputs)
    │       │   └── utils/               <-- Time formatting, math, and DOM helpers
    │       ├── pages/                   <-- Standard platform navigation & document pages
    │       │   ├── navigation/          <-- Platform frame (topbar, sidebar navigation drawer)
    │       │   ├── account/             <-- User login, OAuth callback, settings
    │       │   ├── command_center/      <-- Dashboard, announcements, live server intel
    │       │   ├── operations/          <-- Event schedule, dossier, slotting, ladders
    │       │   │   ├── schedule/        <-- Calendar schedule of upcoming community events
    │       │   │   ├── event_detail/    <-- Event briefing dossier and slot signups
    │       │   │   ├── orbat_selection/ <-- Dedicated full-screen slotting view
    │       │   │   ├── deployments/     <-- Historical operations archive and attendance
    │       │   │   └── leaderboards/    <-- Community player and team match rankings
    │       │   ├── mission_hub/         <-- Scenario library, overview dossier, create modal
    │       │   │   ├── library/         <-- Filterable community scenario catalog
    │       │   │   ├── overview/        <-- Scenario version history, ORBAT preview, metadata
    │       │   │   └── create_dialog/   <-- Initial scenario creation dialog
    │       │   ├── field_tools/         <-- Interactive tactical utilities
    │       │   │   └── mortar/          <-- Mortar ballistics calculation and firing solutions
    │       │   ├── doctrine_and_info/   <-- Knowledgebase and reference catalogs
    │       │   │   ├── wiki/            <-- Markdown tactical doctrine and rules articles
    │       │   │   ├── vehicles/        <-- Vehicle catalog and asset technical specs
    │       │   │   └── modpacks/        <-- Modpack manifests and delta download information
    │       │   └── administration/      <-- Management and administrative control panels
    │       │       ├── event_manager/   <-- Event scheduling and operations calendar admin
    │       │       ├── server_control/  <-- Dedicated server lifecycle and RCON console
    │       │       ├── personnel/       <-- Member roster, rank, and permission management
    │       │       ├── approvals/       <-- Mission submission review and approval queue
    │       │       ├── content_manager/ <-- CMS for doctrine articles and announcements
    │       │       └── audit_logs/      <-- Audit trail of administrative actions
    │       └── apps/                    <-- Standalone CAD workspaces & interactive tools
    │           ├── editor/              <-- Scenario Creator 2D/3D CAD workspace shell (~55k)
    │           │   ├── ui/              <-- Modular CAD docks, outliner, inspector, toolbelt, modals (<500 LOC)
    │           │   ├── input/           <-- DOM Pointer/Keyboard events -> Map Engine commands
    │           │   ├── bridge/          <-- Canvas mount, DPR scaling, rAF heartbeat connector
    │           │   ├── shell/           <-- Tab locks, autosave persistence, session preferences
    │           │   └── arsenal/         <-- Loadout forms, gear catalog, 3D paper doll UI
    │           ├── planner/             <-- Tactical planning whiteboard and briefing interface
    │           ├── aar/                 <-- Telemetry replay and after-action review player
    │           └── debug/               <-- Diagnostics testbench (occlusion and building viewer)
    ├── map-engine/                      <-- World, spatial computation, formats, and mission domain
    │   └── src/
    │       ├── data/                    <-- Scenario compiler/AST/validation + Yjs CRDT store/rows
    │       ├── editing/                 <-- Headless tool FSMs (Ruler, LOS, Place), commands, undo history
    │       ├── world/                   <-- DEM terrain elevation, building meshes, vegetation, water
    │       ├── spatial/                 <-- 3D BVH spatial indexes, picking, raymarching (terrain/world/interior)
    │       ├── camera/                  <-- Camera projections, pan/zoom, metric <-> MGRS coordinate unproject
    │       ├── streaming/               <-- 512m chunk residency, tile cache, memory budgets
    │       ├── io/                      <-- Binary rkyv map formats, compressed containers, POD serialization
    │       ├── symbology/               <-- NATO MIL-STD-2525 symbology and tactical graphics
    │       ├── frame/                   <-- BUILDS graphics_engine::frame packets from world state
    │       ├── doll/                    <-- 3D character equipment and arsenal preview
    │       └── diagnostics/             <-- Engine benchmarks, probe runners
    └── graphics-engine/                 <-- Pure GPU rendering primitives (knows zero map concepts)
        └── src/
            ├── gpu_context/             <-- GPU adapter, logical device, queue, surface lifecycle
            ├── render_passes/           <-- Vertex/index buffers, draw calls, instanced batching
            ├── frame/                   <-- Uniform uploads, render pass layout, timing
            ├── frame_loop/              <-- Damage-driven rAF render loop and frame orchestration
            ├── pipeline/                <-- Render pipelines, bind group layouts, depth states
            ├── shaders/                 <-- WGSL shader source code and compilation helpers
            └── text/                    <-- MSDF and bitmap glyph atlas texture rendering

tools_v2/                               <-- Every developer tool in the repository; four crates plus one npm package
├── verification-core/                  <-- Fail-closed verdicts, pattern scans, process isolation, repository verification lock
├── ticket-engine/                      <-- Ticket corpus storage, validation, generated views, wave lock, metrics, repository paths
├── developer-tools/                    <-- Heavy async CLI suite, blueprint compiler, map verification
│   ├── src/bin/                        <-- Executables: enf, gate, mcpd, world, map, capture
│   │   ├── enf                         <-- Enfusion pak unpacker and script source extractor
│   │   ├── gate                        <-- Headless CDP Chrome gate test harness
│   │   ├── mcpd                        <-- Enfusion MCP broker daemon
│   │   ├── world                       <-- World object and terrain chunk processing pipeline
│   │   ├── map                         <-- Image optimization and satellite tile pipeline
│   │   └── capture                     <-- Headless map snapshot utility
│   ├── fixtures/dom_oracle/            <-- Frozen editor DOM goldens the route-drift gate compares against
│   └── test_fixtures/blueprint/        <-- Prefab and world-object inputs for the blueprint compiler tests
├── xtask/                              <-- `cargo xtask` command router, repository verifications, platform execution
│   ├── deploy/                         <-- deploy.env.example, Caddyfile.website, systemd/ units and timers
│   ├── dedicated_server_profiles/      <-- Dedicated-server JSON profiles for playtest and world boot
│   └── fixtures/mcp/                   <-- Recorded MCP transcripts `cargo xtask mcp selftest` replays
└── enfusion_mcp_node_package/          <-- Pinned enfusion-mcp npm package (node_modules gitignored)

contracts_v2/                            <-- Every shape that crosses a network, process, or language boundary
├── definitions/                         <-- Authoritative JSON Schemas (missions, arsenal, terrain, voice)
├── rules/                               <-- Prefab classification and mission kit aliases
├── catalogs/                            <-- Live Workbench exports the platform ingests
└── fixtures/                            <-- Golden test data, positive and negative

assets_v2/                               <-- Terrain datasets and map symbology
├── terrains/                            <-- Built-in islands (Everon, Arland), served at /map-assets
├── glyphs/                              <-- World-object glyph atlas and SVG sources
├── scratch/                             <-- Local export intermediates (gitignored)
└── storage_spec/                        <-- Production persistent volume specification

docs/                                    <-- Architecture specs, UI surface specs, and runbooks (authoritative)
documentation_v2/                        <-- Blueprint for a reorganised documentation tree; tools read `docs/`
.ai/tickets/                             <-- Ticket registry TOML files
```

---

## 3. Canonical Commands (`cargo xtask`)

Configuration lives in `apps/website/api_v2/.env` (`APP_ENV=development`, Postgres on port 5434).

```bash
# Database (Postgres :5434)
cargo xtask db up              # Start local Postgres container
cargo xtask db down            # Stop local Postgres container (keeps volume)
cargo xtask db seed            # Apply development SQL seeds
cargo xtask db repair-migration-checksum [--version N]  # Repoint the checksums of comments-only edits to applied migrations

# Development Servers
cargo xtask mk rust-api        # Axum API on :8080 (runs migrations on boot)
cargo xtask mk leptos          # Leptos SPA on :3000 (Trunk release build)

# Quality Gates & Testing
cargo xtask ci ci-local        # Replay full CI check suite locally
cargo xtask mk ci-local-leptos # Frontend checks: fmt, clippy wasm32, test, trunk release build
cargo xtask mk leptos-gates    # Full headless Chrome CDP editor gates (runs gate doctor first)
cargo xtask db test-it         # Rust backend integration tests (requires db up)
cargo xtask mod compile        # Compile check Enfusion mod scripts

# Ticket Registry
cargo xtask ticket check       # Validate ticket registry structure
cargo xtask ticket sync        # Regenerate ticket views and roadmaps

# Deployment (tools_v2/xtask/deploy/deploy.env)
cargo xtask deploy website --dry-run  # Print the plan: asset preflight, rsync excludes, remote steps
cargo xtask deploy website     # Rsync, build the API + SPA on the server, restart the unit
```

### Dev Login (No Discord Required)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=admin|mission_maker|enlisted`. Open in browser or read `access_token` from the 302 `Location` fragment for API testing.
