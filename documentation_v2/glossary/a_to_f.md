**Status:** live

# Glossary terms A to F

The glossary's entries from A to F, in alphabetical order, each in the format of the
[glossary entry template](/documentation_v2/standards/templates/glossary_entry.md). The
[glossary index](/documentation_v2/glossary/README.md) lists every term and says how a document
links one.

### administration

The administrator-only side of the platform: the API domain of the member roster, bans, warnings,
membership grace, the Discord role resync and the audit log, and the six `/admin/*` pages.

In code: `apps/website/api_v2/src/administration/`; `apps/website/frontend/src/v2/pages/administration/`.

See: [event manager](#event-manager), [approvals](#approvals), [server control](/documentation_v2/glossary/n_to_z.md#server-control), [personnel](/documentation_v2/glossary/n_to_z.md#personnel), [content manager](#content-manager), [audit logs](#audit-logs).

### after-action review

The review of a finished match, abbreviated AAR: in the game, the END banner and the DEBRIEF
scoreboard; on the website, a map replay that is planned and not built, with a reserved workspace.

In code: `apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/`; `apps/website/frontend/src/v2/apps/aar/`.

See: [After-action review](/documentation_v2/website/frontend/apps/aar/after_action_review.md).

### API

The website's backend: the Axum REST API under `/api/v1` and its Server-Sent Events streams over
Postgres, in eight domains beside a shared `core` and the [background workers](#background-workers).
Documents say the API; the crate is `website-api`, in the folder `api_v2`.

In code: `apps/website/api_v2/` (library `website_api`, binaries `api` and `import-registry`); `apps/website/api_v2/src/core/http_router.rs` merges the domain route tables.

See: [Website API](/apps/website/api_v2/README.md).

### approvals

The mission approval queue at `/admin/approvals`, titled Mission Approvals: an administrator
reviews the [artifact](#artifact) a mission maker submitted and approves it into the live library,
optionally with conditions, or returns it to the author with a reason.

In code: `MissionApprovalsPage` in `apps/website/frontend/src/v2/pages/administration/approvals/`; `apps/website/api_v2/src/missions/handlers/approvals_queue.rs`.

See: [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md).

### armory

The weapons, vehicles and equipment a [mission](/documentation_v2/glossary/g_to_m.md#mission) makes available per faction, each with an
optional quantity (none is unlimited), shown on the mission overview; not the [arsenal](#arsenal).

In code: `MissionArmory` in `apps/website/api_v2/src/missions/models/mission.rs`; `apps/website/api_v2/src/missions/handlers/mission_armory.rs`.

See: [Mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md).

### arsenal

The Mission Creator's Arsenal tab, where a mission maker edits one [slot](/documentation_v2/glossary/n_to_z.md#slot)'s loadout (weapons,
wear, attachments, cargo) with a doll preview and weight and validity checks. Its catalog is the
item [registry](/documentation_v2/glossary/n_to_z.md#registry), which the code calls the Virtual Arsenal catalog.

In code: `apps/website/frontend/src/v2/apps/editor/arsenal/`.

See: [armory](#armory), [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator).

### artifact

The immutable compiled form of one [mission](/documentation_v2/glossary/g_to_m.md#mission) version. Submitting a mission compiles its
current version into an artifact, a review decides exactly that artifact, and a
[mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) runs an approved one on a server, which fetches it by ID.

In code: `MissionArtifact` in `apps/website/api_v2/src/missions/services/mission_artifacts/artifact_store.rs`; `mission_submission.rs` and `game_runtime_missions.rs` in `apps/website/api_v2/src/missions/handlers/`.

See: [Mission artifacts evidence](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md).

### audit logs

The trail of administrative actions at `/admin/audit`, newest first: the page loads it a page at a
time, filters the loaded entries by text in the browser and inspects one entry. The API also serves
a CSV export and a live [SSE](/documentation_v2/glossary/n_to_z.md#sse) feed, which the page does not use.

In code: `AuditLogsPage` in `apps/website/frontend/src/v2/pages/administration/audit_logs/`; `apps/website/api_v2/src/administration/handlers/audit_logs.rs`.

See: [Audit logs page](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md).

### background workers

The interval tasks the [API](#api) binary starts at boot and never awaits: token purge, event
lifecycle, leaderboards, server status, Discord roles and membership, rate limits, audit
publication, reservations, runtime sessions, fleet commands and mission deployments.

In code: `spawn_all` and `WorkerHandles` in `apps/website/api_v2/src/background_workers/mod.rs`.

See: [Background workers](/apps/website/api_v2/src/background_workers/README.md).

### command center

The web app's landing area (the dashboard at `/`, server intel, announcements) and the API domain of
the dashboard, leaderboards and player statistics; not the [orchestrator](/documentation_v2/glossary/n_to_z.md#orchestrator).

In code: `apps/website/frontend/src/v2/pages/command_center/`; `apps/website/api_v2/src/command_center/`.

See: [Command center domain](/apps/website/api_v2/src/command_center/README.md).

### community content

The API domain of what the community reads (announcements, the doctrine wiki, the vehicle database,
modpack manifests) and the CMS routes the [content manager](#content-manager) writes through.

In code: `apps/website/api_v2/src/community_content/`.

See: [Community content domain](/apps/website/api_v2/src/community_content/README.md).

### content manager

The `/admin/content` page, whose breadcrumb reads Comms Broadcaster: administrators write, publish,
edit and delete announcements, upload a hero image, and push a post to Discord.

In code: `ContentManagerPage` in `apps/website/frontend/src/v2/pages/administration/content_manager/`.

See: [Content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md).

### damage-driven render

The rendering rule of every map canvas and of the Arsenal's doll: a frame is encoded and submitted
only while something that would be drawn has changed (the damage flag) or continuous rendering is
on, so an idle `requestAnimationFrame` tick returns without touching the GPU.

In code: `RenderDamage` and `FrameDecision` in `apps/website/graphics-engine/src/frame/damage.rs`; `mark_dirty` and `set_continuous_render` on the [render engine](/documentation_v2/glossary/n_to_z.md#render-engine) in `apps/website/map-engine/src/frame/lifecycle.rs`; the frame pump in `apps/website/graphics-engine/src/loop/`; the pins in `apps/website/map-engine/src/frame/tests/damage_discipline.rs`.

See: [frame packet](#frame-packet), [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md).

### DEM

Digital elevation model: a terrain's ground height as a raster. Everon's is one 6400 × 6400 16-bit
greyscale image at 2 m per pixel, which the map engine decodes into metres for the hillshade, the
contour lines, the sea band, the height readout and line-of-sight walks.

In code: `apps/website/map-engine/src/world/terrain/dem/` (`DemVectorGrid` in `grid.rs`); `apps/website/map-engine/src/editing/tools/line_of_sight/terrain_survey.rs`; `assets_v2/terrains/everon/dem/everon-dem-16bit.png`.

See: [Elevation model](/apps/website/map-engine/src/world/terrain/dem/README.md), [Everon elevation model](/assets_v2/terrains/everon/dem/README.md).

### deployment

Four meanings: a [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) runs an approved artifact on a game
server; a member's deployments are the events on their [service record](/documentation_v2/glossary/n_to_z.md#service-record); a
game-runtime deployment puts one player life into a slot; a website deployment ships the platform.

In code: `mission_deployments.rs` in `apps/website/api_v2/src/missions/handlers/`; `member_service_record.rs` and `game_runtime_deployments.rs` in `apps/website/api_v2/src/operations/handlers/`.

See: [Website deployment runbook](/documentation_v2/runbooks/website_deployment.md).

### dev login

A development-only sign-in without Discord: with `APP_ENV=development`, the dev-login route signs in
as a local account of the requested [role](/documentation_v2/glossary/n_to_z.md#role) (`admin` for an unknown one) and redirects to
`/auth/callback` with the token in the URL fragment; elsewhere it answers 404.

In code: `dev_login` in `apps/website/api_v2/src/identity_and_access/handlers/developer_login.rs`, serving `GET /api/v1/auth/dev-login?role=<role>`.

See: [Local development](/documentation_v2/runbooks/local_development.md).

### Eden

Most often the Eden editor, Arma 3's scenario editor: the design the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) is measured against,
catalogued interaction by interaction with an ID each. In [Enfusion](#enfusion) resource paths,
Eden is also Everon's world, `worlds/Eden/Eden.ent`, which the mod's worlds inherit.

In code: the Mission Creator's shell measurements taken from Eden in `apps/website/frontend/src/v2/apps/editor/shell/layout.rs`; `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent` and `apps/mod/tbd-export/worlds/TBD_Export_Everon.ent`, whose parent is `worlds/Eden/Eden.ent`.

See: [Eden editor reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/README.md), [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md).

### EnfScript

Enforce Script, Enfusion's C-like scripting language, in `.c` files under an addon's `Scripts/`:
`Scripts/Game/` compiles into the game and the dedicated server, `Scripts/WorkbenchGame/` into
Workbench. Its comment rules are in the [documentation standards](/documentation_v2/standards/documentation_standards.md#6-enfusion-comments).

In code: `apps/mod/tbd-framework/Scripts/Game/TBD/`; `cargo xtask mod compile`, the compile gate.

See: [Enfusion](#enfusion), [mod](/documentation_v2/glossary/g_to_m.md#mod).

### Enfusion

Bohemia Interactive's engine behind Arma Reforger: the game, its dedicated server and the Workbench
editor run on it. Its scripts are [EnfScript](#enfscript), and its content ships as addons.

In code: the three addons under `apps/mod/`, each with an `addon.gproj`.

See: [Workbench](/documentation_v2/glossary/n_to_z.md#workbench), [mission header](/documentation_v2/glossary/g_to_m.md#mission-header), [mod](/documentation_v2/glossary/g_to_m.md#mod).

### event

A scheduled community session record: start time, briefing, attached [missions](/documentation_v2/glossary/g_to_m.md#mission) with their
own start times, their [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) slots, sign-ups and waitlist. "Event" alone means this record
(never an SSE or DOM event); code and screen titles also say operation.

In code: `Event` and `EventMission` in `apps/website/api_v2/src/operations/models/event.rs`; the `event_*.rs` handlers in `apps/website/api_v2/src/operations/handlers/`.

See: [slot](/documentation_v2/glossary/n_to_z.md#slot), [Event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md).

### event manager

The `/admin/events` page, titled the operations calendar: a month grid and a day panel from which
administrators schedule, edit and cancel [events](#event), attach missions and set who may join.

In code: `EventManagerPage` in `apps/website/frontend/src/v2/pages/administration/event_manager/`; `apps/website/api_v2/src/operations/handlers/event_create_update.rs`.

See: [Event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md).

### factory

The platform's agent build process: an [orchestrator](/documentation_v2/glossary/n_to_z.md#orchestrator)
session takes one [wave](/documentation_v2/glossary/n_to_z.md#wave) of file-disjoint tickets at a
time, gives each to a [slice](/documentation_v2/glossary/n_to_z.md#slice) agent in its own git
worktree, lands the slices whose [gate](/documentation_v2/glossary/g_to_m.md#gate) passed on `main`
and has one adversarial verifier attack the result. The mod program runs the same shape.

In code: `cargo xtask platform wave` and `cargo xtask platform slice-worktree` in `tools_v2/xtask/src/commands/platform/`; `cargo xtask mod wave` in `tools_v2/xtask/src/commands/mod_ops/wave_execution/`.

See: [Factory waves](/documentation_v2/runbooks/factory_waves/README.md), [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md).

### feature doc

A document under `documentation_v2/` for one feature or page area, a layer below the code README:
where it lives, behaviour, data, design, open work and decisions. It sits beside its folder's
README index and is named after what it covers (`<page component>_page.md`,
`<screen>_specification.md` or a subject name); the code README links it and never repeats it.

In code: none; the code README of the folder a feature doc describes links it under Related documentation, as `apps/website/frontend/src/v2/pages/administration/personnel/README.md` links `personnel_roster_page.md`.

See: [Feature doc template](/documentation_v2/standards/templates/feature_doc.md), [README standard](/documentation_v2/standards/readme_standard.md).

### fleet command

One operator command to one game server (`start`, `stop`, `restart`, `list_players`, `broadcast`,
`kick`), kept in the API's command ledger from acceptance through an executor's claim to its
outcome; [mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment) alone issue `load_mission` and `restart_with_mission`.

In code: `FleetAction` in `apps/website/api_v2/src/server_infrastructure/models/fleet_command.rs`; `fleet_commands.rs` and `fleet_executor.rs` in `apps/website/api_v2/src/server_infrastructure/handlers/`.

See: [fleet host agent](#fleet-host-agent), [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime), [server control](/documentation_v2/glossary/n_to_z.md#server-control).

### fleet host agent

The program on each game host beside the Arma Reforger dedicated server: it polls the API outbound
over HTTPS for the [fleet commands](#fleet-command) addressed to its server, performs process
control, [RCON](/documentation_v2/glossary/n_to_z.md#rcon) commands and scenario switches, and reports each step; the API never connects in.

In code: `apps/fleet_host_agent/`; `tools_v2/xtask/deploy/systemd/fleet-host-agent.service`.

See: [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential), [Fleet host agent](/apps/fleet_host_agent/README.md).

### fleet scenario

An entry of the registry that names, for each terrain, the [mission header](/documentation_v2/glossary/g_to_m.md#mission-header) the
fleet boots; a mission deployment to a terrain without one is refused. The code says scenario here.

In code: `apps/website/api_v2/src/server_infrastructure/handlers/fleet_scenarios.rs`.

See: [scenario](/documentation_v2/glossary/n_to_z.md#scenario), [server control](/documentation_v2/glossary/n_to_z.md#server-control).

### frame packet

One frame's whole draw list for the graphics engine: camera, clear colour, batches, glyph runs and
indirect draws in ascending [lane](/documentation_v2/glossary/g_to_m.md#lane) order, and the pipelines and bind groups they use.

In code: `FramePacket` in `apps/website/graphics-engine/src/frame/packet.rs`.

See: [Graphics engine frame](/apps/website/graphics-engine/src/frame/README.md).
