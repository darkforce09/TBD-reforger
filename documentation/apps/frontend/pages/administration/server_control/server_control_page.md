**Status:** live

# Server control page

The `/admin/server` page, titled Server Control: administrators register game servers, change
their registration and take them out of service or back, pick one of the configured servers, read
its live state, issue [fleet commands](/documentation/glossary/a_to_f.md#fleet-command) to
it and follow each to its outcome, deploy a [mission](/documentation/glossary/g_to_m.md#mission)'s
approved [artifact](/documentation/glossary/a_to_f.md#artifact) to it, keep the
[registry](/documentation/glossary/n_to_z.md#registry) of
[fleet scenarios](/documentation/glossary/a_to_f.md#fleet-scenario), and issue and revoke the server's
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential). Nothing here reaches a host
directly: a command or a deployment is a request the [API](/documentation/glossary/a_to_f.md#api)
records and answers with 202, and the page follows it to the outcome an executor or a runtime
session reports.

## Where it lives

- Code: [`apps/frontend/src/pages/administration/server_control/`](/apps/frontend/src/pages/administration/server_control/):
  `page.rs` holds the route component `ServerControlPage`, the list states and the picker;
  `server_cards.rs` the server list rows and the selected server's card, whose telemetry band is
  `server_card_telemetry.rs`; `server_registry/` holds the server list, the selection and the
  registration sheet; four subfolders hold
  the card's panels: `fleet_commands/` (the command console), `mission_deployments/` (the
  deployments panel), `fleet_scenarios/` (the fleet scenario sheet) and `machine_credentials/`
  (the credential sheet). The folder's
  [README](/apps/frontend/src/pages/administration/server_control/README.md) describes
  each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/frontend/src/pages/administration/server_control/README.md#routes).
- Related: the [server control](/documentation/glossary/n_to_z.md#server-control) glossary entry; the
  [server intel page](/documentation/apps/frontend/pages/command_center/server_intel/server_intel_page.md),
  the members' read-only view of the same servers; the API's
  [server infrastructure domain](/apps/api/src/server_infrastructure/README.md) and
  [missions domain](/apps/api/src/missions/README.md); the
  [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) and its
  [README](/apps/fleet_host_agent/README.md); the
  [fleet command ledger evidence](/documentation/apps/api/verification_evidence/fleet_command_ledger.md)
  and [machine credentials evidence](/documentation/apps/api/verification_evidence/machine_credentials.md).

## Behaviour

The page body sits in `AdminGate` (`apps/frontend/src/foundation/auth/gates.rs`), which shows
the session and access states of the README's
[States](/apps/frontend/src/pages/administration/server_control/README.md#states) in
place of the page until a signed-in viewer holds the `admin`
[role](/documentation/glossary/n_to_z.md#role). The README's States quote every text the steps below
mention.

### Servers and the server card

1. The server list loads on arrival. The picker counts the servers, offers the "Fleet scenarios"
   button, marks each server online, with a pulsing dot, or offline, and ends with "Add server". A
   server outside the configured fleet (`is_active` false) carries an "Inactive" badge.
2. The first active server opens, else the first one; the detail says when none is selected. With
   no server configured it says so, explains that a registered server is what gets machine
   credentials, fleet commands and deployments, and offers "Add server".
3. The card shows the server's name (with the "Inactive" badge when it is inactive), its address
   and its id, an "Edit" button that opens the registration sheet on it, a "Credentials" button
   that opens the
   credential sheet, and a launch control that only toasts
   that the Reforger client is needed. Four telemetry columns follow: the players over the maximum
   and the uptime; the terrain and the active mission, which shows the current match id, since
   the server row names no mission; the server FPS and the required modpack; the telemetry queue
   the game runtime last reported, as backlog over capacity with "Dropped" (in the error tone
   above zero), "Oldest" (the age of the oldest waiting entry) and "Reported" (when the API stored
   the reading, in the viewer's zone). A server that reports no status shows zeros and dashes; a
   server that never reported a queue reading shows "No reading" in the queue column. The
   [telemetry specification](/documentation/apps/api/verification_evidence/telemetry.md#game-runtime-telemetry-queue)
   defines the reading.

### Registering and editing servers

1. "Add server" opens the registration sheet, "Add a server": a name, an address, a game port
   and an optional required modpack, chosen from the modpacks read when the sheet opens. "Edit" on
   a card opens it as "Server settings", filled from the server's row.
2. The form is checked as the API checks it before anything is sent: the name is trimmed and
   required; the address must be a literal IPv4 or IPv6 address — not a hostname, and not a
   `/mask` — and an address typed with its port is told to put the port in its own field; the game
   port is a whole number from 1 to 65535. The address is sent in its canonical form.
3. "Register server" adds the answered server at the end of the picker and selects it, so its
   card, with "Credentials", shows at once. "Save changes" sends only the fields that differ from
   the server's row, clearing the required modpack with `null`; with nothing changed it says so
   and sends nothing. A refusal shows the API's sentence in the sheet and keeps the form.
4. The sheet's "Service" section says whether the server is active. "Deactivate" asks first,
   explaining that the server's host agent and game runtime are refused at once, that it takes no
   fleet command, deployment or new credential, that members stop seeing it, and that nothing is
   deleted. A deactivated server offers "Reactivate", which restores it with the same
   credentials. Each change updates the card in place, so its command console, deployments and
   credential sheet keep their state.

### Fleet commands

1. The "Fleet commands" section offers start, stop, restart and list players, the "Server console"
   box, a broadcast and a kick. Stop and restart ask first, since every connected player is
   disconnected.
2. A broadcast needs 1 to 256 bytes without line breaks or control characters. A kick needs the
   player's Arma identity (up to 128 bytes), the runtime session it is issued against, and an
   optional reason shown to the player (up to 128 bytes). The players of the newest successful
   player listing are offered for the identity, and the session that confirmed the latest
   deployment is offered for the session. A console line may hold no control character and no
   line or paragraph separator anywhere in what was typed; once trimmed it needs 1 to 256 bytes
   and must not start with `@`, which begins the [RCON](/documentation/glossary/n_to_z.md#rcon)
   commands for the host agent's own RCON session. The trimmed line is what is sent.
3. The console box sits under the process-control buttons, because the host agent carries out a
   `console_command` over RCON, as it does the player list. "Send" and Enter in the field both
   send the line; nothing is sent while another request is in flight, and a sent line leaves the
   field, since the host agent transmits a line once and nothing repeats it.
4. An accepted request is toasted as waiting for its executor, the host agent or the
   [game runtime](/documentation/glossary/g_to_m.md#game-runtime), and the "Your command" panel
   follows it, re-read every two seconds. The follow stops when the card goes away, when a newer
   request replaces it, or after five failed reads in a row.
5. Only `succeeded` is announced as a success, with what the command observed. `failed` gives the
   executor's reason; `expired` says no executor carried it out in time, so nothing ran;
   `cancelled` says it was cancelled before any executor claimed it; `indeterminate` says the
   executor stopped reporting after the command started, so it may or may not have taken effect,
   nothing repeats it, and the server needs inspecting before the command is issued again.
6. A succeeded console command says what the server replied: its one line quoted (at most 120
   characters of it), how many lines it holds, or that it was empty, and that the host agent kept
   only its first 4096 bytes when it cut the reply. The reply shows whole, as the server sent it,
   under "Your command" and under the command's history row. A line the server never answered
   fails with "no RCON response; the command may or may not have run".
7. "History" lists the server's commands: state, action, executor and claim count, who requested
   it and when, when it was claimed, started, finished or expires, its arguments, its outcome and
   failure reason. A command still queued offers "Cancel"; one an executor has taken up is
   refused as no longer cancellable.

### Mission deployments

1. The "Mission deployments" section offers "Request a deployment". The form reads its choices
   when it first opens: the live missions whose latest approval names an artifact, and the
   [event](/documentation/glossary/a_to_f.md#event) missions of upcoming operations scheduled on this
   server. With no deployable mission it says that a mission becomes deployable once a reviewer
   approves its artifact.
2. The form takes a mission and optionally an event mission, whose seats the deployment binds,
   and "Deploy" sends it. An accepted request is toasted as waiting for a runtime session to
   confirm it, and the "Your deployment" panel follows it every two seconds until it is
   confirmed, failed or cancelled.
3. A refusal is worded per code, below the form: the artifact is not the approved one of a live
   mission; its modpack differs from the server's; no fleet scenario is registered for its
   terrain; the event mission is not on an operation scheduled on this server; the event mission's
   seats and the artifact's [slots](/documentation/glossary/n_to_z.md#slot) do not correspond one to
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

## Data

The README's [Data](/apps/frontend/src/pages/administration/server_control/README.md#data)
lists each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/servers` (`list_servers` in
  `apps/api/src/server_infrastructure/handlers/server_intel.rs`): every server,
  active or not, in name order, each with its cached
  status row, its required modpack and the terrain of its current match.
- `POST /api/v1/servers` (`create_server` in
  `apps/api/src/server_infrastructure/handlers/server_registry.rs`): registers a
  server, active unless the body says otherwise, and answers 201 with its row, whose `status` and
  `terrain` are `null` until the server reports. It trims the name and refuses a blank one
  ("name is required"), refuses an `ip` that is not a literal address ("ip must be a literal IPv4
  or IPv6 address — not a hostname, and not a /mask"): the column is `inet`, which cannot hold a
  hostname and would silently drop a mask. It refuses a port outside 1 to 65535 and an unknown
  `required_modpack_id`, and records `server.create` in the transaction that writes the row
  (`register_server` in
  `apps/api/src/server_infrastructure/services/server_registration.rs`).
- `PATCH /api/v1/servers/{id}` (`update_server`): changes the fields the body names, with the same
  checks; `required_modpack_id: null` clears the requirement, `is_active` deactivates or
  reactivates, and a body naming nothing is refused. It answers the changed row and records
  `server.update`, at warning severity when it deactivates.
- `DELETE /api/v1/servers/{id}` (`deactivate_server`): sets `is_active` to false and nothing
  else, answers 204 even for a server already inactive, and records `server.deactivate` at
  warning severity. The row, its credentials and its history stay; there is no hard delete. A
  deactivated server's machine credentials are refused (403 "the credential's server is
  deactivated"), it is refused commands, deployments (`SERVER_INACTIVE`) and new credentials, and
  members no longer list it.
- `GET /api/v1/modpacks` (`list_modpacks` in the community content domain): every modpack, for
  the registration form's required-modpack choice.
- `GET /api/v1/servers/{id}/commands` (`list_server_commands` in
  `apps/api/src/server_infrastructure/handlers/fleet_commands.rs`): the server's
  commands, newest first, 50 by default.
- `POST /api/v1/servers/{id}/commands` (`request_server_command`): records the command and answers 202 with its receipt; a
  deactivated server is refused with 409 "a deactivated server accepts no commands", and a kick
  against a session that has ended with 409 `RUNTIME_SESSION_ENDED`. It records
  `server.command_requested`, whose audit row carries a console command's line. `start`, `stop`,
  `restart`, `list_players` and `console_command` go to the fleet host agent; `broadcast` and
  `kick` go to the game runtime (`FleetAction` in
  `crates/contracts/fleet_wire_contract/src/fleet_action.rs`). A console line is
  stored trimmed and refused with 400 outside 1 to 256 bytes, with a control character or a line
  or paragraph separator, or starting with `@`; a succeeded console command reports
  `{response, response_truncated}`, the server's reply cut by the host agent on a character
  boundary at 4096 bytes. A queued command expires unclaimed after 300 seconds. Once an executor
  reports that it is executing, it has an execution window to report the outcome: 30 seconds for
  `list_players`, `broadcast`, `kick` and `console_command`, 120 for `stop`, 180 for `start` and
  `restart`. A window that lapses leaves the command `indeterminate`, which nothing repeats.
- `GET /api/v1/servers/{id}/commands/{commandId}` (`get_server_command`) and
  `POST /api/v1/servers/{id}/commands/{commandId}/cancel` (`cancel_server_command`): one receipt,
  and cancellation while the command is still queued (409 `COMMAND_NOT_CANCELLABLE`, with its
  state, otherwise).
- `GET /api/v1/servers/{id}/deployments` (`list_server_deployments` in
  `apps/api/src/missions/handlers/mission_deployments.rs`): the server's deployments,
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
  `apps/api/src/server_infrastructure/handlers/fleet_scenarios.rs`): the registry of
  fleet scenarios; the API checks the same patterns and records `fleet.scenario_registered` or
  `fleet.scenario_removed`.
- `GET /api/v1/servers/{id}/credentials` (`list_server_credentials` in
  `apps/api/src/server_infrastructure/handlers/machine_credentials.rs`): the
  server's credentials without secrets. `POST` on the same path issues one, for `host_agent` or
  `mod_runtime`, and its answer is the only one that carries the secret.
  `DELETE /api/v1/servers/{id}/credentials/{credentialId}?reason=<text>`
  (`revoke_server_credential`) revokes one credential, keeping the reason in the audit trail and
  ending the game-runtime sessions it authenticated.
- The API has no [RCON](/documentation/glossary/n_to_z.md#rcon) route. RCON is the host agent's
  business: it lists players over RCON when it carries out `list_players`, and sends a console
  line, once, when it carries out `console_command`.

## Design

- A full-bleed `SplitPane` over the topographic backdrop: a 17rem picker and the server card as
  the detail. The registration, fleet scenario and credential sheets slide in from the side. The
  layout as built (the address is a placeholder):

```text
+-----------------+-----------------------------------------------------------+
| SERVERS 3       | TBD Primary — Everon   [Edit] [Credentials] [LAUNCH ...]  |
| [Fleet scen.]   | <ip>:<port>  <server id>                                  |
| ● Primary       |-----------------------------------------------------------|
| ○ Secondary [I] | PERSONNEL 47/64 | TERRAIN Everon | FPS 58.7 | QUEUE 3/512 |
| ○ Staging   [I] |-----------------------------------------------------------|
| [+ Add server]  | FLEET COMMANDS              | MISSION DEPLOYMENTS         |
|                 | [Start][Stop][Restart]      | [Request a deployment]      |
|                 | [List players]              |  mission v  event mission v |
|                 | Console [ #players ] [Send] |  [Deploy]                   |
|                 | Broadcast [ message ] [>]   |                             |
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

REGISTRATION SHEET (side sheet: "Add a server", or "Server settings" from Edit)
| Name [TBD Staging — Everon]                     |
| Address [203.0.113.24]          Game port [2001] |
| Required modpack [No required modpack v]        |
|                       [Register server | Save]  |
| Service: Active …                  [Deactivate] |
```

- No visual reference set exists for this page, and the archived platform spec has no section
  for it; the built layout is the reference.

## Open work

- [T-086 — Server Control + RCON API](/.ai/tickets/T-086.toml) (deferred, no plan): a live server
  control panel wired to an RCON backend. The page's console box sends one line at a time as a
  `console_command` fleet command and shows the reply the host agent reports; it streams no live
  console and calls no RCON route.

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
- A server leaves the fleet only by deactivation, which the page offers with its consequences
  stated and undoes with "Reactivate": the API keeps every server row, because its statuses,
  credentials, commands and deployments refer to it.
- The registration form refuses a hostname rather than resolving it: the API stores a literal
  address, and a name resolved in the browser could differ from the one players resolve.
- Process control, the player list, console lines, broadcasts and kicks are fleet commands and
  loading a mission is a deployment. The console box reaches RCON only through a `console_command`
  the host agent carries out, so every line is authorised, recorded, audited and followed to its
  outcome like any other command, and the page calls no RCON route.
