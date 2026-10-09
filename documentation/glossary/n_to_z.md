**Status:** live

# Glossary terms N to Z

The glossary's entries from N to Z, in alphabetical order, each in the format of the
[glossary entry template](/documentation/standards/templates/glossary_entry.md). The
[glossary index](/documentation/glossary/README.md) lists every term and says how a document
links one.

### operational receipt

The evidence an operational check (`staging_fleet`, `staging_discord`, `staging_load`) leaves when
the [staging harness](#staging-harness) records a run: `<check>.log`, `<check>.fixture.json` and
`<check>.json` under `target/staging/receipts/`, holding the environment, every observation with the
SHA-256 of its raw artifact, the measurements, and one case line per declared case. It passes only
when every declared case is ok and the measurements meet the check's acceptance thresholds; a
partial run still writes a failing receipt that names its missing dependencies.

In code: `tools/commands/staging_procedures/src/procedure_receipts/`.

See: [Staging receipts design](/documentation/crates/api/api_server/verification_evidence/staging.md), [Staging procedure receipts](/tools/commands/staging_procedures/src/procedure_receipts/README.md).

### operations

The domain around [events](/documentation/glossary/a_to_f.md#event): the event calendar, the [ORBAT](#orbat) and slotting, squad
reservations and the waitlist, member search, [service records](#service-record) and leave requests;
the API domain also serves the ballistics catalogs, the mortar fire missions and the game-runtime
roster and deployments.

In code: `crates/api/api_operations/src/`; `crates/frontend/pages/operations_pages/src/`.

See: [Operations domain](/crates/api/api_operations/src/README.md).

### oracle

A source an agent checks a fact against instead of recalling it. The Enfusion script oracle is the
`enf` tool over the vanilla game scripts and the upstream framework: it indexes their symbols,
answers lookups and checks `@idx` citations. Its sources, the gitignored oracle lanes, are linked
into every [slice](#slice) worktree to read and cite, never to copy. The ballistics oracle is the tbd-export
Workbench plugin and play-mode component that record the engine's own shell flights, against
which a ballistics catalog is calibrated.

In code: `tools/enfusion/enfusion_script_index/src/` (the `enf` binary); the lane links in `tools/commands/platform_execution/src/slice_worktree/git_plain.rs`; `cargo xtask verify no-crf-leak`; `mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/` and `mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/`.

See: [Enfusion script oracle](/tools/enfusion/enfusion_script_index/src/README.md), [Oracle lanes](/documentation/runbooks/mod_slice_workflow.md#oracle-lanes), [Ballistics oracle run](/documentation/runbooks/ballistics_oracle_run.md).

### ORBAT

The order of battle: the factions, squads and role [slots](#slot) of one mission within an event. A
mission carries it in its document; each event mission holds it as `orbat_slots` rows.

In code: `OrbatSlot` in `crates/api/api_operations/src/models/event.rs`; `crates/api/api_operations/src/handlers/orbat_view.rs`.

See: [ORBAT selection page](/crates/frontend/pages/operations_pages/src/orbat_selection/README.md).

### orchestrator

The agent session that plans, launches, reviews, integrates and verifies the work of sub-agents,
never implementing beyond mechanical fixes: in a sub-agent program and in the factory's
[wave](#wave) alike. In the mod, the round orchestrator `TBD_FrameworkManager`.

In code: `cargo xtask platform wave`; `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`.

See: [Sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md), [Factory waves](/documentation/runbooks/factory_waves/README.md), [command center](/documentation/glossary/a_to_f.md#command-center).

### personnel

The `/admin/personnel` page, titled Personnel Roster: the paged member roster beside one member's
dossier, where administrators ban and warn members with a reason and run the Discord role resync. A
member's [role](#role) follows their Discord roles, so the dossier explains it and never sets it.

In code: `PersonnelRosterPage` in `crates/frontend/pages/administration_pages/src/personnel/`.

See: [Personnel roster page](/documentation/crates/frontend/pages/administration_pages/personnel/personnel_roster_page.md).

### perturbation proof

Evidence that a check can fail: a deliberate defect is planted in the code or data a new test or
gate guards, the check is run and its red cases recorded, and the file is restored and proven
byte-equal to its saved copy by `sha256sum`. Every new check in a sub-agent program carries one.

In code: none; each proof is a row of the program's execution record, with its logs.

See: [orchestrator](#orchestrator), [Sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md).

### probable error

The distance within which half of a gun's impacts fall along one axis, 0.6745 times the standard
deviation; the mortar calculator reports it along range and along deflection with the 50 % impact
ellipse. The platform derives it from the game's dispersion parameters, a documented
interpretation that no engine call verifies, not from measured impacts.

In code: `charge_dispersion` in `crates/ballistics/ballistics_solver/src/dispersion.rs`; the dispersion card in `crates/frontend/pages/field_tools_pages/src/mortar/solution/dispersion_card.rs`.

See: [charge ring](/documentation/glossary/a_to_f.md#charge-ring), [Game ballistics engine](/documentation/crates/ballistics/game_ballistics_engine.md).

### RCON

BattlEye RCon, the remote-console protocol the dedicated server speaks over UDP; the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) uses it to list players and to send an administrator's
[console command](/documentation/glossary/a_to_f.md#console-command), one line transmitted once. Broadcasts and kicks run in the
[game runtime](/documentation/glossary/g_to_m.md#game-runtime), and Reforger's RCON has no broadcast command.

In code: `crates/fleet/game_server_host_agent/src/rcon/`; `FleetAction` in `crates/contracts/fleet_wire_contract/src/fleet_action.rs`.

See: [fleet command](/documentation/glossary/a_to_f.md#fleet-command), [console command](/documentation/glossary/a_to_f.md#console-command).

### registry

Most often the item registry: one modpack's flat catalog of the engine items the
[arsenal](/documentation/glossary/a_to_f.md#arsenal) offers, with a graph of what fits in or on what, exported from Workbench and
imported into Postgres. Other registries are named in full (ticket, server, fleet scenario).

In code: `RegistryItem` and `RegistryCompatEdge` in `crates/api/api_missions/src/models/registry.rs`; `contracts/catalogs/`.

See: [Contract catalogs](/contracts/catalogs/README.md).

### render engine

The map renderer's drawing object, owner of the GPU device, canvas surface, camera and draw batches;
it hands `gpu_frame`'s encoder a [frame packet](/documentation/glossary/a_to_f.md#frame-packet) when something changed.

In code: `RenderEngine` in `crates/map_rendering/map_renderer/src/engine.rs`.

See: [Render engine and frame packet](/crates/map_rendering/map_renderer/src/README.md).

### role

An account's tier on the permission ladder, lowest first: `guest`, `enlisted`, `leader`,
`mission_maker`, `admin`; a route's access tier is the lowest role it admits. A member's role
follows their Discord roles through the `discord_roles` mappings; the website sets none itself.

In code: `Role` in `crates/frontend/foundation/frontend_api_dtos/src/role.rs`; `role_rank` in `crates/api/api_http_layer/src/middleware/mod.rs`.

See: [dev login](/documentation/glossary/a_to_f.md#dev-login), [personnel](#personnel).

### runtime session

One boot of a server's [game runtime](/documentation/glossary/g_to_m.md#game-runtime) as the
API records it. Starting one takes the server's next generation and supersedes its open session;
heartbeats every 15 s carry a strictly rising sequence; a session silent for 60 s expires and its
server goes offline; ending a session ends the player lives still open in it.

In code: `crates/api/api_server_infrastructure/src/services/runtime_sessions.rs` (table `server_runtime_sessions`); `POST /api/v1/game-runtime/sessions` and its `/end` in `game_runtime_sessions.rs` beside it under `handlers/`; the heartbeat route in `crates/api/api_match_telemetry/src/routes.rs`; `crates/api/api_background_workers/src/runtime_session_expiry.rs`; `mod/tbd-framework/Scripts/Game/TBD/API/RuntimeSession/TBD_RuntimeSession.c`.

See: [machine credential](/documentation/glossary/g_to_m.md#machine-credential), [server infrastructure](#server-infrastructure).

### safe start

The mod's warm-up stage between the briefing and the live round, in which nobody can be hurt:
damage handling is off on every body, shots and grenades are deleted as they appear and weapon
safety is on. A countdown, 300 s unless the mission or an administrator sets 5 to 3600, runs it to
`LIVE`; the same shield already holds in the lobby and the briefing. The code spells it safestart.

In code: `TBD_SafestartManager` in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/Safestart/TBD_SafestartManager.c`; `SAFE_START` in `TBD_EGameStage` beside it; the mission's `flow.safeStartSeconds`.

See: [Round stages and safe start](/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/README.md), [Safe start HUD](/documentation/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md).

### scenario

A code spelling, never a prose term. Platform code that says scenario means a [mission](/documentation/glossary/g_to_m.md#mission);
fleet code means a [mission header](/documentation/glossary/g_to_m.md#mission-header); Enfusion's
own names (`scenarioId`, the `SCR_EScenario*` types) keep it. Prose says mission or mission header.

In code: `crates/api/api_server_infrastructure/src/handlers/fleet_scenarios.rs`.

See: [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario).

### server control

The `/admin/server` page: the configured game servers with their live state, the server registry
where administrators register, edit, deactivate and reactivate a server, and, for the selected
server, its [fleet commands](/documentation/glossary/a_to_f.md#fleet-command), [mission deployments](/documentation/glossary/g_to_m.md#mission-deployment) and
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential), with the [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) registry.

In code: `ServerControlPage` in `crates/frontend/pages/administration_pages/src/server_control/`.

See: [Server control page](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md).

### server infrastructure

The API domain of the game servers: the server registry, each server's status and live [SSE](#sse)
feed, [machine credentials](/documentation/glossary/g_to_m.md#machine-credential), the [fleet command](/documentation/glossary/a_to_f.md#fleet-command) ledger and its
executor routes, runtime sessions and the [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) registry.

In code: `crates/api/api_server_infrastructure/src/`.

See: [RCON](#rcon), [Server infrastructure domain](/crates/api/api_server_infrastructure/src/README.md).

### service record

A member's own record at `/deployments` (My Deployments): matches played, upcoming deployments, past
matches and leave requests; no combat figures, though the API sends kills, deaths and K/D.

In code: `crates/api/api_operations/src/handlers/member_service_record.rs`; `DeploymentsPage` in `crates/frontend/pages/operations_pages/src/deployments/`.

See: [Deployments page](/documentation/crates/frontend/pages/operations_pages/deployments/deployments_page.md).

### service worker pack

The files the mortar calculator stores for offline use on its first visit: the app shell, every
published ballistics catalog version, the Everon manifest, elevation, imagery and map tiles z0–6,
about 248 MB. The Rust service worker answers from it with no connection; the page calls it the
offline pack and shows its state in `data-offline-state`.

In code: `crates/frontend/shell/offline_service_worker/` (the worker); `offline_pack` and `offline_manifest` in `crates/frontend/foundation/frontend_offline/src/`; `cargo xtask map tile-index` writes the tile list the pack reads.

See: [Offline mortar page](/documentation/runbooks/offline_mortar_page.md), [Mortar calculator page](/documentation/crates/frontend/pages/field_tools_pages/mortar/mortar_calculator_page.md).

### slice

One ticket's unit of work in the [factory](/documentation/glossary/a_to_f.md#factory) or the mod
program, built by one agent in its own git worktree, `.ai/artifacts/worktrees/<slice>/` on the
branch `slice/<slice>` made from `main`. The agent runs the slice
[gate](/documentation/glossary/g_to_m.md#gate) and reports; the orchestrator lands it. A
sub-slice, with two dots in its ID, shares its parent's worktree.

In code: `cargo xtask platform slice-worktree` in `tools/commands/platform_execution/src/slice_worktree/`, which also links the [oracle](#oracle) lanes; `cargo xtask platform slice-run`; `cargo xtask platform wave gate`.

See: [wave](#wave), [Factory waves](/documentation/runbooks/factory_waves/README.md).

### slot

One fillable position in an [ORBAT](#orbat): a faction, squad, callsign, role and loadout that one
member occupies, and in the game a spawn position. Slotting fills them: members reserve a slot or
join the waitlist, squad managers assign seats, and players claim their slot in the game's lobby.

In code: `OrbatSlot` in `crates/api/api_operations/src/models/event.rs`; `slot_registration.rs` and `slot_assignment.rs` in `crates/api/api_operations/src/handlers/`.

See: [event](/documentation/glossary/a_to_f.md#event), [arsenal](/documentation/glossary/a_to_f.md#arsenal).

### SSE

Server-Sent Events: the one-way HTTP streams on which the API pushes live updates, such as a
server's status feed and the audit log feed. An SSE event is one message, never an [event](/documentation/glossary/a_to_f.md#event).

In code: `Hub` in `crates/api/api_http_layer/src/realtime_hub/mod.rs` (the status feed); the audit feed's `LISTEN audit_log` in `crates/api/api_administration/src/services/audit_notifier.rs`; the clients in `crates/frontend/foundation/frontend_transport/src/sse.rs` (the status feed) and `crates/frontend/foundation/frontend_transport/src/audit_stream.rs` (the audit feed, resumed from the last event id it received).

See: [audit logs](/documentation/glossary/a_to_f.md#audit-logs), [server infrastructure](#server-infrastructure).

### staging harness

`cargo xtask staging`, the tool that runs the three operational checks against the staging host and
records their [operational receipts](#operational-receipt). It checks the environment, performs the confirmed remote actions (backup, game server update, fleet provisioning,
credential rotation, load seeding), and for each procedure step prints `AWAIT <step>: <instruction>`
and polls its read-only observers (host shell, database, unit journal, console log, metrics, Discord
member reads, saved Chrome page reads) until the effect shows or the deadline passes, without ever
reading stdin.

In code: `tools/commands/staging_procedures/src/` with `procedure_runner/` and the `fleet_procedure/`, `discord_procedure/` and `load_procedure/` step tables; the host tool `staging-fixtures` in `tools/staging/staging_fixtures/src/`.

See: [fleet instance](/documentation/glossary/a_to_f.md#fleet-instance), [acknowledgement-dropping relay](/documentation/glossary/a_to_f.md#acknowledgement-dropping-relay), [load workload](/documentation/glossary/g_to_m.md#load-workload), [Staging harness](/tools/commands/staging_procedures/src/README.md).

### Stitch visual reference

A design-phase picture of a page or screen made with Stitch, an AI interface design tool, kept as a
set in the `visual_references/` folder of the feature it depicts. A set is named `<subject>_<kind>`
(blueprint, mockup or render) and holds the Stitch export as `<set>.html`, its screenshot as
`<set>.png` and, when the export carries tokens, `design_tokens.md`. The built interface wins; the
[feature doc](/documentation/glossary/a_to_f.md#feature-doc)'s Design section says how it differs.

In code: none; the built styles a set is compared with are `crates/frontend/shell/frontend_application/style/aegis.css` on the website and `mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/TBD_UITheme.c` in the mod.

See: [Design system](/documentation/design_system/README.md), [Stitch token exports](/documentation/design_system/token_exports/README.md).

### synthetic load account

One of the 1,100 member accounts the staging load run signs in as, created by
`staging-fixtures seed-load-population` with the Player role and a Discord id from the reserved range
9100000000000000000 to 9100000000000099999, and removed by `clean-load-population`, which deletes
reserved accounts only. Seeding refuses while the API's Discord bot token is set or when any reserved
id already exists.

In code: `tools/staging/staging_fixtures/src/reserved_accounts.rs` and `load_population/`; `tools/xtask/staging/load_population.json`; `account_rotation.rs` in `tools/staging/staging_load_generator/src/`.

See: [load workload](/documentation/glossary/g_to_m.md#load-workload), [staging harness](#staging-harness).

### ticket

One unit of planned work, stored as `.ai/tickets/T-<id>.toml` for parents and dotted children
alike, with a status of `idea`, `queued`, `ready`, `running`, `review`, `shipped`, `deferred` or
`cancelled`. Every ticket operation is a `cargo xtask ticket` command.

In code: `load_registry` in `tools/tickets/ticket_registry/src/registry/mod.rs`; `StatusName` in `tools/tickets/ticket_model/src/model/status.rs`.

See: [wave](#wave), [ticketboard](#ticketboard), [Ticket registry](/.ai/tickets/README.md).

### ticketboard

The native desktop viewer of the ticket registry, built on egui: parent and child tickets, wave
lanes, the program tree, run receipts and estimates, with specs and documents beside them. Every
change it makes runs a `cargo xtask ticket` command.

In code: the package and binary `ticketboard_desktop` in `tools/tickets/ticketboard_desktop/` (`cargo run -p ticketboard_desktop`), which reads the registry through `tools/tickets/ticket_model/`.

See: [Ticketboard](/tools/tickets/ticketboard_desktop/README.md).

### time fuze

A fuze that bursts the shell a set time after firing, carried by the illumination shells; its
window and default come from the shell's catalog entry. The mortar calculator sets it from a burst
height: the time of flight to the burst point above the target, on the charge with the fewest
rings whose time lies inside the window. A refused setting carries no time and names its cause:
outside the window, or the burst point out of reach (above the apex, beyond range, inside the
minimum range).

In code: `TimeFuze` (`min_s`, `max_s`, `default_s`) in `crates/ballistics/ballistics_model/src/catalog/shell.rs`; `solve_time_fuze_over_charges` and `FuzeRefusal` in `crates/ballistics/fire_mission_planning/src/fuze.rs`; `FuzeSetting` and `FuzeRefusal` in `contracts/definitions/fire-mission.schema.json`.

See: [charge ring](/documentation/glossary/a_to_f.md#charge-ring), [Game ballistics engine](/documentation/crates/ballistics/game_ballistics_engine.md).

### wave

A numbered group of [tickets](#ticket) in `.ai/tickets/wave.lock`, which `cargo xtask wave repack`
alone writes. Tickets run in parallel only when the files they own do not overlap;
`cargo xtask platform wave` and `cargo xtask mod wave` drive a wave for the platform and the mod.

In code: `tools/tickets/ticket_wave_lock/src/`; `tools/xtask/src/commands/wave/cli.rs`.

See: [Factory waves](/documentation/runbooks/factory_waves/README.md).

### Workbench

Enfusion's editor application, where worlds, prefabs and mission headers are edited and plugins
run: the `tbd-export` plugins export the terrain, object and registry data the platform ingests,
and the `tbd-emcp` handlers let the Enfusion MCP tools drive Workbench from outside.

In code: `mod/tbd-export/Scripts/WorkbenchGame/`; `tools/developer_tools/src/bin/mcpd.rs`.

See: [Enfusion](/documentation/glossary/a_to_f.md#enfusion), [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md).
