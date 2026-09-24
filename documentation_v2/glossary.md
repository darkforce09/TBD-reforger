**Status:** live

# Glossary

The terms the documents use with a meaning particular to this project, and their abbreviations,
one entry each in alphabetical order, in the format of the
[glossary entry template](/documentation_v2/standards/templates/glossary_entry.md). A document links
a term's first use to its entry, as `[mission](/documentation_v2/glossary.md#mission)`, so a heading
keeps its wording. Code identifiers keep their own spelling; an entry says where the code differs.

### administration

The administrator-only side of the platform: in the API, the domain of the member roster, bans,
warnings, membership grace, the Discord role resync and the audit log; in the web app, the six
`/admin/*` pages, which also call other domains.

In code: `apps/website/api_v2/src/administration/`; `apps/website/frontend/src/v2/pages/administration/`.

See: [event manager](#event-manager), [approvals](#approvals), [server control](#server-control),
[personnel](#personnel), [content manager](#content-manager), [audit logs](#audit-logs).

### API

The website's backend: the Axum REST API under `/api/v1` and its Server-Sent Events streams over
Postgres, in eight domains beside a shared `core` and the [background workers](#background-workers).
Documents say the API; the crate is `website-api`, in the folder `api_v2`.

In code: `apps/website/api_v2/` (library `website_api`, binaries `api` and `import-registry`);
`apps/website/api_v2/src/core/http_router.rs` merges the domain route tables.

See: [Website API](/apps/website/api_v2/README.md).

### approvals

The mission approval queue at `/admin/approvals`, titled Mission Approvals: an administrator
reviews the [artifact](#artifact) a mission maker submitted and approves it into the live library,
optionally with conditions, or returns it to the author with a reason.

In code: `MissionApprovalsPage` in `apps/website/frontend/src/v2/pages/administration/approvals/`;
`apps/website/api_v2/src/missions/handlers/approvals_queue.rs`.

See: [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md).

### armory

The weapons, vehicles and equipment a [mission](#mission) makes available, per faction, each line
with an optional quantity (none means unlimited), shown on the mission overview; not the
[arsenal](#arsenal).

In code: `MissionArmory` in `apps/website/api_v2/src/missions/models/mission.rs`;
`apps/website/api_v2/src/missions/handlers/mission_armory.rs`.

See: [Mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md).

### arsenal

The Mission Creator's Arsenal tab, where a mission maker edits one [slot](#slot)'s loadout (weapons,
wear, attachments, cargo) with a doll preview and weight and validity checks. Its catalog is the
item [registry](#registry), which the code calls the Virtual Arsenal catalog.

In code: `apps/website/frontend/src/v2/apps/editor/arsenal/`.

See: [armory](#armory), [Mission Creator](#mission-creator).

### artifact

The immutable compiled form of one [mission](#mission) version. Submitting a mission compiles its
current version into an artifact, a review decides exactly that artifact, and a
[mission deployment](#mission-deployment) runs an approved one on a server, which fetches it by ID.

In code: `MissionArtifact` in `apps/website/api_v2/src/missions/services/mission_artifacts/artifact_store.rs`;
`mission_submission.rs` and `game_runtime_missions.rs` in `apps/website/api_v2/src/missions/handlers/`.

See: [Mission artifacts evidence](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md).

### audit logs

The trail of administrative actions at `/admin/audit`, newest first: the page loads it a page at a
time, filters the loaded entries by text in the browser and inspects one entry. The API also serves
a CSV export and a live [SSE](#sse) feed, which the page does not use.

In code: `AuditLogsPage` in `apps/website/frontend/src/v2/pages/administration/audit_logs/`;
`apps/website/api_v2/src/administration/handlers/audit_logs.rs`.

See: [Audit logs page](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md).

### background workers

The interval tasks the [API](#api) binary starts at boot and never awaits, each keeping shared state
current without a request: token purge, event lifecycle, leaderboards, server status, Discord roles
and membership, rate limits, audit publication, reservations, runtime sessions, fleet commands and
mission deployments.

In code: `spawn_all` and `WorkerHandles` in `apps/website/api_v2/src/background_workers/mod.rs`.

See: [Background workers](/apps/website/api_v2/src/background_workers/README.md).

### command center

In the web app, the members' landing area: the dashboard at `/`, server intel and announcements. In
the API, the domain of the dashboard, the leaderboards and per-player statistics.

In code: `apps/website/frontend/src/v2/pages/command_center/`; `apps/website/api_v2/src/command_center/`.

See: [Command center domain](/apps/website/api_v2/src/command_center/README.md).

### community content

The API domain of what the community reads: announcements, the doctrine wiki, the vehicle database
and modpack manifests, with the CMS routes for announcements and uploads that the
[content manager](#content-manager) writes through.

In code: `apps/website/api_v2/src/community_content/`.

See: [Community content domain](/apps/website/api_v2/src/community_content/README.md).

### content manager

The `/admin/content` page, whose breadcrumb reads Comms Broadcaster: administrators write, publish,
edit and delete announcements, upload a hero image, and push a post to Discord.

In code: `ContentManagerPage` in `apps/website/frontend/src/v2/pages/administration/content_manager/`.

See: [Content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md).

### deployment

A word with four meanings; documents say which. A [mission deployment](#mission-deployment) runs an
approved artifact on a game server. A member's deployments are the events they took part in, on
their [service record](#service-record). A game-runtime deployment authorizes one player life into
an ORBAT slot. A website deployment ships the platform to its host (`cargo xtask deploy website`).

In code: `mission_deployments.rs` in `apps/website/api_v2/src/missions/handlers/`;
`member_service_record.rs` and `game_runtime_deployments.rs` in `apps/website/api_v2/src/operations/handlers/`.

See: [Website deployment runbook](/documentation_v2/runbooks/website_deployment.md).

### dev login

A development-only sign-in without Discord: with `APP_ENV=development`,
`GET /api/v1/auth/dev-login?role=<role>` signs in as a fixed local account of that [role](#role)
(any other value signs in as `admin`) and redirects to `/auth/callback` with the token in the URL
fragment. Outside development the route answers 404.

In code: `dev_login` in `apps/website/api_v2/src/identity_and_access/handlers/developer_login.rs`.

See: [Local development](/documentation_v2/runbooks/local_development.md).

### EnfScript

Enforce Script, Enfusion's C-like scripting language, in `.c` files under an addon's `Scripts/`:
`Scripts/Game/` compiles into the game and the dedicated server, `Scripts/WorkbenchGame/` into
Workbench. Its comment rules are in the [documentation standards](/documentation_v2/standards/documentation_standards.md#6-enfusion-comments).

In code: `apps/mod/tbd-framework/Scripts/Game/TBD/`; `cargo xtask mod compile`, the compile gate.

See: [Enfusion](#enfusion), [mod](#mod).

### Enfusion

Bohemia Interactive's engine behind Arma Reforger: the game, its dedicated server and the Workbench
editor run on it. Its scripts are [EnfScript](#enfscript), and its content ships as addons.

In code: the three addons under `apps/mod/`, each with an `addon.gproj`.

See: [Workbench](#workbench), [mission header](#mission-header), [mod](#mod).

### event

A scheduled community session record: its start time, briefing, the [missions](#mission) attached
(each with its own start time), their [ORBAT](#orbat) slots, sign-ups and waitlist. "Event" alone
means this record; other kinds are named in full (SSE event, DOM event, script event). Code comments
and screen titles also call an event an operation, as in the operations calendar.

In code: `Event` and `EventMission` in `apps/website/api_v2/src/operations/models/event.rs`; the
`event_*.rs` handlers in `apps/website/api_v2/src/operations/handlers/`.

See: [slot](#slot), [Event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md).

### event manager

The `/admin/events` page, titled the operations calendar: a month grid and a day panel from which
administrators schedule, edit and cancel [events](#event), attach missions and set who may join.

In code: `EventManagerPage` in `apps/website/frontend/src/v2/pages/administration/event_manager/`;
`apps/website/api_v2/src/operations/handlers/event_create_update.rs`.

See: [Event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md).

### fleet command

One operator command to one game server (`start`, `stop`, `restart`, `list_players`, `broadcast`,
`kick`), kept in the API's command ledger from acceptance through an executor's claim to its
outcome; [mission deployments](#mission-deployment) alone issue `load_mission` and `restart_with_mission`.

In code: `FleetAction` in `apps/website/api_v2/src/server_infrastructure/models/fleet_command.rs`;
`fleet_commands.rs` and `fleet_executor.rs` in `apps/website/api_v2/src/server_infrastructure/handlers/`.

See: [fleet host agent](#fleet-host-agent), [game runtime](#game-runtime), [server control](#server-control).

### fleet host agent

The program on each game host beside the Arma Reforger dedicated server: it polls the API outbound
over HTTPS for the [fleet commands](#fleet-command) addressed to its server, performs process
control, [RCON](#rcon) commands and scenario switches, and reports each step; the API never connects in.

In code: `apps/fleet_host_agent/`; `tools_v2/xtask/deploy/systemd/fleet-host-agent.service`.

See: [machine credential](#machine-credential), [Fleet host agent](/apps/fleet_host_agent/README.md).

### fleet scenario

An entry of the registry that names, for each terrain, the [mission header](#mission-header) the
fleet boots; a mission deployment to a terrain without one is refused. The code says scenario here.

In code: `apps/website/api_v2/src/server_infrastructure/handlers/fleet_scenarios.rs`.

See: [scenario](#scenario), [server control](#server-control).

### game runtime

The TBD framework mod running inside a dedicated server, seen from the API: with a `mod_runtime`
[machine credential](#machine-credential) it opens and ends runtime sessions, sends heartbeats, reads
rosters and artifacts, authorizes player lives into slots, and runs broadcasts, kicks and loads.

In code: the `/api/v1/game-runtime/` routes; `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.

See: [fleet command](#fleet-command), [deployment](#deployment).

### identity and access

The API domain of sign-in and the caller's own account: Discord OAuth2 login, token refresh and
logout, the [dev login](#dev-login), the caller's profile at `/api/v1/me`, and the handshake that
links a Discord account to an Arma identity.

In code: `apps/website/api_v2/src/identity_and_access/`.

See: [Identity and access domain](/apps/website/api_v2/src/identity_and_access/README.md).

### machine credential

A per-server secret that authenticates one program on a game host: `host_agent` for the
[fleet host agent](#fleet-host-agent) or `mod_runtime` for the [game runtime](#game-runtime). An
administrator issues one (its secret shows once), lists them without secrets and revokes each alone.

In code: `MachineCredential` and `ExecutorKind` in `apps/website/api_v2/src/server_infrastructure/models/machine_credential.rs`.

See: [Machine credentials evidence](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md).

### match telemetry

The API domain that takes in what game servers report: runtime-session heartbeats with the live
server status, and finished match results behind the service token.

In code: `apps/website/api_v2/src/match_telemetry/`.

See: [Match telemetry domain](/apps/website/api_v2/src/match_telemetry/README.md).

### mission

The platform document a mission maker authors in the [Mission Creator](#mission-creator), with its
versions, reviews, [artifacts](#artifact) and deployments; its status is `draft`, `pending_approval`,
`live`, `rejected` or `archived`. It is not the [mission header](#mission-header) a server boots.

In code: `Mission`, `MissionVersion` and `MissionStatus` in `apps/website/api_v2/src/missions/models/mission.rs`;
`contracts_v2/definitions/mission.schema.json`; some code spells it [scenario](#scenario).

See: [missions](#missions), [event](#event).

### Mission Creator

The 2D/3D CAD editor in which mission makers build a [mission](#mission) on the map, at
`/missions/:id/edit`, for the `mission_maker` [role](#role) and above. Prose never calls it the
Scenario Creator; code identifiers say editor.

In code: `apps/website/frontend/src/v2/apps/editor/`; `MissionEditorPage` in its `mission_editor.rs`.

See: [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md).

### mission deployment

A request that runs an approved [artifact](#artifact) on one game server: the API issues a
[fleet command](#fleet-command) (`load_mission` on the same terrain, `restart_with_mission` for
another), follows it to confirmation, and cancels it while no executor has claimed it.

In code: `apps/website/api_v2/src/missions/handlers/mission_deployments.rs`;
`contracts_v2/definitions/mission-deployment.schema.json`.

See: [deployment](#deployment), [fleet scenario](#fleet-scenario).

### mission header

Enfusion's world plus game-mode configuration that a dedicated server boots: an
`SCR_MissionHeader` config naming the world, the game mode, and the name and description players
see. It is not a [mission](#mission), which the mod's mission loader loads into the running game.

In code: `apps/mod/tbd-framework/Missions/` (`TBD_Dev_POC.conf`); `game.scenarioId` in
`tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json`; the code says scenario.

See: [fleet scenario](#fleet-scenario), [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md).

### missions

The API domain that owns [missions](#mission): the library, the versions the Mission Creator saves,
submission and [approvals](#approvals), artifacts, mission deployments, the [armory](#armory), the
faction library, the item [registry](#registry) and the game-runtime routes that serve artifacts.

In code: `apps/website/api_v2/src/missions/`.

See: [Missions domain](/apps/website/api_v2/src/missions/README.md).

### mod

The Arma Reforger modification the repository ships, in three addons (Enfusion packages, each with
an `addon.gproj`): `tbd-framework`, the game mod that runs TBD sessions; `tbd-export`, the Workbench
export tooling; `tbd-emcp`, the Workbench bridge handlers the Enfusion MCP tools call.

In code: `apps/mod/tbd-framework/`, `apps/mod/tbd-export/`, `apps/mod/tbd-emcp/`.

See: [EnfScript](#enfscript), [Mod suite](/apps/mod/README.md).

### operations

The domain around [events](#event): the event calendar, the [ORBAT](#orbat) and slotting, squad
reservations and the waitlist, member search, [service records](#service-record) and leave requests;
the API domain also serves the mortar fire-mission tools and the game-runtime roster and deployments.

In code: `apps/website/api_v2/src/operations/`; `apps/website/frontend/src/v2/pages/operations/`.

See: [Operations domain](/apps/website/api_v2/src/operations/README.md).

### ORBAT

The order of battle: the factions, squads and role [slots](#slot) of one mission within an event,
which members fill. A mission carries its ORBAT in its document; each event mission holds it as
`orbat_slots` rows, read grouped by faction and squad.

In code: `OrbatSlot` in `apps/website/api_v2/src/operations/models/event.rs`;
`apps/website/api_v2/src/operations/handlers/orbat_view.rs`.

See: [ORBAT selection page](/apps/website/frontend/src/v2/pages/operations/orbat_selection/README.md).

### personnel

The `/admin/personnel` page, titled Personnel Roster: the member roster beside one member's dossier,
where administrators ban and warn members with a reason and run the Discord role resync. A member's
[role](#role) follows their Discord roles, so the dossier explains it and never sets it.

In code: `PersonnelRosterPage` in `apps/website/frontend/src/v2/pages/administration/personnel/`.

See: [Personnel roster page](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md).

### RCON

BattlEye RCon, the remote-console protocol the Arma Reforger dedicated server speaks over UDP; the
[fleet host agent](#fleet-host-agent) uses it to list players. The platform has no RCON console,
and broadcasts and kicks run in the game runtime, which knows its players by identity; Reforger's
RCON has no broadcast command.

In code: `apps/fleet_host_agent/src/rcon/`; `FleetAction` in
`apps/website/api_v2/src/server_infrastructure/models/fleet_command.rs`.

See: [fleet command](#fleet-command).

### registry

Most often the item registry: one modpack's flat catalog of the engine items the
[arsenal](#arsenal) offers, with a graph of what fits in or on what, exported from Workbench and
imported into Postgres. Other registries are named in full (ticket, server, fleet scenario).

In code: `RegistryItem` and `RegistryCompatEdge` in `apps/website/api_v2/src/missions/models/registry.rs`;
`contracts_v2/catalogs/`.

See: [Contract catalogs](/contracts_v2/catalogs/README.md).

### role

An account's tier on the permission ladder, lowest first: `guest`, `enlisted`, `leader`,
`mission_maker`, `admin`; a route's access tier is the lowest role it admits. A member's role
follows their Discord roles through the `discord_roles` mappings; the website sets none itself.

In code: `Role` in `apps/website/frontend/src/v2/core/auth/role.rs`; `role_rank` in
`apps/website/api_v2/src/core/middleware/mod.rs`.

See: [dev login](#dev-login), [personnel](#personnel).

### scenario

A code spelling, never a prose term. Platform code that says scenario means a [mission](#mission)
(the map engine's mission domain); fleet code means a [mission header](#mission-header); Enfusion's
own names (`scenarioId`, the `SCR_EScenario*` types) keep it. Prose says mission or mission header.

In code: `apps/website/map-engine/src/data/scenario/`;
`apps/website/api_v2/src/server_infrastructure/handlers/fleet_scenarios.rs`.

See: [fleet scenario](#fleet-scenario).

### server control

The `/admin/server` page: the configured game servers with their live state and, for the selected
server, its [fleet commands](#fleet-command), [mission deployments](#mission-deployment) and
[machine credentials](#machine-credential), with the [fleet scenario](#fleet-scenario) registry.

In code: `ServerControlPage` in `apps/website/frontend/src/v2/pages/administration/server_control/`.

See: [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md).

### server infrastructure

The API domain of the game servers: the server registry, each server's status and live [SSE](#sse)
feed, [machine credentials](#machine-credential), the [fleet command](#fleet-command) ledger and its
executor routes, runtime sessions and the [fleet scenario](#fleet-scenario) registry.

In code: `apps/website/api_v2/src/server_infrastructure/`.

See: [RCON](#rcon), [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md).

### service record

A member's own record of service at `/deployments`, titled My Deployments: aggregate combat
figures, upcoming deployments, past match participation and leave requests.

In code: `apps/website/api_v2/src/operations/handlers/member_service_record.rs`;
`DeploymentsPage` in `apps/website/frontend/src/v2/pages/operations/deployments/`.

See: [Deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md).

### slot

One fillable position in an [ORBAT](#orbat): a faction, squad, callsign, role and loadout that one
member occupies, and in the game a spawn position. Slotting fills them: members reserve a slot or
join the waitlist, squad managers assign seats, and players claim their slot in the game's lobby.

In code: `OrbatSlot` in `apps/website/api_v2/src/operations/models/event.rs`; `slot_registration.rs`
and `slot_assignment.rs` in `apps/website/api_v2/src/operations/handlers/`.

See: [event](#event), [arsenal](#arsenal).

### SSE

Server-Sent Events: the one-way HTTP streams on which the API pushes live updates, such as a
server's status feed and the audit log feed. An SSE event is one message on a stream, never an
[event](#event).

In code: `Hub` in `apps/website/api_v2/src/core/realtime_hub/mod.rs`; the client in
`apps/website/frontend/src/v2/core/api/sse.rs`.

See: [audit logs](#audit-logs), [server infrastructure](#server-infrastructure).

### ticket

One unit of planned work, stored as `.ai/tickets/T-<id>.toml` for parents and dotted children
alike, with a status of `idea`, `queued`, `ready`, `running`, `review`, `shipped`, `deferred` or
`cancelled`. Every ticket operation is a `cargo xtask ticket` command.

In code: `load_registry` in `tools_v2/ticket-engine/src/registry/mod.rs`; `StatusName` in
`tools_v2/ticket-engine/src/model/status.rs`.

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

See: [Enfusion](#enfusion), [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md).
