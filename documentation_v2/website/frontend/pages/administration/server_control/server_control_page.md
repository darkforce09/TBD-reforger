**Status:** live

# Server control page

The `/admin/server` page, titled Server Control: administrators pick one of the configured game
servers, read its live state, issue [fleet commands](/documentation_v2/glossary.md#fleet-command)
to it and follow each to its outcome, deploy an approved mission
[artifact](/documentation_v2/glossary.md#artifact) to it, keep the
[fleet scenario](/documentation_v2/glossary.md#fleet-scenario) registry, and issue and revoke the
server's [machine credentials](/documentation_v2/glossary.md#machine-credential). Nothing here
reaches a host directly: a command or a deployment is a request the API records and answers with
202, and the page follows it to the outcome an executor or a runtime session reports.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/server_control/`](/apps/website/frontend/src/v2/pages/administration/server_control/):
  `page.rs` holds the route component `ServerControlPage`, the server list fetch and the picker;
  `server_cards.rs` the server list rows and the selected server's card; four subfolders hold
  the card's panels: `fleet_commands/` (the command console), `mission_deployments/` (the
  deployments panel), `fleet_scenarios/` (the fleet scenario sheet) and `machine_credentials/`
  (the credential sheet). The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/server_control/README.md) describes
  each file.
- Entry: the `/admin/server` route renders `ServerControlPage`
  (`apps/website/frontend/src/app_routes.rs`); `apps/website/frontend/src/router.rs` declares it
  for the `admin` tier, full-bleed, with the breadcrumb "Administration" › "Server Control", and
  the sidebar lists it as "Server Control"
  (`apps/website/frontend/src/v2/pages/navigation/nav_config.rs`).
- Related: the [server control](/documentation_v2/glossary.md#server-control) glossary entry; the
  [server intel page](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md),
  the members' read-only view of the same servers; the API's
  [server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) and
  [missions domain](/apps/website/api_v2/src/missions/README.md); the
  [fleet host agent](/apps/fleet_host_agent/README.md); the
  [fleet command ledger evidence](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  and [machine credentials evidence](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md).

## Behaviour

The page body sits in `AdminGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`): "Loading
session…" while the session restores, a sign-in prompt for a signed-out viewer, and "Admin access
required." below the `admin` [role](/documentation_v2/glossary.md#role).

### Servers and the server card

1. The server list loads on arrival: "Loading servers…" meanwhile, "Failed to load servers." on
   failure. The picker is headed "Servers" with their count and a "Fleet scenarios" button ("Which
   scenario the fleet runs for each terrain"); each row carries an "Online" dot that pulses, or
   "Offline".
2. The first active server opens, else the first one. With no servers the detail reads "No
   servers configured."; with servers but none selected, "No server selected.".
3. The card shows the server's name and address, a "Credentials" button that opens the credential
   sheet, and "LAUNCH & CONNECT", which only toasts "Launch requires the Reforger client". Three
   telemetry columns follow: "Active Personnel" (players over the maximum) and "Uptime"
   ("Nd HHh MMm", the days left out under one day); "Terrain" (capitalised, or a dash) and
   "Active Mission", which shows the current match id, since the server row names no mission;
   "Server FPS" ("x.y Hz") and "Mod Configuration" (the required modpack as "name vX", or a
   dash). A server that reports no status shows zeros and dashes.
4. The page offers no way to add, edit or deactivate a server: the registry's write routes are
   API-only.

### Fleet commands

1. The "Fleet commands" section offers "Start", "Stop", "Restart" and "List players", a broadcast
   ("Message, at most 256 bytes", "Broadcast") and a kick. "Stop" and "Restart" ask first: "Stop
   <server>? Every connected player is disconnected." or "Restart <server>? Every connected player
   is disconnected until it is back.", answered by "Stop the server" or "Restart the server", or
   "Keep it running".
2. A broadcast needs 1 to 256 bytes without line breaks or control characters. A kick needs the
   player's Arma identity (up to 128 bytes), the runtime session it is issued against, and an
   optional reason shown to the player (up to 128 bytes). The players of the newest successful
   "List players" are offered for the identity, and "Use the session that confirmed the latest
   deployment (<id>)" fills the session.
3. An accepted request toasts "<Action> accepted — waiting for the <executor> to carry it out"
   (the executor being the host agent or the game runtime), and the "Your command" panel follows
   it: "<Action> is <state> — following it until it finishes.", re-read every two seconds. The
   follow stops when the card goes away, when a newer request replaces it, or after five failed
   reads in a row ("Stopped following the command: …").
4. Only `succeeded` is announced as a success ("<Action> succeeded", with what it observed, such
   as "N player(s): …" or "No players connected"). `failed` gives the executor's reason; `expired`
   says no executor carried it out in time, so nothing ran; `cancelled` says it was cancelled
   before any executor claimed it; `indeterminate` ("Outcome unknown") says the executor stopped
   reporting after the command started, so it may or may not have taken effect, nothing repeats
   it, and the server needs inspecting before the command is issued again.
5. "History" lists the server's commands ("Loading the command history…", "No command has been
   requested for this server yet.", "Refresh"): state, action, executor and claim count, who
   requested it and when, when it was claimed, started, finished or expires, its arguments, its
   outcome and failure reason. A command still queued offers "Cancel", which answers "<Action>
   cancelled — nothing ran"; one an executor has taken up is refused as no longer cancellable.

### Mission deployments

1. The "Mission deployments" section offers "Request a deployment". The form reads its choices
   when it first opens: the live missions whose latest approval names an artifact, and the event
   missions of upcoming operations scheduled on this server. With no deployable mission it says
   "No live mission has an approved artifact to deploy. A mission becomes deployable once a
   reviewer approves its artifact.".
2. The form takes a mission ("Choose a mission…") and optionally an event mission ("None — deploy
   without binding seats"), and "Deploy" sends it. An accepted request toasts "Deployment of
   <mission> recorded — waiting for a runtime session to confirm it", and the "Your deployment"
   panel follows it every two seconds until it is confirmed, failed or cancelled.
3. A refusal is worded per code, below the form: the artifact is not the approved one of a live
   mission; its modpack differs from the server's; no fleet scenario is registered for its
   terrain; the event mission is not on an operation scheduled on this server; the event mission's
   seats and the artifact's slots do not correspond one to one, listing every unbound seat and
   unseated slot; another deployment of the server is in flight; or the server is deactivated.
4. "Deployments" lists the server's deployments, newest first ("Loading the deployments…",
   "Nothing has been deployed to this server yet.", "Refresh"), each as "In flight", "Confirmed",
   "Failed" or "Cancelled". Opened, one shows its mission, state, artifact digest, document
   SHA-256, terrain, scenario, transition ("Scenario restart — the running game loads the artifact
   and restarts in-process" or "Host restart — the host agent restarts the server on the terrain's
   scenario"), who requested it from where ("the website" or "in game") and when, the deadline,
   the bound seats, the fleet command and its state, the event mission, the runtime session that
   confirmed it, when it finished and why it failed. A deployment in flight offers "Cancel this
   deployment", refused once an executor has claimed its command.

### Fleet scenarios sheet

1. "Fleet scenarios" opens a side sheet: "A deployment runs its artifact on the scenario registered
   for the artifact's terrain; a terrain with none cannot be deployed.". It lists each terrain's
   scenario with "Updated by <who>, <UTC time>" ("you" for the viewer), "Edit" and "Remove"
   ("Loading the fleet scenarios…", "No terrain has a registered scenario yet.").
2. "Register or replace a terrain's scenario" takes a terrain key (a lowercase letter, then
   lowercase letters, digits or underscores, at most 64), a display name (1 to 128 bytes) and a
   scenario id (sixteen uppercase hex digits in braces, then a path ending in `.conf`, as in the
   placeholder "{1111222233334444}Missions/TBD_Arland.conf"); "Save scenario" answers "Terrain
   <key> runs <name>".
3. "Remove" asks first: "New deployments of this terrain are refused until a scenario is
   registered again; deployments already recorded are untouched." ("Remove the scenario" or
   "Keep"), and answers "Terrain <key> is no longer offered to new deployments".

### Machine credentials sheet

1. "Credentials" opens "Machine credentials" for the selected server, listing every credential
   without its secret ("Loading credentials…", "This server has no credentials yet."): its label,
   its program, "Issued by <who>, <when>", and "Live · last used <when>", "Live · never used" or
   "Revoked by <who>, <when>: <reason>".
2. "Issue a credential" takes a label ("Label, for example the host it runs on", 1 to 128 bytes)
   and a program: "Host agent — process control, the RCON player list and cross-terrain restarts"
   or "Game runtime — sessions, heartbeats, roster reads, broadcasts, kicks and deployments".
   "Issue credential" answers "Issued <label>" and shows the secret once, under "Store this secret
   now — it cannot be shown again", with "Copy" and "I have stored it — hide the secret". Closing
   the sheet discards it; the page never stores it.
3. "Revoke" asks for a reason (1 to 512 bytes, "Why it is revoked (kept in the audit trail)")
   and warns: "Revoking stops this credential at once and ends every game-runtime session it
   authenticated. The server's other credentials are untouched."; "Revoke credential" answers
   "Revoked <label>".

### Known discrepancies

- The deployment form offers event missions only from operations whose server is this one, and
  no page sets an operation's server: the event manager's forms never send `server_id`, which the
  events API accepts. An operation gets a server only through the API.

## Data

The page README lists no calls, so the DTOs are named here. Server-side:

- `GET /api/v1/servers` (`list_servers` in
  `apps/website/api_v2/src/server_infrastructure/handlers/server_intel.rs`): read as
  `DataEnvelope<ServerRowDto>`, every server, active or not, in name order, each with its cached
  status row, its required modpack and the terrain of its current match.
- `GET /api/v1/servers/{id}/commands` (`list_server_commands` in
  `apps/website/api_v2/src/server_infrastructure/handlers/fleet_commands.rs`): a
  `FleetCommandList` of `FleetCommandReceipt`s, newest first, 50 by default.
- `POST /api/v1/servers/{id}/commands` with a `FleetCommandRequest` (`action`, `arguments`)
  (`request_server_command`): records the command and answers 202 with its receipt; a
  deactivated server is refused with 409 "a deactivated server accepts no commands", and a kick
  against a session that has ended with 409 `RUNTIME_SESSION_ENDED`. It records
  `server.command_requested`. `start`, `stop`, `restart` and `list_players` go to the
  [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent); `broadcast` and `kick` go
  to the [game runtime](/documentation_v2/glossary.md#game-runtime) (`FleetAction` in
  `apps/website/api_v2/src/server_infrastructure/models/fleet_command.rs`). A queued command
  expires unclaimed after 300 seconds. Once an executor reports that it is executing, it has an
  execution window to report the outcome: 30 seconds for `list_players`, `broadcast` and `kick`,
  120 for `stop`, 180 for `start` and `restart`. A window that lapses leaves the command
  `indeterminate`, which nothing repeats.
- `GET /api/v1/servers/{id}/commands/{commandId}` (`get_server_command`) and
  `POST /api/v1/servers/{id}/commands/{commandId}/cancel` (`cancel_server_command`): one receipt,
  and cancellation while the command is still queued (409 `COMMAND_NOT_CANCELLABLE`, with its
  state, otherwise).
- `GET /api/v1/servers/{id}/deployments` (`list_server_deployments` in
  `apps/website/api_v2/src/missions/handlers/mission_deployments.rs`): a `MissionDeploymentPage`
  of `MissionDeployment`s, newest first, 20 by default.
- `POST /api/v1/servers/{id}/deployments` with a `DeploymentRequest` (`mission`, `artifact`,
  optional `event_mission`) (`request_server_deployment`): in one transaction the API validates
  the request, records the deployment with its seat bindings, issues its fleet command and
  records the audit entry, then answers 202. On a server already running the artifact's terrain
  the command is `load_mission`, which the game runtime carries out as a scenario restart with a
  600-second deadline; otherwise it is `restart_with_mission`, which the host agent carries out on
  the terrain's fleet scenario, a host restart with a 1200-second deadline. A deployment is
  confirmed only when a runtime session that started after the request reports the artifact with
  its exact document SHA-256. The refusal
  codes are `ARTIFACT_NOT_APPROVED`, `MODPACK_MISMATCH`, `TERRAIN_NOT_RUNNABLE`,
  `EVENT_MISSION_NOT_ON_SERVER`, `ORBAT_ARTIFACT_MISMATCH` (with every unbound seat and unseated
  slot), `DEPLOYMENT_IN_PROGRESS` and `SERVER_INACTIVE`.
- `GET /api/v1/servers/{id}/deployments/{deploymentId}` (`get_server_deployment`) and
  `POST …/{deploymentId}/cancel` (`cancel_server_deployment`): one deployment, and cancellation
  while its command is unclaimed (`DEPLOYMENT_NOT_IN_FLIGHT` or `COMMAND_NOT_CANCELLABLE`
  otherwise).
- The deployment form's choices: `GET /api/v1/missions?limit=100` (the page keeps live missions
  with an `approved_artifact_id`), `GET /api/v1/events?scope=upcoming&limit=100` (it keeps the
  operations whose `server_id` is this server) and `GET /api/v1/events/{id}` for each of those.
- `GET /api/v1/fleet/scenarios`, `PUT /api/v1/fleet/scenarios/{terrainKey}` with a
  `FleetScenarioUpdate` (`scenario_id`, `display_name`) and `DELETE` on the same path
  (`list_fleet_scenarios`, `put_fleet_scenario` and `delete_fleet_scenario` in
  `apps/website/api_v2/src/server_infrastructure/handlers/fleet_scenarios.rs`): the registry of
  `FleetScenario`s; the API checks the same patterns and records `fleet.scenario_registered` or
  `fleet.scenario_removed`.
- `GET /api/v1/servers/{id}/credentials` (`list_server_credentials` in
  `apps/website/api_v2/src/server_infrastructure/handlers/machine_credentials.rs`): a
  `MachineCredentialList` without secrets. `POST` on the same path with a
  `MachineCredentialIssue` (`label`, `executor_kind` `host_agent` or `mod_runtime`) answers an
  `IssuedMachineCredential`, the only answer that carries the secret.
  `DELETE /api/v1/servers/{id}/credentials/{credentialId}?reason=<text>`
  (`revoke_server_credential`) revokes one credential, keeping the reason in the audit trail and
  ending the game-runtime sessions it authenticated.
- The API has no RCON route. [RCON](/documentation_v2/glossary.md#rcon) is the host agent's
  business: it lists players over RCON when it carries out `list_players`.

## Design

- A full-bleed `SplitPane` over the topographic backdrop: a 17rem picker and the server card as
  the detail. The fleet scenario and credential sheets slide in from the side. The layout as built
  (the address is a placeholder):

```text
+-----------------+-----------------------------------------------------------+
| SERVERS 3       | TBD Primary — Everon          [Credentials] [LAUNCH ...]  |
| [Fleet scen.]   | <ip>:<port>                                               |
| ● Primary       |-----------------------------------------------------------|
| ○ Secondary     | ACTIVE PERSONNEL 47/64 | TERRAIN Everon | SERVER FPS 58.7 |
| ○ Staging       |-----------------------------------------------------------|
|                 | FLEET COMMANDS              | MISSION DEPLOYMENTS         |
|                 | [Start][Stop][Restart]      | [Request a deployment]      |
|                 | [List players]              |  mission v  event mission v |
|                 | Broadcast [ message ] [>]   |  [Deploy]                   |
|                 | Kick [uid][session][reason] | refusal: unbound seats …    |
|                 | Your command: queued …      | Your deployment: recorded … |
|                 | HISTORY            Refresh  | DEPLOYMENTS       Refresh   |
|                 | [Expired] Restart w/ miss.  | [Failed] Operation Iron Veil|
|                 | [Queued] List players Cancel| [Confirmed] Operation Iron… |
|                 | [Succeeded] …  [Failed] …   |   detail rows · [Cancel]    |
+-----------------+-----------------------------------------------------------+

FLEET SCENARIOS SHEET (side sheet)            CREDENTIALS SHEET (side sheet)
| arland — Arland   {1111…}TBD_Arland.conf |  | secret once · issue · list  |
|   Updated by you, …        [Edit][Remove]|  | revoke with a reason        |
| Register or replace: [terrain][name]     |
|   [scenario id]              [Save]      |
```

- No visual reference set exists for this page, and the archived platform spec has no section
  for it; the built layout is the reference.

## Open work

None.

## Decisions

- A command's 202 is an acceptance, not an outcome: the page follows the receipt until it is
  terminal and announces it once, so success is claimed only when an executor reports it.
- An `indeterminate` command is never repeated for the administrator: it may already have taken
  effect, so the page says the outcome is unknown and asks for the server to be inspected first.
- A deployment is confirmed by a runtime session reporting the artifact's exact document SHA-256,
  not by its fleet command succeeding: a restart that loads the wrong document is not a
  deployment.
- A credential's secret is shown once and never stored in the page: closing the sheet discards
  it, and the list shows credentials without secrets.
- Process control, the player list, broadcasts and kicks are fleet commands and loading a mission
  is a deployment; the page has no RCON console.
