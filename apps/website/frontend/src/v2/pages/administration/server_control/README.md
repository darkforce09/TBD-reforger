# Server control page

The `/admin/server` page, [server control](/documentation_v2/glossary/n_to_z.md#server-control):
administrators pick one of the configured game servers, read its live state, issue
[fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command) to it, deploy a
[mission](/documentation_v2/glossary/g_to_m.md#mission)'s approved
[artifact](/documentation_v2/glossary/a_to_f.md#artifact) to it, keep the
[fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario) registry, and issue and revoke its
[machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential).

## Contents

```text
apps/website/frontend/src/v2/pages/administration/server_control/
├── fleet_commands/       the card's fleet command console: request, follow, history and cancel
├── fleet_scenarios/      the fleet scenario sheet: each terrain's registered mission header
├── machine_credentials/  the card's credential sheet: issue with a one-time secret, list and revoke
├── mission_deployments/  the card's deployments panel: request, follow, list, detail and cancel
├── mod.rs                the module tree; re-exports `ServerControlPage`
├── page.rs               `ServerControlPage`: the gate, the server list, the picker and the sheet
├── server_cards.rs       the picker's rows and the selected server's card with its readings
└── tests/                unit tests for the typed server read, the card's readings and its state
```

## How it works

`ServerControlPage` renders `ServerControlInner` inside `AdminGate`. The inner component fetches
the server list once and builds the one `ScenarioRegistry`, which belongs to the whole fleet, so
the picker's heading opens its sheet. The picker opens on the first active server, else the first
one (`pick_default_id`). `server_detail` builds the card of the selected server and, with it, that
server's `CommandConsole`, `DeploymentPanel` and `CredentialPanel`, so switching servers never
shows one server's commands, deployments, credentials or fresh secret under another's name.

Nothing here reaches a host directly. A command or a deployment is a request the
[API](/documentation_v2/glossary/a_to_f.md#api) records and answers with 202; the page then reads its
receipt every two seconds and announces the outcome only once an executor or a runtime session
reports it. The card shows only what the server row carries: a server with no status reads as
zeros and dashes, the terrain is capitalised or a dash between matches, and "Active Mission" shows
the current match id, since the row names no mission. The launch control only says the game client
is needed. The page has no [RCON](/documentation_v2/glossary/n_to_z.md#rcon) console, and no control adds,
edits or deactivates a server. Every request runs in the browser build only; a native build
renders the failure branch.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/server` | `ServerControlPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` [role](/documentation_v2/glossary/n_to_z.md#role) only | full-bleed inside the navigation frame, over the topographic backdrop; breadcrumb Administration / Server Control; sidebar entry "Server Control" |

## Data

- `GET /api/v1/servers`: read as `DataEnvelope<ServerRowDto>`; the picker reads `id`, `name`,
  `is_active` and `status.is_online`, and the card `ip`, `port`, `terrain`, `required_modpack`
  (name and version) and `status` (`player_count`, `max_players`, `uptime_seconds`, `server_fps`,
  `current_match_id`).
- Fleet commands (`fleet_commands/`):
  - `GET /api/v1/servers/{id}/commands`: read as `FleetCommandList` of `FleetCommandReceipt`.
  - `POST /api/v1/servers/{id}/commands`: sends a `FleetCommandRequest` (`action`, `arguments`),
    read back as the `FleetCommandReceipt`.
  - `GET /api/v1/servers/{id}/commands/{commandId}`: the followed receipt, every two seconds.
  - `POST /api/v1/servers/{id}/commands/{commandId}/cancel` with `{}`: read as the receipt.
- [Mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment) (`mission_deployments/`):
  - `GET /api/v1/servers/{id}/deployments`: read as `MissionDeploymentPage` of `MissionDeployment`.
  - `POST /api/v1/servers/{id}/deployments`: sends a `DeploymentRequest` (`mission_id`,
    `artifact_id`, optional `event_mission_id`), read back as the `MissionDeployment`.
  - `GET /api/v1/servers/{id}/deployments/{deploymentId}`: the followed deployment, every two
    seconds.
  - `POST /api/v1/servers/{id}/deployments/{deploymentId}/cancel` with `{}`.
  - The form's choices, read when it first opens: `GET /api/v1/missions?limit=100`
    (`Paginated<MissionCard>`), `GET /api/v1/events?scope=upcoming&limit=100`
    (`Paginated<EventListItem>`), and `GET /api/v1/events/{id}` (`EventHub`) for each
    [event](/documentation_v2/glossary/a_to_f.md#event) whose `server_id` is this server.
- Fleet scenarios (`fleet_scenarios/`): `GET /api/v1/fleet/scenarios`, read as
  `FleetScenarioList`; `PUT /api/v1/fleet/scenarios/{terrainKey}`, sending a `FleetScenarioUpdate`
  (`scenario_id`, `display_name`); `DELETE` on the same path.
- Machine credentials (`machine_credentials/`): `GET /api/v1/servers/{id}/credentials`, read as
  `MachineCredentialList`; `POST` on the same path, sending a `MachineCredentialIssue` (`label`,
  `executor_kind`), read as `IssuedMachineCredential`, the one answer that carries the secret;
  `DELETE /api/v1/servers/{id}/credentials/{credentialId}?reason=<text>`, read as the revoked
  `MachineCredential`.
- The page reads the `AuthStore` context and the toast queue; "Copy" writes an issued secret to the
  clipboard, and nothing is stored in the browser.

## States

### Servers and the card

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| loading | "Loading servers…" |
| failed | "Failed to load servers." |
| picker | "Servers" with their count, "Fleet scenarios" (titled "Which scenario the fleet runs for each terrain"), and per server its name and "Online" with a pulsing dot, or "Offline" |
| no servers | "No servers configured." |
| none selected | "No server selected." |
| card | the name and `ip:port`, "Credentials" and "LAUNCH & CONNECT" (which toasts "Launch requires the Reforger client"); "Active Personnel" (players / max), "Uptime" ("Nd HHh MMm", the day part dropped under a day), "Terrain", "Active Mission" (the match id), "Server FPS" ("x.y Hz") and "Mod Configuration" ("name vX"); "—" and zeros without a status |

### Fleet commands

| State | What the viewer sees |
|---|---|
| controls | "Fleet commands": "Start", "Stop", "Restart", "List players"; "Message, at most 256 bytes" and "Broadcast"; "Arma identity (UID)", "Runtime session id — the server's open session", "Reason shown to the player (optional)" and "Kick", with the listed players offered and "Use the session that confirmed the latest deployment (<id>)" |
| stop or restart asked | "Stop <server>? Every connected player is disconnected." or "Restart <server>? Every connected player is disconnected until it is back.", with "Stop the server" or "Restart the server", and "Keep it running" |
| accepted | the toast "<Action> accepted — waiting for the <executor> to carry it out", and "Your command": "<Action> is <state> — following it until it finishes." |
| ended | "<Action> succeeded" (with what it observed, such as "N player(s): …" or "No players connected"), "<Action> failed: <reason>", "<Action> expired — no executor carried it out before <time>, so nothing ran", "<Action> was cancelled before any executor claimed it — nothing ran", or "<Action>: the outcome is unknown. The executor stopped reporting after the command started, so it may or may not have taken effect, and nothing repeats it. Inspect the server before issuing it again." |
| follow abandoned | "Stopped following the command: <why>" |
| history | "History" and "Refresh": "Loading the command history…", the failure ("The command history could not be read"), "No command has been requested for this server yet.", or per command its state ("Queued", "Claimed", "Executing", "Succeeded", "Failed", "Expired", "Cancelled", "Outcome unknown"), action, "<executor> · N claim(s)", "Requested by <who>, <time>", claimed, executing, finished or expires times, "Arguments — …", its outcome and failure reason; "Cancel" on a queued one, answered "<Action> cancelled — nothing ran" |
| refused | the API's sentence, else "The command could not be requested" or "The command could not be cancelled"; an unclaimable cancel reads "The command is <state> — an executor has taken it up, so it can no longer be cancelled.", an ended session "That runtime session is not the server's open session any more — the game server has restarted since. Issue the kick against the session running now." |
| request checks | "Enter the <field>", "The <field> is too long: at most <n> bytes", "The <field> cannot hold line breaks or control characters", "Enter the runtime session the kick is issued against, as its id" |

### Mission deployments

| State | What the viewer sees |
|---|---|
| form | "Mission deployments": "Request a deployment"; opened, "Reading the live missions and this server's operations…", then "Choose a mission…", "None — deploy without binding seats", "Deploy" and "Close the form"; "No live mission has an approved artifact to deploy. A mission becomes deployable once a reviewer approves its artifact." when none qualifies |
| accepted | the toast "Deployment of <mission> recorded — waiting for a runtime session to confirm it", and "Your deployment": "<mission> is recorded — its fleet command is <state>; waiting for a runtime session to confirm the artifact by <deadline>." |
| ended | "<mission> is confirmed — a runtime session reported artifact <digest>", "The deployment of <mission> failed: <reason>", or "The deployment of <mission> was cancelled before its command was claimed" |
| refused | one sentence per code: `ARTIFACT_NOT_APPROVED` "Only the artifact a live mission's latest approval decided can be deployed. …", `MODPACK_MISMATCH` "The artifact was compiled against modpack <id> but this server requires modpack <id>. …", `TERRAIN_NOT_RUNNABLE` "No fleet scenario is registered for terrain <key>. …", `EVENT_MISSION_NOT_ON_SERVER` "The event mission must run this mission on an operation scheduled on this server. …", `ORBAT_ARTIFACT_MISMATCH` with "Seats with no compiled slot" and "Compiled slots no seat stands for", `DEPLOYMENT_IN_PROGRESS` "Another deployment of this server is in flight. …", `SERVER_INACTIVE` "This server is deactivated and accepts no deployment."; any other code, the API's sentence |
| list | "Deployments" and "Refresh": "Loading the deployments…", the failure ("The deployments could not be read"), "Nothing has been deployed to this server yet.", or per deployment its state ("In flight", "Confirmed", "Failed" or "Cancelled"), its mission and "<terrain> · artifact <digest> · requested <time>" |
| detail | Mission, State, Artifact digest, Document SHA-256, Terrain, Scenario, Transition ("Scenario restart — …" or "Host restart — …"), Requested ("by <who> from the website, <time>", or "from in game"), Deadline, Bound seats, Fleet command, Event mission, Confirmed by runtime session, Finished and Failure reason; "Cancel this deployment" while in flight, answered "The deployment of <mission> is cancelled — its command never ran" |

### Fleet scenarios sheet

| State | What the viewer sees |
|---|---|
| sheet | "Fleet scenarios" and "A deployment runs its artifact on the scenario registered for the artifact's terrain; a terrain with none cannot be deployed." |
| form | "Register or replace a terrain's scenario": "Terrain key, for example arland", "Display name", "{1111222233334444}Missions/TBD_Arland.conf" and "Save scenario"; a failed check shows its reason |
| list | "Loading the fleet scenarios…", the failure ("The fleet scenarios could not be read"), "No terrain has a registered scenario yet.", or per terrain its name, scenario id and "Updated by <who>, <time>" ("you" for the viewer), with "Edit" and "Remove" |
| removal asked | "New deployments of this terrain are refused until a scenario is registered again; deployments already recorded are untouched.", "Remove the scenario" and "Keep" |
| toasts | "Terrain <key> runs <name>", "Terrain <key> is no longer offered to new deployments"; a refusal shows the API's sentence, else "The scenario could not be registered" or "The scenario could not be removed" |

### Machine credentials sheet

| State | What the viewer sees |
|---|---|
| list | "Machine credentials": "Loading credentials…", the failure ("Could not load the credentials"), "This server has no credentials yet.", or per credential its program ("Host agent" or "Game runtime"), label, "Issued by <who>, <time>", and "Live · last used <time>", "Live · never used" or "Revoked by <who>, <time>: <reason>" |
| issue form | "Issue a credential": "Label, for example the host it runs on", the program "Game runtime — sessions, heartbeats, roster reads, broadcasts, kicks and deployments" or "Host agent — process control, the RCON player list and cross-terrain restarts", and "Issue credential" |
| secret shown | "Store this secret now — it cannot be shown again", "It authenticates the <program> credential "<label>" of this server. Closing this sheet discards it.", "Copy" (toast "Secret copied") and "I have stored it — hide the secret" |
| revoke asked | "Revoke" ("Keep"), "Revoking stops this credential at once and ends every game-runtime session it authenticated. The server's other credentials are untouched.", "Why it is revoked (kept in the audit trail)" and "Revoke credential" |
| toasts | "Issued <label>", "Revoked <label>"; a refusal shows the API's sentence, else "Could not issue the credential" or "Could not revoke the credential"; a failed check shows its reason |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `DataEnvelope`, `ServerRowDto`, `ModpackDto` and
  the command, deployment, scenario and credential DTOs and endpoint modules),
  `crate::v2::core::auth` (`AuthStore`), `crate::v2::core::ui` (`AdminGate`, `SplitPane`, `Sheet`,
  `MaterialIcon`, `cn`, the toast queue) and `crate::v2::core::utils` (`utc_timestamp`,
  `clipboard`); over HTTP, the server, command, scenario and credential routes of the
  [server infrastructure](/documentation_v2/glossary/n_to_z.md#server-infrastructure) domain, the
  deployment routes and mission library of the [missions](/documentation_v2/glossary/g_to_m.md#missions)
  domain, and the event reads of the [operations](/documentation_v2/glossary/n_to_z.md#operations) domain.
- Used by: the `/admin/server` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Server Control" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `server_control_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the page's sources for its
  tests; the DOM oracle's `servercontrol` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the page never calls an RCON route (`no_rcon_route_is_called_or_served`) and shows no
  invented server or console (`no_mock_servers_or_fabricated_console`); every card builds the
  state of its own server (`every_card_builds_the_state_of_its_own_server`); the default pick
  prefers an active server (`pick_default_prefers_active`), all in `tests/server_control.rs`;
  each panel's own rules are in its folder's README.

## Related documentation

- [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — the
  server, command, scenario and credential routes.
- [Missions domain](/apps/website/api_v2/src/missions/README.md) — the deployment routes.
