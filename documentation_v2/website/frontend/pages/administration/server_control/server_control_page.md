**Status:** live

# Server control page

The `/admin/server` page, titled Server Control: administrators pick one of the configured game
servers, read its live state, issue [fleet commands](/documentation_v2/glossary.md#fleet-command) to
it and follow each to its outcome, deploy a [mission](/documentation_v2/glossary.md#mission)'s
approved [artifact](/documentation_v2/glossary.md#artifact) to it, keep the
[registry](/documentation_v2/glossary.md#registry) of
[fleet scenarios](/documentation_v2/glossary.md#fleet-scenario), and issue and revoke the server's
[machine credentials](/documentation_v2/glossary.md#machine-credential). Nothing here reaches a host
directly: a command or a deployment is a request the [API](/documentation_v2/glossary.md#api)
records and answers with 202, and the page follows it to the outcome an executor or a runtime
session reports.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/server_control/`](/apps/website/frontend/src/v2/pages/administration/server_control/):
  `page.rs` holds the route component `ServerControlPage`, the server list fetch and the picker;
  `server_cards.rs` the server list rows and the selected server's card; four subfolders hold
  the card's panels: `fleet_commands/` (the command console), `mission_deployments/` (the
  deployments panel), `fleet_scenarios/` (the fleet scenario sheet) and `machine_credentials/`
  (the credential sheet). The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/server_control/README.md) describes
  each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/administration/server_control/README.md#routes).
- Related: the [server control](/documentation_v2/glossary.md#server-control) glossary entry; the
  [server intel page](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md),
  the members' read-only view of the same servers; the API's
  [server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) and
  [missions domain](/apps/website/api_v2/src/missions/README.md); the
  [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) and its
  [README](/apps/fleet_host_agent/README.md); the
  [fleet command ledger evidence](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  and [machine credentials evidence](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md).

## Behaviour

The page body sits in `AdminGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows
the session and access states of the README's
[States](/apps/website/frontend/src/v2/pages/administration/server_control/README.md#states) in
place of the page until a signed-in viewer holds the `admin`
[role](/documentation_v2/glossary.md#role). The README's States quote every text the steps below
mention.

### Servers and the server card

1. The server list loads on arrival. The picker counts the servers, offers the "Fleet scenarios"
   button, and marks each server online, with a pulsing dot, or offline.
2. The first active server opens, else the first one; the detail says so when there are no
   servers, or none is selected.
3. The card shows the server's name and address, a "Credentials" button that opens the credential
   sheet, and a launch control that only toasts that the Reforger client is needed. Three
   telemetry columns follow: the players over the maximum and the uptime; the terrain and the
   active mission, which shows the current match id, since the server row names no mission; the
   server FPS and the required modpack. A server that reports no status shows zeros and dashes.
4. The page offers no way to add, edit or deactivate a server: the registry's write routes are
   API-only.

### Fleet commands

1. The "Fleet commands" section offers start, stop, restart and list players, a broadcast and a
   kick. Stop and restart ask first, since every connected player is disconnected.
2. A broadcast needs 1 to 256 bytes without line breaks or control characters. A kick needs the
   player's Arma identity (up to 128 bytes), the runtime session it is issued against, and an
   optional reason shown to the player (up to 128 bytes). The players of the newest successful
   player listing are offered for the identity, and the session that confirmed the latest
   deployment is offered for the session.
3. An accepted request is toasted as waiting for its executor, the host agent or the
   [game runtime](/documentation_v2/glossary.md#game-runtime), and the "Your command" panel
   follows it, re-read every two seconds. The follow stops when the card goes away, when a newer
   request replaces it, or after five failed reads in a row.
4. Only `succeeded` is announced as a success, with what the command observed. `failed` gives the
   executor's reason; `expired` says no executor carried it out in time, so nothing ran;
   `cancelled` says it was cancelled before any executor claimed it; `indeterminate` says the
   executor stopped reporting after the command started, so it may or may not have taken effect,
   nothing repeats it, and the server needs inspecting before the command is issued again.
5. "History" lists the server's commands: state, action, executor and claim count, who requested
   it and when, when it was claimed, started, finished or expires, its arguments, its outcome and
   failure reason. A command still queued offers "Cancel"; one an executor has taken up is
   refused as no longer cancellable.

### Mission deployments

1. The "Mission deployments" section offers "Request a deployment". The form reads its choices
   when it first opens: the live missions whose latest approval names an artifact, and the
   [event](/documentation_v2/glossary.md#event) missions of upcoming operations scheduled on this
   server. With no deployable mission it says that a mission becomes deployable once a reviewer
   approves its artifact.
2. The form takes a mission and optionally an event mission, whose seats the deployment binds,
   and "Deploy" sends it. An accepted request is toasted as waiting for a runtime session to
   confirm it, and the "Your deployment" panel follows it every two seconds until it is
   confirmed, failed or cancelled.
3. A refusal is worded per code, below the form: the artifact is not the approved one of a live
   mission; its modpack differs from the server's; no fleet scenario is registered for its
   terrain; the event mission is not on an operation scheduled on this server; the event mission's
   seats and the artifact's [slots](/documentation_v2/glossary.md#slot) do not correspond one to
   one, listing every unbound seat and unseated slot; another deployment of the server is in
   flight; or the server is deactivated.
4. "Deployments" lists the server's deployments, newest first, each with its state. Opened, one
   shows its mission, state, artifact digest, document SHA-256, terrain, scenario, transition (a
   scenario restart, in which the running game loads the artifact, or a host restart, in which
   the host agent restarts the server on the terrain's scenario), who requested it from where and
   when, the deadline, the bound seats, the fleet command and its state, the event mission, the
   runtime session that confirmed it, when it finished and why it failed. A deployment in flight
   offers "Cancel this deployment", refused once an executor has claimed its command.

### Fleet scenarios sheet

1. "Fleet scenarios" opens a side sheet that explains the rule: a deployment runs its artifact on
   the scenario registered for the artifact's terrain, and a terrain with none cannot be
   deployed. It lists each terrain's scenario with who last updated it and when, "Edit" and
   "Remove".
2. The registration form takes a terrain key (a lowercase letter, then lowercase letters, digits
   or underscores, at most 64), a display name (1 to 128 bytes) and a scenario id (sixteen
   uppercase hex digits in braces, then a path ending in `.conf`).
3. "Remove" asks first: new deployments of the terrain are refused until a scenario is registered
   again, and deployments already recorded are untouched.

### Machine credentials sheet

1. "Credentials" opens the selected server's credential sheet, listing every credential without
   its secret: its label, its program, who issued it and when, and its last use or who revoked it,
   when and why.
2. The issue form takes a label (1 to 128 bytes) and a program, the host agent or the game
   runtime. An issued credential's secret shows once, with a copy control and a control that
   hides it; closing the sheet discards it, and the page never stores it.
3. "Revoke" asks for a reason (1 to 512 bytes, kept in the audit trail) and warns that revoking
   stops the credential at once and ends every game-runtime session it authenticated, leaving the
   server's other credentials untouched.

### Known discrepancies

- The deployment form offers event missions only from operations whose server is this one, and
  no page sets an operation's server: the event manager's forms never send `server_id`, which the
  events API accepts. An operation gets a server only through the API.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/administration/server_control/README.md#data)
lists each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/servers` (`list_servers` in
  `apps/website/api_v2/src/server_infrastructure/handlers/server_intel.rs`): every server,
  active or not, in name order, each with its cached
  status row, its required modpack and the terrain of its current match.
- `GET /api/v1/servers/{id}/commands` (`list_server_commands` in
  `apps/website/api_v2/src/server_infrastructure/handlers/fleet_commands.rs`): the server's
  commands, newest first, 50 by default.
- `POST /api/v1/servers/{id}/commands` (`request_server_command`): records the command and answers 202 with its receipt; a
  deactivated server is refused with 409 "a deactivated server accepts no commands", and a kick
  against a session that has ended with 409 `RUNTIME_SESSION_ENDED`. It records
  `server.command_requested`. `start`, `stop`, `restart` and `list_players` go to the fleet host
  agent; `broadcast` and `kick` go to the game runtime (`FleetAction` in
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
  `apps/website/api_v2/src/missions/handlers/mission_deployments.rs`): the server's deployments,
  newest first, 20 by default.
- `POST /api/v1/servers/{id}/deployments` (`request_server_deployment`): in one transaction the API validates
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
  `POST /api/v1/servers/{id}/deployments/{deploymentId}/cancel` (`cancel_server_deployment`): one
  deployment, and cancellation while its command is unclaimed (`DEPLOYMENT_NOT_IN_FLIGHT` or
  `COMMAND_NOT_CANCELLABLE` otherwise).
- The deployment form's choices: `GET /api/v1/missions?limit=100` (the page keeps live missions
  with an `approved_artifact_id`), `GET /api/v1/events?scope=upcoming&limit=100` (it keeps the
  operations whose `server_id` is this server) and `GET /api/v1/events/{id}` for each of those.
- `GET /api/v1/fleet/scenarios`, `PUT /api/v1/fleet/scenarios/{terrainKey}` and `DELETE` on the
  same path (`list_fleet_scenarios`, `put_fleet_scenario` and `delete_fleet_scenario` in
  `apps/website/api_v2/src/server_infrastructure/handlers/fleet_scenarios.rs`): the registry of
  fleet scenarios; the API checks the same patterns and records `fleet.scenario_registered` or
  `fleet.scenario_removed`.
- `GET /api/v1/servers/{id}/credentials` (`list_server_credentials` in
  `apps/website/api_v2/src/server_infrastructure/handlers/machine_credentials.rs`): the
  server's credentials without secrets. `POST` on the same path issues one, for `host_agent` or
  `mod_runtime`, and its answer is the only one that carries the secret.
  `DELETE /api/v1/servers/{id}/credentials/{credentialId}?reason=<text>`
  (`revoke_server_credential`) revokes one credential, keeping the reason in the audit trail and
  ending the game-runtime sessions it authenticated.
- The API has no [RCON](/documentation_v2/glossary.md#rcon) route. RCON is the host agent's
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

- [T-1022 — Add website admin UI to manage game servers](/.ai/tickets/T-1022.toml) (idea, no
  plan): the page creates, edits and deactivates servers, which the API already allows, and the
  event manager sets an operation's server, so the deployment form can offer its event missions.

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
