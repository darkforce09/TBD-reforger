**Status:** live

# Glossary terms A to F

The glossary's entries from A to F, in alphabetical order, each in the format of the
[glossary entry template](/documentation/standards/templates/glossary_entry.md). The
[glossary index](/documentation/glossary/README.md) lists every term and says how a document
links one.

### acknowledgement-dropping relay

A loopback HTTP relay on the staging host between the API and the
[fleet host agent](#fleet-host-agent) of one [fleet instance](#fleet-instance) (instance 5). Armed
over its control socket, it holds back the API's next 200 answer to a claim or to a result past the
agent's 20-second timeout and then closes the connection, so the staging fleet check proves that a
lost acknowledgement neither repeats nor loses a [fleet command](#fleet-command); it never stores or
logs the `Authorization` header.

In code: the `acknowledgement-dropping-relay` executable (`serve`; `control arm drop-next-claim-response|drop-next-result-response`, `disarm`, `status`) in `tools/developer_tools/src/staging_verification/acknowledgement_relay/`; `deploy/systemd/acknowledgement-dropping-relay@.service`.

See: [staging harness](/documentation/glossary/n_to_z.md#staging-harness), [Acknowledgement relay](/tools/developer_tools/src/staging_verification/acknowledgement_relay/README.md).

### administration

The administrator-only side of the platform: the API domain of the member roster, bans, warnings,
membership grace, the Discord role resync and the audit log, and the seven `/admin/*` pages.

In code: `apps/api/src/administration/`; `apps/frontend/src/pages/administration/`.

See: [event manager](#event-manager), [approvals](#approvals), [server control](/documentation/glossary/n_to_z.md#server-control), [personnel](/documentation/glossary/n_to_z.md#personnel), [content manager](#content-manager), [audit logs](#audit-logs).

### after-action review

The review of a finished match, abbreviated AAR: in the game, the END banner and the DEBRIEF
scoreboard; on the website, a map replay that is planned and not built, with a reserved workspace.

In code: `apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/`; `apps/frontend/src/workspaces/aar/`.

See: [After-action review](/documentation/apps/frontend/workspaces/aar/after_action_review.md).

### API

The website's backend: the Axum REST API under `/api/v1` and its Server-Sent Events streams over
Postgres, in eight domains beside a shared `core` and the [background workers](#background-workers).
Documents say the API; the crate is `api`, in the folder `apps/api/`.

In code: `apps/api/` (library `api`, binaries `api` and `import-registry`); `apps/api/src/core/http_router.rs` merges the domain route tables.

See: [Website API](/apps/api/README.md).

### approvals

The mission approval queue at `/admin/approvals`, titled Mission Approvals: an administrator
reviews the [artifact](#artifact) a mission maker submitted and approves it into the live library,
optionally with conditions, or returns it to the author with a reason.

In code: `MissionApprovalsPage` in `apps/frontend/src/pages/administration/approvals/`; `apps/api/src/missions/handlers/approvals_queue.rs`.

See: [Mission approvals page](/documentation/apps/frontend/pages/administration/approvals/mission_approvals_page.md).

### armory

The weapons, vehicles and equipment a [mission](/documentation/glossary/g_to_m.md#mission) makes available per faction, each with an
optional quantity (none is unlimited), shown on the mission overview; not the [arsenal](#arsenal).

In code: `MissionArmory` in `apps/api/src/missions/models/mission.rs`; `apps/api/src/missions/handlers/mission_armory.rs`.

See: [Mission overview page](/documentation/apps/frontend/pages/mission_hub/overview/mission_overview_page.md).

### arsenal

The Mission Creator's Arsenal tab, where a mission maker edits one [slot](/documentation/glossary/n_to_z.md#slot)'s loadout (weapons,
wear, attachments, cargo) with a doll preview and weight and validity checks. Its catalog is the
item [registry](/documentation/glossary/n_to_z.md#registry), which the code calls the Virtual Arsenal catalog.

In code: `apps/frontend/src/workspaces/editor/arsenal/`.

See: [armory](#armory), [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator).

### artifact

The immutable compiled form of one [mission](/documentation/glossary/g_to_m.md#mission) version. Submitting a mission compiles its
current version into an artifact, a review decides exactly that artifact, and a
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) runs an approved one on a server, which fetches it by ID.

In code: `MissionArtifact` in `apps/api/src/missions/services/mission_artifacts/artifact_store.rs`; `mission_submission.rs` and `game_runtime_missions.rs` in `apps/api/src/missions/handlers/`.

See: [Mission artifacts evidence](/documentation/apps/api/verification_evidence/mission_artifacts.md).

### audit logs

The trail of administrative actions at `/admin/audit`, newest first: the page follows the live
[SSE](/documentation/glossary/n_to_z.md#sse) feed, resuming after a drop from the last event id,
loads older entries a page at a time, shows each entry once whichever way it arrived, filters the
loaded entries by text in the browser and inspects one entry. The API also serves a CSV export,
which the page does not use.

In code: `AuditLogsPage` in `apps/frontend/src/pages/administration/audit_logs/`; `apps/frontend/src/foundation/transport/audit_stream.rs`; `apps/api/src/administration/handlers/audit_logs.rs`.

See: [Audit logs page](/documentation/apps/frontend/pages/administration/audit_logs/audit_logs_page.md).

### background workers

The interval tasks the [API](#api) binary starts at boot and never awaits: token purge, event
lifecycle, leaderboards, server status, Discord roles and membership, rate limits, audit
publication, reservations, runtime sessions, fleet commands and mission deployments.

In code: `spawn_all` and `WorkerHandles` in `apps/api/src/background_workers/mod.rs`.

See: [Background workers](/apps/api/src/background_workers/README.md).

### charge ring

One of the propellant increments on a mortar shell; the number of rings fitted is the shell's
charge, and each charge multiplies the shell's initial speed by its own coefficient. A shell's
charges are listed by ring count in a ballistics catalog, with one marked default; the firing
solver solves every charge and recommends the one with the fewest rings that reaches the target.

In code: `Charge` (`rings`, `init_speed_coef`, `is_default`) in `legacy/map_engine/src/data/scenario/ballistics/catalog/shell.rs`; `charges` in `contracts/definitions/ballistics-catalog.schema.json`; the game's `SCR_MortarShellGadgetComponent` `m_aChargeRingConfig`.

See: [probable error](/documentation/glossary/n_to_z.md#probable-error), [time fuze](/documentation/glossary/n_to_z.md#time-fuze), [Game ballistics engine](/documentation/legacy/map_engine/data/scenario/ballistics/game_ballistics_engine.md).

### closing-fix batch

A list of small, independent findings the orchestrator queues during a sub-agent program and hands
to one agent near the close, named `G1`, `G2` and on; a single defect found late (by a final gate or
the live walkthrough) gets a closing-fix agent of its own under the same numbering. It is not a
follow-up agent (`<ID>b`), which takes one narrow slice as soon as its owner has reported.

In code: none; the queued items live in the orchestrator's session scratchpad, and each batch is a
row of the program's execution record and amendments table.

See: [orchestrator](/documentation/glossary/n_to_z.md#orchestrator), [Sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md#routing-a-finding).

### command center

The web app's landing area (the dashboard at `/`, server intel, announcements) and the API domain of
the dashboard, leaderboards and player statistics; not the [orchestrator](/documentation/glossary/n_to_z.md#orchestrator).

In code: `apps/frontend/src/pages/command_center/`; `apps/api/src/command_center/`.

See: [Command center domain](/apps/api/src/command_center/README.md).

### community content

The API domain of what the community reads (announcements, the doctrine wiki, the vehicle database,
modpack manifests) and the CMS routes the [content manager](#content-manager) writes through.

In code: `apps/api/src/community_content/`.

See: [Community content domain](/apps/api/src/community_content/README.md).

### console command

The [fleet command](#fleet-command) `console_command`: one line an administrator types (1 to 256
bytes, no control characters, no leading `@`), sent to the game server's
[RCON](/documentation/glossary/n_to_z.md#rcon) console by its [fleet host agent](#fleet-host-agent).
The agent may repeat the login but transmits the line once and never resends it; the outcome keeps
the reply up to 4,096 bytes with a truncation flag, and without a reply the command may or may not
have run.

In code: `FleetAction::ConsoleCommand` in `crates/contracts/fleet_wire_contract/src/fleet_action.rs`; `HostCommand::ConsoleCommand` in `apps/fleet_host_agent/src/command_execution/host_command.rs` and `SessionRequest::ExecuteOnce` in `apps/fleet_host_agent/src/rcon/rcon_session.rs`; `console_command_form.rs` in `apps/frontend/src/pages/administration/server_control/fleet_commands/`.

See: [RCON](/documentation/glossary/n_to_z.md#rcon), [API decisions](/documentation/apps/api/decisions.md).

### content manager

The `/admin/content` page, whose breadcrumb reads Comms Broadcaster: administrators write, publish,
edit and delete announcements, upload a hero image, and push a post to Discord.

In code: `ContentManagerPage` in `apps/frontend/src/pages/administration/content_manager/`.

See: [Content manager page](/documentation/apps/frontend/pages/administration/content_manager/content_manager_page.md).

### damage-driven render

The rendering rule of every map canvas and of the Arsenal's doll: a frame is encoded and submitted
only while something that would be drawn has changed (the damage flag) or continuous rendering is
on, so an idle `requestAnimationFrame` tick returns without touching the GPU.

In code: `RenderDamage` and `FrameDecision` in `crates/graphics/render_primitives/src/frame/damage.rs`; `mark_dirty` and `set_continuous_render` on the [render engine](/documentation/glossary/n_to_z.md#render-engine) in `legacy/map_engine/src/frame/lifecycle.rs`; the frame pump in `legacy/graphics_engine/src/loop/`; the pins in `legacy/map_engine/src/frame/tests/damage_discipline.rs`.

See: [frame packet](#frame-packet), [Engine boundary rules](/documentation/standards/engine_boundary_rules.md).

### DEM

Digital elevation model: a terrain's ground height as a raster. Everon's is one 6400 × 6400 16-bit
greyscale image at 2 m per pixel, which the map engine decodes into metres for the hillshade, the
contour lines, the sea band, the height readout and line-of-sight walks.

In code: `crates/terrain/terrain_elevation/src/` (`DemVectorGrid` in `grid.rs`); `legacy/map_engine/src/editing/tools/line_of_sight/terrain_survey.rs`; `assets/terrains/everon/dem/everon-dem-16bit.png`.

See: [Elevation model](/crates/terrain/terrain_elevation/README.md), [Everon elevation model](/assets/terrains/everon/dem/README.md).

### deployment

Four meanings: a [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) runs an approved artifact on a game
server; a member's deployments are the events on their [service record](/documentation/glossary/n_to_z.md#service-record); a
game-runtime deployment puts one player life into a slot; a website deployment ships the platform.

In code: `mission_deployments.rs` in `apps/api/src/missions/handlers/`; `member_service_record.rs` and `game_runtime_deployments.rs` in `apps/api/src/operations/handlers/`.

See: [Website deployment runbook](/documentation/runbooks/website_deployment.md).

### dev login

A development-only sign-in without Discord: with `APP_ENV=development`, the dev-login route signs in
as a local account of the requested [role](/documentation/glossary/n_to_z.md#role) (`admin` for an unknown one) and redirects to
`/auth/callback` with the token in the URL fragment; elsewhere it answers 404.

In code: `dev_login` in `apps/api/src/identity_and_access/handlers/developer_login.rs`, serving `GET /api/v1/auth/dev-login?role=<role>`.

See: [Local development](/documentation/runbooks/local_development.md).

### Eden

Most often the Eden editor, Arma 3's scenario editor: the design the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) is measured against,
catalogued interaction by interaction with an ID each. In [Enfusion](#enfusion) resource paths,
Eden is also Everon's world, `worlds/Eden/Eden.ent`, which the mod's worlds inherit.

In code: the Mission Creator's shell measurements taken from Eden in `apps/frontend/src/workspaces/editor/session/layout.rs`; `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent` and `apps/mod/tbd-export/worlds/TBD_Export_Everon.ent`, whose parent is `worlds/Eden/Eden.ent`.

See: [Eden editor reference](/documentation/apps/frontend/workspaces/editor/eden_editor_reference/README.md), [Eden gap analysis](/documentation/apps/frontend/workspaces/editor/eden_editor_reference/eden_gap_analysis.md).

### EnfScript

Enforce Script, Enfusion's C-like scripting language, in `.c` files under an addon's `Scripts/`:
`Scripts/Game/` compiles into the game and the dedicated server, `Scripts/WorkbenchGame/` into
Workbench. Its comment rules are in the [documentation standards](/documentation/standards/documentation_standards.md#6-enfusion-comments).

In code: `apps/mod/tbd-framework/Scripts/Game/TBD/`; `cargo xtask mod compile`, the compile gate.

See: [Enfusion](#enfusion), [mod](/documentation/glossary/g_to_m.md#mod).

### Enfusion

Bohemia Interactive's engine behind Arma Reforger: the game, its dedicated server and the Workbench
editor run on it. Its scripts are [EnfScript](#enfscript), and its content ships as addons.

In code: the three addons under `apps/mod/`, each with an `addon.gproj`.

See: [Workbench](/documentation/glossary/n_to_z.md#workbench), [mission header](/documentation/glossary/g_to_m.md#mission-header), [mod](/documentation/glossary/g_to_m.md#mod).

### event

A scheduled community session record: start time, briefing, attached [missions](/documentation/glossary/g_to_m.md#mission) with their
own start times, their [ORBAT](/documentation/glossary/n_to_z.md#orbat) slots, sign-ups and waitlist. "Event" alone means this record
(never an SSE or DOM event); code and screen titles also say operation.

In code: `Event` and `EventMission` in `apps/api/src/operations/models/event.rs`; the `event_*.rs` handlers in `apps/api/src/operations/handlers/`.

See: [slot](/documentation/glossary/n_to_z.md#slot), [Event schedule page](/documentation/apps/frontend/pages/operations/schedule/event_schedule_page.md).

### event manager

The `/admin/events` page, titled the operations calendar: a month grid and a day panel from which
administrators schedule, edit and cancel [events](#event), attach missions and set who may join.

In code: `EventManagerPage` in `apps/frontend/src/pages/administration/event_manager/`; `apps/api/src/operations/handlers/event_create_update.rs`.

See: [Event manager page](/documentation/apps/frontend/pages/administration/event_manager/event_manager_page.md).

### factory

The platform's agent build process: an [orchestrator](/documentation/glossary/n_to_z.md#orchestrator)
session takes one [wave](/documentation/glossary/n_to_z.md#wave) of file-disjoint tickets at a
time, gives each to a [slice](/documentation/glossary/n_to_z.md#slice) agent in its own git
worktree, lands the slices whose [gate](/documentation/glossary/g_to_m.md#gate) passed on `main`
and has one adversarial verifier attack the result. The mod program runs the same shape.

In code: `cargo xtask platform wave` and `cargo xtask platform slice-worktree` in `tools/xtask/src/commands/platform/`; `cargo xtask mod wave` in `tools/xtask/src/commands/mod_ops/wave_execution/`.

See: [Factory waves](/documentation/runbooks/factory_waves/README.md), [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md).

### feature doc

A document under `documentation/` for one feature or page area, a layer below the code README:
where it lives, behaviour, data, design, open work and decisions. It sits beside its folder's
README index and is named after what it covers (`<page component>_page.md`,
`<screen>_specification.md` or a subject name); the code README links it and never repeats it.

In code: none; the code README of the folder a feature doc describes links it under Related documentation, as `apps/frontend/src/pages/administration/personnel/README.md` links `personnel_roster_page.md`.

See: [Feature doc template](/documentation/standards/templates/feature_doc.md), [README standard](/documentation/standards/readme_standard.md).

### fleet command

One operator command to one game server (`start`, `stop`, `restart`, `list_players`, `broadcast`,
`kick`, `console_command`), kept in the API's command ledger from acceptance through an executor's claim to its
outcome; [mission deployments](/documentation/glossary/g_to_m.md#mission-deployment) alone issue `load_mission` and `restart_with_mission`.

In code: `FleetAction` in `crates/contracts/fleet_wire_contract/src/fleet_action.rs`; `fleet_commands.rs` and `fleet_executor.rs` in `apps/api/src/server_infrastructure/handlers/`.

See: [fleet host agent](#fleet-host-agent), [console command](#console-command), [game runtime](/documentation/glossary/g_to_m.md#game-runtime), [server control](/documentation/glossary/n_to_z.md#server-control).

### fleet host agent

The program beside each Arma Reforger dedicated server of a game host, one per fleet instance: it
polls the API outbound over HTTPS for the [fleet commands](#fleet-command) addressed to its server,
performs process control, [RCON](/documentation/glossary/n_to_z.md#rcon) commands and scenario switches, and reports each step; the API never connects in.

In code: `apps/fleet_host_agent/`; `deploy/systemd/fleet_host_agent@.service`, one
`fleet_host_agent@N.service` per fleet instance. The package name `fleet_host_agent` is also the
binary, the configuration folder `~/.config/fleet_host_agent/instance-N/` and the HTTP user agent
`fleet_host_agent/<version>`.

See: [machine credential](/documentation/glossary/g_to_m.md#machine-credential), [Fleet host agent](/apps/fleet_host_agent/README.md).

### fleet instance

One of the game servers a staging host runs side by side, numbered 1 to `TBD_FLEET_INSTANCES`
(five on staging): instance N is the registered server "TBD Staging N", with its own dedicated server
unit `tbd-reforger@N.service`, its own [fleet host agent](#fleet-host-agent)
`fleet_host_agent@N.service`, its own [machine credentials](/documentation/glossary/g_to_m.md#machine-credential) and RCON password, and the game port
2000+N, the A2S port 17776+N and the loopback RCON port 19998+N.

In code: `tools/xtask/src/commands/deploy/staging/fleet_instances.rs`; `deploy/systemd/tbd-reforger@.service` and `fleet_host_agent@.service`; on the host, `~/tbd/fleet/instance-N/`.

See: [acknowledgement-dropping relay](#acknowledgement-dropping-relay), [Deploy staging](/tools/xtask/src/commands/deploy/staging/README.md).

### fleet scenario

An entry of the registry that names, for each terrain, the [mission header](/documentation/glossary/g_to_m.md#mission-header) the
fleet boots; a mission deployment to a terrain without one is refused. The code says scenario here.

In code: `apps/api/src/server_infrastructure/handlers/fleet_scenarios.rs`.

See: [scenario](/documentation/glossary/n_to_z.md#scenario), [server control](/documentation/glossary/n_to_z.md#server-control).

### frame packet

One frame's whole draw list for the graphics engine: camera, clear colour, batches, glyph runs and
indirect draws in ascending [lane](/documentation/glossary/g_to_m.md#lane) order, and the pipelines and bind groups they use.

In code: `FramePacket` in `legacy/graphics_engine/src/frame/packet.rs`.

See: [Graphics engine frame](/legacy/graphics_engine/src/frame/README.md).
