**Status:** live

# Glossary terms N to Z

The glossary's entries from N to Z, in alphabetical order, each in the format of the
[glossary entry template](/documentation_v2/standards/templates/glossary_entry.md). The
[glossary index](/documentation_v2/glossary/README.md) lists every term and says how a document
links one.

### operations

The domain around [events](/documentation_v2/glossary/a_to_f.md#event): the event calendar, the [ORBAT](#orbat) and slotting, squad
reservations and the waitlist, member search, [service records](#service-record) and leave requests;
the API domain also serves the mortar fire-mission tools and the game-runtime roster and deployments.

In code: `apps/website/api_v2/src/operations/`; `apps/website/frontend/src/v2/pages/operations/`.

See: [Operations domain](/apps/website/api_v2/src/operations/README.md).

### oracle

A source an agent checks a fact against instead of recalling it. The Enfusion script oracle is the
`enf` tool over the vanilla game scripts and the upstream framework: it indexes their symbols,
answers lookups and checks `@idx` citations. Its sources, the gitignored oracle lanes, are linked
into every [slice](#slice) worktree to read and cite, never to copy. The DOM oracle is the frozen
page goldens that `gate v-suite` holds the built app to.

In code: `tools_v2/developer-tools/src/enfusion_tooling/` (the `enf` binary); the lane links in `tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs`; `cargo xtask verify no-crf-leak`; `tools_v2/developer-tools/src/browser_testing/dom_oracle/` and the goldens in `tools_v2/developer-tools/fixtures/dom_oracle/`.

See: [Enfusion script oracle](/tools_v2/developer-tools/src/enfusion_tooling/README.md), [Oracle lanes](/documentation_v2/runbooks/mod_slice_workflow.md#oracle-lanes), [DOM oracle fixtures](/tools_v2/developer-tools/fixtures/dom_oracle/README.md).

### ORBAT

The order of battle: the factions, squads and role [slots](#slot) of one mission within an event. A
mission carries it in its document; each event mission holds it as `orbat_slots` rows.

In code: `OrbatSlot` in `apps/website/api_v2/src/operations/models/event.rs`; `apps/website/api_v2/src/operations/handlers/orbat_view.rs`.

See: [ORBAT selection page](/apps/website/frontend/src/v2/pages/operations/orbat_selection/README.md).

### orchestrator

In the factory, the agent session that plans, dispatches, integrates and verifies a [wave](#wave),
never implementing; in the mod, the round orchestrator `TBD_FrameworkManager`.

In code: `cargo xtask platform wave`; `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`.

See: [Factory waves](/documentation_v2/runbooks/factory_waves/README.md), [command center](/documentation_v2/glossary/a_to_f.md#command-center).

### personnel

The `/admin/personnel` page, titled Personnel Roster: the member roster beside one member's dossier,
where administrators ban and warn members with a reason and run the Discord role resync. A member's
[role](#role) follows their Discord roles, so the dossier explains it and never sets it.

In code: `PersonnelRosterPage` in `apps/website/frontend/src/v2/pages/administration/personnel/`.

See: [Personnel roster page](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md).

### RCON

BattlEye RCon, the remote-console protocol the dedicated server speaks over UDP; the [fleet host
agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) uses it to list players. The platform has no RCON console: broadcasts and
kicks run in the [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime), and Reforger's RCON has no broadcast command.

In code: `apps/fleet_host_agent/src/rcon/`; `FleetAction` in `apps/website/api_v2/src/server_infrastructure/models/fleet_command.rs`.

See: [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command).

### registry

Most often the item registry: one modpack's flat catalog of the engine items the
[arsenal](/documentation_v2/glossary/a_to_f.md#arsenal) offers, with a graph of what fits in or on what, exported from Workbench and
imported into Postgres. Other registries are named in full (ticket, server, fleet scenario).

In code: `RegistryItem` and `RegistryCompatEdge` in `apps/website/api_v2/src/missions/models/registry.rs`; `contracts_v2/catalogs/`.

See: [Contract catalogs](/contracts_v2/catalogs/README.md).

### render engine

The map engine's drawing object, owner of the GPU device, canvas surface, camera and draw batches;
it hands the graphics engine a [frame packet](/documentation_v2/glossary/a_to_f.md#frame-packet) when something changed.

In code: `RenderEngine` in `apps/website/map-engine/src/frame/engine.rs`.

See: [Render engine and frame packet](/apps/website/map-engine/src/frame/README.md).

### role

An account's tier on the permission ladder, lowest first: `guest`, `enlisted`, `leader`,
`mission_maker`, `admin`; a route's access tier is the lowest role it admits. A member's role
follows their Discord roles through the `discord_roles` mappings; the website sets none itself.

In code: `Role` in `apps/website/frontend/src/v2/core/auth/role.rs`; `role_rank` in `apps/website/api_v2/src/core/middleware/mod.rs`.

See: [dev login](/documentation_v2/glossary/a_to_f.md#dev-login), [personnel](#personnel).

### runtime session

One boot of a server's [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) as the
API records it. Starting one takes the server's next generation and supersedes its open session;
heartbeats every 15 s carry a strictly rising sequence; a session silent for 60 s expires and its
server goes offline; ending a session ends the player lives still open in it.

In code: `apps/website/api_v2/src/server_infrastructure/services/runtime_sessions.rs` (table `server_runtime_sessions`); `POST /api/v1/game-runtime/sessions` and its `/end` in `game_runtime_sessions.rs` beside it under `handlers/`; the heartbeat route in `apps/website/api_v2/src/match_telemetry/routes.rs`; `apps/website/api_v2/src/background_workers/runtime_session_expiry.rs`; `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_RuntimeSession.c`.

See: [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential), [server infrastructure](#server-infrastructure).

### safe start

The mod's warm-up stage between the briefing and the live round, in which nobody can be hurt:
damage handling is off on every body, shots and grenades are deleted as they appear and weapon
safety is on. A countdown, 300 s unless the mission or an administrator sets 5 to 3600, runs it to
`LIVE`; the same shield already holds in the lobby and the briefing. The code spells it safestart.

In code: `TBD_SafestartManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/TBD_SafestartManager.c`; `SAFE_START` in `TBD_EGameStage` beside it; the mission's `flow.safeStartSeconds`.

See: [Round stages and safe start](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/README.md), [Safe start HUD](/documentation_v2/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md).

### scenario

A code spelling, never a prose term. Platform code that says scenario means a [mission](/documentation_v2/glossary/g_to_m.md#mission)
(the map engine's mission domain); fleet code means a [mission header](/documentation_v2/glossary/g_to_m.md#mission-header); Enfusion's
own names (`scenarioId`, the `SCR_EScenario*` types) keep it. Prose says mission or mission header.

In code: `apps/website/map-engine/src/data/scenario/`; `apps/website/api_v2/src/server_infrastructure/handlers/fleet_scenarios.rs`.

See: [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario).

### server control

The `/admin/server` page: the configured game servers with their live state and, for the selected
server, its [fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command), [mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment) and
[machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential), with the [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario) registry.

In code: `ServerControlPage` in `apps/website/frontend/src/v2/pages/administration/server_control/`.

See: [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md).

### server infrastructure

The API domain of the game servers: the server registry, each server's status and live [SSE](#sse)
feed, [machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential), the [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) ledger and its
executor routes, runtime sessions and the [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario) registry.

In code: `apps/website/api_v2/src/server_infrastructure/`.

See: [RCON](#rcon), [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md).

### service record

A member's own record at `/deployments` (My Deployments): matches played, upcoming deployments, past
matches and leave requests; no combat figures, though the API sends kills, deaths and K/D.

In code: `apps/website/api_v2/src/operations/handlers/member_service_record.rs`; `DeploymentsPage` in `apps/website/frontend/src/v2/pages/operations/deployments/`.

See: [Deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md).

### slice

One ticket's unit of work in the [factory](/documentation_v2/glossary/a_to_f.md#factory) or the mod
program, built by one agent in its own git worktree, `.ai/artifacts/worktrees/<slice>/` on the
branch `slice/<slice>` made from `main`. The agent runs the slice
[gate](/documentation_v2/glossary/g_to_m.md#gate) and reports; the orchestrator lands it. A
sub-slice, with two dots in its ID, shares its parent's worktree.

In code: `cargo xtask platform slice-worktree` in `tools_v2/xtask/src/commands/platform/slice_worktree/`, which also links the [oracle](#oracle) lanes; `cargo xtask platform slice-run`; `cargo xtask platform wave gate`.

See: [wave](#wave), [Factory waves](/documentation_v2/runbooks/factory_waves/README.md).

### slot

One fillable position in an [ORBAT](#orbat): a faction, squad, callsign, role and loadout that one
member occupies, and in the game a spawn position. Slotting fills them: members reserve a slot or
join the waitlist, squad managers assign seats, and players claim their slot in the game's lobby.

In code: `OrbatSlot` in `apps/website/api_v2/src/operations/models/event.rs`; `slot_registration.rs` and `slot_assignment.rs` in `apps/website/api_v2/src/operations/handlers/`.

See: [event](/documentation_v2/glossary/a_to_f.md#event), [arsenal](/documentation_v2/glossary/a_to_f.md#arsenal).

### SSE

Server-Sent Events: the one-way HTTP streams on which the API pushes live updates, such as a
server's status feed and the audit log feed. An SSE event is one message, never an [event](/documentation_v2/glossary/a_to_f.md#event).

In code: `Hub` in `apps/website/api_v2/src/core/realtime_hub/mod.rs` (the status feed); the audit feed's `LISTEN audit_log` in `apps/website/api_v2/src/administration/services/audit_notifier.rs`; the client in `apps/website/frontend/src/v2/core/api/sse.rs`.

See: [audit logs](/documentation_v2/glossary/a_to_f.md#audit-logs), [server infrastructure](#server-infrastructure).

### Stitch visual reference

A design-phase picture of a page or screen made with Stitch, an AI interface design tool, kept as a
set in the `visual_references/` folder of the feature it depicts. A set is named `<subject>_<kind>`
(blueprint, mockup or render) and holds the Stitch export as `<set>.html`, its screenshot as
`<set>.png` and, when the export carries tokens, `design_tokens.md`. The built interface wins; the
[feature doc](/documentation_v2/glossary/a_to_f.md#feature-doc)'s Design section says how it differs.

In code: none; the built styles a set is compared with are `apps/website/frontend/style/aegis.css` on the website and `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UITheme.c` in the mod.

See: [Design system](/documentation_v2/design_system/README.md), [Stitch token exports](/documentation_v2/design_system/token_exports/README.md).

### ticket

One unit of planned work, stored as `.ai/tickets/T-<id>.toml` for parents and dotted children
alike, with a status of `idea`, `queued`, `ready`, `running`, `review`, `shipped`, `deferred` or
`cancelled`. Every ticket operation is a `cargo xtask ticket` command.

In code: `load_registry` in `tools_v2/ticket-engine/src/registry/mod.rs`; `StatusName` in `tools_v2/ticket-engine/src/model/status.rs`.

See: [wave](#wave), [ticketboard](#ticketboard), [Ticket registry](/.ai/tickets/README.md).

### ticketboard

The native desktop viewer of the ticket registry, built on egui: parent and child tickets, wave
lanes, the program tree, run receipts and estimates, with specs and documents beside them. Every
change it makes runs a `cargo xtask ticket` command.

In code: `apps/ticketboard/`, which reads the registry through `tools_v2/ticket-engine/`.

See: [Ticketboard](/apps/ticketboard/README.md).

### wave

A numbered group of [tickets](#ticket) in `.ai/tickets/wave.lock`, which `cargo xtask wave repack`
alone writes. Tickets run in parallel only when the files they own do not overlap;
`cargo xtask platform wave` and `cargo xtask mod wave` drive a wave for the platform and the mod.

In code: `tools_v2/ticket-engine/src/wave_lock/`; `tools_v2/xtask/src/commands/wave/cli.rs`.

See: [Factory waves](/documentation_v2/runbooks/factory_waves/README.md).

### Workbench

Enfusion's editor application, where worlds, prefabs and mission headers are edited and plugins
run: the `tbd-export` plugins export the terrain, object and registry data the platform ingests,
and the `tbd-emcp` handlers let the Enfusion MCP tools drive Workbench from outside.

In code: `apps/mod/tbd-export/Scripts/WorkbenchGame/`; `tools_v2/developer-tools/src/bin/mcpd.rs`.

See: [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion), [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md).
