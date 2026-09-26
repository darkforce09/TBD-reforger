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
   - `website-graphics-engine`: Pure GPU rendering primitives (pipelines, shaders, draw batching). Knows **zero** map concepts.
   - `website-map-engine`: Map graphics, spatial computation, terrain formats, asset streaming, camera math, and the mission domain (compilation, validation, Yjs CRDT document model). Speaks graphics engine frame vocabulary; zero UI/Leptos dependencies.
   - `website-frontend`: Presentation, navigation, and CAD workspaces (`src/v2/`); consumes engine crates.
   - `website-api`: Axum REST API and SSE realtime hub.
7. **File Size Limits & Test Placement (Hard Ceilings — Zero Exemptions)**:
   - Production files must stay **at or under 500 lines**.
   - Test files (inside a `tests/` folder or named `*_tests.rs`) must stay **at or under 1000 lines**.
   - The ceilings apply in every language; `cargo xtask verify file-length` enforces them on the Rust source trees by raw line count.
   - **Zero Exemptions / No Allowlist**: There is NO allowlist file and NO exemption mechanism. Never create an allowlist (`.coding-standards-allowlist.yaml` or any other), use allowlist comments, or bypass these limits. If a file approaches or exceeds 500 lines, you MUST decompose it by responsibility into cohesive submodules.
   - **No inline test modules**: Unit tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
8. **Present-Tense, Context-Free Code Documentation**:
   Comments and docstrings must describe strictly what the code does *now* and *why* (invariants, mathematical models, hardware/engine constraints). Never document historical transitions (no "rewritten from X", "fixed in Y"). Commit history owns history.
9. **API & Contract Parity**:
   - Backend Rust models (`apps/website/api_v2/src/<domain>/models/`) are the snake_case API source of truth.
   - Contract types are generated from `contracts_v2/definitions/*.json` via `cargo xtask ci schema-codegen`.
   - Frontend DTOs (`apps/website/frontend/src/v2/core/api/dto/`) mirror models with strict R-api golden test parity.
10. **Documentation Ships With the Code**:
    - Documentation lands in the same commit as the code it describes, whichever agent writes that code: the comments of the code it alters, the README.md of every folder whose contents, surface, commands or boundaries change, and the feature docs whose behaviour changes.
    - [documentation_v2/README.md](/documentation_v2/README.md) is the documentation entry (map and authority ladder); [documentation_v2/standards/](/documentation_v2/standards/README.md) holds the documentation, README and coding standards and the templates.
    - Terms follow the [glossary](/documentation_v2/glossary/README.md): the editor is the **Mission Creator**; the authored document is a **mission** (never "scenario" in prose; code identifiers stay quoted as spelled); Enfusion's world plus game-mode config is the **mission header**; an **event** is a scheduled session record; **operations** is its domain.
    - Before committing, run the three documentation gates over what changed (§3).

---

## 2. Monorepo Directory Atlas

```text
apps/
├── fleet_host_agent/                    <-- Agent on each game host: claims fleet commands from the API, runs process control, RCON reads, mission header switches
├── ticketboard/                         <-- Native egui/eframe desktop viewer for .ai/tickets
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
│   │       ├── Game/TBD/Export/         <-- Runtime road-network export component
│   │       └── WorkbenchGame/           <-- Workbench export plugins (MapExport, EquipmentExport, EquipmentVehicleExport, VehicleExport, registry)
│   ├── tbd-emcp/                        <-- Enfusion MCP bridge handler scripts (TBD_EMCP)
│   │   └── Scripts/WorkbenchGame/EnfusionMCP/ <-- 19 committed NetAPI automation handlers
│   ├── crf_framework/                   <-- Reference: upstream Coalition Reforger Framework scripts (gitignored)
│   └── vanilla_reference/               <-- Reference: extracted vanilla Reforger scripts and API docs (gitignored)
└── website/                             <-- Web platform applications and engines
    ├── api_v2/                          <-- Axum + sqlx REST API and SSE backend (:8080)
    │   └── src/                         <-- Domain-driven backend: core + workers + eight domains
    │       ├── core/                    <-- Configuration, database, state, errors, router, middleware, observability, realtime hub, auth primitives
    │       ├── background_workers/      <-- Interval tasks the API binary arms at boot
    │       ├── bin/                     <-- The `api` server and the `import-registry` tool
    │       ├── administration/          <-- Member roster, moderation actions, Discord role resync, audit log
    │       ├── command_center/          <-- Dashboard, leaderboards, per-player statistics
    │       ├── community_content/       <-- Announcements, wiki, vehicle database, modpacks, uploads
    │       ├── identity_and_access/     <-- Discord OAuth2, session tokens, profile, Arma link handshake
    │       ├── match_telemetry/         <-- Game-runtime heartbeats and finished match results
    │       ├── missions/                <-- Missions, versions, artifacts, reviews, deployments, armory, registries
    │       ├── operations/              <-- Events, ORBAT slotting, reservations, service records, fire missions
    │       ├── server_infrastructure/   <-- Game-server registry, live status SSE, machine credentials, fleet commands, runtime sessions
    │       └── tests/architecture_rules.rs <-- Executable layout rules checked against src/
    ├── frontend/                        <-- Leptos 0.8 CSR single-page app (Trunk/WASM, :3000)
    │   └── src/v2/                      <-- Domain-driven frontend architecture
    │       ├── core/                    <-- Shared foundations across the frontend
    │       │   ├── api/                 <-- HTTP client, DTOs, endpoints, SSE subscriber
    │       │   ├── auth/                <-- Session storage, role hierarchy, route guards
    │       │   ├── ui/                  <-- Reusable design system primitives (dialogs, sheets, selects, toasts)
    │       │   └── utils/               <-- Time formatting, clipboard, sanitising helpers
    │       ├── pages/                   <-- Standard platform navigation & document pages
    │       │   ├── navigation/          <-- Platform frame (top nav, sidebar, not-found page)
    │       │   ├── account/             <-- User login, OAuth callback, settings
    │       │   ├── command_center/      <-- Dashboard, announcements, live server intel
    │       │   ├── operations/          <-- Event schedule, event detail, slotting, deployments, leaderboards
    │       │   │   ├── schedule/        <-- Upcoming events with the selected event's hub
    │       │   │   ├── event_detail/    <-- Event briefing dossier and slot signups
    │       │   │   ├── orbat_selection/ <-- Dedicated full-screen slotting view
    │       │   │   ├── deployments/     <-- "My Deployments": the viewer's service record and upcoming slots
    │       │   │   └── leaderboards/    <-- Community player rankings with a slide-over dossier
    │       │   ├── mission_hub/         <-- Mission library, overview dossier, create dialog, review
    │       │   │   ├── library/         <-- Filterable community mission catalog
    │       │   │   ├── overview/        <-- Mission dossier: briefing, details, armory, review record
    │       │   │   ├── create_dialog/   <-- "New Mission" dialog that opens the Mission Creator
    │       │   │   ├── mission_review/  <-- Shared review record: history, thread, artifact provenance, submit control
    │       │   │   └── review_workspace/ <-- Mission Creator opened read-only on a submitted version
    │       │   ├── field_tools/         <-- Interactive tactical utilities
    │       │   │   └── mortar/          <-- Mortar ballistics calculation and firing solutions
    │       │   ├── doctrine_and_info/   <-- Knowledgebase and reference catalogs
    │       │   │   ├── wiki/            <-- Markdown tactical doctrine and rules articles
    │       │   │   ├── vehicles/        <-- Vehicle identification index and dossiers
    │       │   │   └── modpacks/        <-- Modpack manifests and Workshop collection links
    │       │   └── administration/      <-- Management and administrative control panels
    │       │       ├── event_manager/   <-- Event scheduling and operations calendar admin
    │       │       ├── server_control/  <-- Game-server state, fleet commands, mission deployment, host credentials
    │       │       ├── personnel/       <-- Member roster, rank, and permission management
    │       │       ├── approvals/       <-- Mission submission review and approval queue
    │       │       ├── content_manager/ <-- Announcement authoring ("Comms Broadcaster")
    │       │       └── audit_logs/      <-- Audit trail of administrative actions
    │       └── apps/                    <-- Standalone CAD workspaces & interactive tools
    │           ├── editor/              <-- Mission Creator: top-down 2D CAD workspace (3D only in the Arsenal paper doll)
    │           │   ├── mission_editor/  <-- Route component parts: canvas mount, page effects, registry loading, transforms
    │           │   ├── ui/              <-- CAD docks, outliner, inspectors, Arsenal tab, modals
    │           │   ├── input/           <-- DOM pointer/keyboard events -> map engine commands and tools
    │           │   ├── bridge/          <-- Engine seam: boot, viewport and frame timing, hosted mission document, overlays
    │           │   ├── shell/           <-- Per-tab session: IndexedDB drafts, hydrate, cross-tab lock, review mode, layout prefs
    │           │   └── arsenal/         <-- Loadout domain, gear catalog trees, 3D paper doll
    │           ├── planner/             <-- Reserved for the mission planner whiteboard (README only, no code)
    │           ├── aar/                 <-- Reserved for the after-action review replay (README only, no code)
    │           └── debug/               <-- URL-only benches: building viewer, building interior, world line of sight
    ├── map-engine/                      <-- World, spatial computation, formats, and mission domain
    │   └── src/
    │       ├── data/                    <-- Mission domain (`scenario` module: shapes, compiler, checks) + Yjs CRDT store
    │       ├── editing/                 <-- Live mission document, undo history, headless map tools (select, ruler, LOS, viewshed)
    │       ├── world/                   <-- Terrain (DEM, relief, roads, satellite, water), buildings, vegetation, labels
    │       ├── spatial/                 <-- BVHs, point indexes, picking, line of sight (terrain, world, building interiors)
    │       ├── camera/                  <-- Orthographic map camera, doll orbit camera, grid-reference labels
    │       ├── streaming/               <-- Served map data: fetch, world chunk residency, draw buffers, memory budget
    │       ├── io/                      <-- On-disk formats: rkyv archives, containers, density grids, POD layouts
    │       ├── overlay/                 <-- Map overlay lanes and draw order
    │       │   └── symbology/           <-- Bespoke unit-role and vehicle glyphs, side tints, labels, marker glyphs
    │       ├── frame/                   <-- Render engine: builds graphics_engine::frame packets, upload belts
    │       ├── doll/                    <-- Arsenal 3D mannequin preview: scene, picking, renderer
    │       ├── shaders/                 <-- The doll renderer's WGSL program
    │       └── diagnostics/             <-- Readback checks, benchmarks, timing, probes
    └── graphics-engine/                 <-- Pure GPU rendering primitives (knows zero map concepts)
        └── src/
            ├── device/                  <-- Pooled per-lane GPU buffers, readback fences
            ├── draw/                    <-- Triangulation, instance layouts, uploads, culling, the frame encoder
            ├── frame/                   <-- Frame vocabulary: packet, batches, ids, camera, present, damage, atlases
            ├── layout/                  <-- Every byte layout shared with callers (re-exports)
            ├── loop/                    <-- Shared requestAnimationFrame pump and its FrameTarget trait
            ├── pipeline/                <-- Render pipeline constructors
            ├── shaders/                 <-- WGSL: every vertex, fragment and compute entry point
            └── text/                    <-- Bitmap font, ASCII glyph atlas bake, glyph layout, sprite packing

tools_v2/                               <-- Every developer tool in the repository; four crates plus one npm package
├── verification-core/                  <-- Fail-closed verdicts, pattern scans, process isolation, repository verification lock
├── ticket-engine/                      <-- Ticket storage, validation, queue and roadmap sync, wave lock, metrics
├── developer-tools/                    <-- Heavy async CLI suite, blueprint compiler, map verification
│   ├── src/bin/                        <-- Executables: enf, gate, mcpd, world, map, capture
│   │   ├── enf                         <-- Symbol indexes, lookups and checks over Enfusion scripts
│   │   ├── gate                        <-- Headless CDP Chrome gates of the single-page app
│   │   ├── mcpd                        <-- Enfusion MCP broker daemon
│   │   ├── world                       <-- World-export pipeline and its verification gates
│   │   ├── map                         <-- Satellite, cartographic, label, water and glyph map assets
│   │   └── capture                     <-- Mission Creator screenshots, zoom sweeps, crops
│   ├── fixtures/dom_oracle/            <-- DOM goldens, screenshots and route inventories the browser gates compare against
│   └── test_fixtures/blueprint/        <-- Prefab and world-object inputs for the blueprint compiler tests
├── xtask/                              <-- `cargo xtask` command router, repository verifications, platform execution
│   ├── deploy/                         <-- deploy.env.example, Caddyfile.website, systemd/ units and timers
│   ├── dedicated_server_profiles/      <-- Dedicated-server profile the local mod servers start from
│   └── fixtures/mcp/                   <-- Recorded MCP transcripts `cargo xtask mcp selftest` replays
└── enfusion_mcp_node_package/          <-- Pinned enfusion-mcp npm package (node_modules gitignored)

contracts_v2/                            <-- Every shape that crosses a network, process, or language boundary
├── definitions/                         <-- Authoritative JSON Schemas (missions, events, registry, loadouts, map objects, terrain, fleet)
├── rules/                               <-- Prefab classification and mission kit aliases
├── catalogs/                            <-- Live Workbench exports the platform ingests
└── fixtures/                            <-- Golden test data, positive and negative

assets_v2/                               <-- Terrain datasets and the world-object glyph set
├── terrains/                            <-- Built-in islands (Everon, Arland) and the terrain registry, served at /map-assets
├── glyphs/                              <-- World-object glyph atlas and SVG sources
├── scratch/                             <-- Local export intermediates (gitignored)
└── storage_spec/                        <-- Production persistent volume specification (not yet built)

documentation_v2/                        <-- All documentation; entry, map and authority ladder: README.md
├── website/ mod/ tools_v2/ contracts_v2/ assets_v2/ fleet_host_agent/ ticketboard/
│                                        <-- Feature docs at the code's path minus apps/, src/, src/v2/, Scripts/Game/TBD/
├── runbooks/                            <-- Procedures: local development, deployment, gates, playtests
├── standards/                           <-- Documentation, README and coding standards; document templates
├── glossary/                            <-- Project terms, one file per letter range
├── design_system/                       <-- Design tokens, map symbology, interaction patterns
├── known_bugs/                          <-- Live known-bug registry
├── tickets/                             <-- Ticket specs and plans (flat; frozen once the ticket closes)
├── archive/                             <-- Frozen history, one folder per topic
└── product_roadmap.md                   <-- Planned product items and open product questions
.ai/tickets/                             <-- Ticket registry: one TOML per ticket, queue.json, ticket templates
```

---

## 3. Canonical Commands (`cargo xtask`)

Configuration lives in `apps/website/api_v2/.env`, copied from `apps/website/api_v2/.env.example` (`APP_ENV=development`, Postgres on port 5434). Step-by-step setup: [local development](/documentation_v2/runbooks/local_development.md).

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

# Documentation Gates (add --path <folder> to narrow)
cargo xtask ci verify-documentation    # All three over the committed tree (a ci-local step; ci.yml language-gates runs them)
cargo xtask verify readme-coverage     # Every README.md Contents block matches its folder's tracked children
cargo xtask verify link-check          # Links, anchors, backticked paths and cited commands resolve
cargo xtask verify markdown-placement  # No Markdown but README.md in code trees; live docs at or under 500 lines

# Ticket Registry
cargo xtask ticket check       # Validate ticket registry structure
cargo xtask ticket next        # Show the active slice and the next five ready or queued tickets
cargo xtask ticket sync        # Regenerate queue.json and the roadmap next-work block (the gap-analysis ticket column is kept by hand)

# Deployment (tools_v2/xtask/deploy/deploy.env)
cargo xtask deploy website --dry-run  # Print the plan: asset preflight, rsync excludes, remote steps
cargo xtask deploy website     # Rsync, build the API + SPA on the server, restart the unit
```

### Dev Login (No Discord Required)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=guest|enlisted|leader|mission_maker|admin` (any other or missing role signs in as `admin`). Open in browser or read `access_token` from the 302 `Location` fragment (`/auth/callback#…`) for API testing.
