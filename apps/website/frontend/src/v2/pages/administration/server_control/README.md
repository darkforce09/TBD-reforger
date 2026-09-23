# Server Control (`/admin/server`)

The configured game servers, the state of the one selected, the fleet commands and mission
deployments that act on it, the fleet's scenario registry, and the machine credentials its host
agent and game runtime authenticate with. Nothing here reaches a host directly: a command or a
deployment is a request the backend records and answers with 202, and the screen follows it to the
outcome an executor or a runtime session reports.

## Architecture
- **`page.rs`**: route component — fetches `GET /servers`, puts it behind the administrator gate,
  owns the selected server and the fleet scenario registry, and arranges the picker (with the
  "Fleet scenarios" control) beside the server card.
- **`server_cards.rs`**: the selectable list of servers with their status dots, the selected
  server's header (credential sheet and launch controls), the three telemetry columns — terrain
  read off the server row — and the fleet command and deployment sections. Each card builds the
  command console, deployments panel and credential sheet of the server it shows.
- **`fleet_commands/`**: the fleet command console. `mod.rs` holds its state and the four
  operations: read `GET /servers/:id/commands`, request (`POST …/commands`, answered 202 with the
  receipt), follow (`GET …/commands/:commandId` every two seconds until a terminal state) and
  cancel (`POST …/commands/:commandId/cancel`, only while queued — `409 COMMAND_NOT_CANCELLABLE`
  names the state otherwise). `command_requests.rs` offers start, stop and restart (the last two
  confirmed first), list players, a broadcast message, and a kick naming the Arma identity and the
  runtime session it is issued against (players offered from the newest successful listing, the
  session from the newest confirmed deployment). `command_history.rs` shows the followed command
  and every receipt — state, action, executor, requester and time, claim count, arguments, outcome
  and failure reason. `command_wording.rs` words all of it: only `succeeded` is announced as
  success, and `indeterminate` is announced as an unknown outcome that nothing repeats.
- **`mission_deployments/`**: the deployments panel. `mod.rs` holds its state and operations: read
  `GET /servers/:id/deployments`, read the request form's choices when it first opens, request
  (`POST …/deployments`, answered 202), follow (`GET …/deployments/:deploymentId` until confirmed,
  failed or cancelled) and cancel (`POST …/deployments/:deploymentId/cancel`).
  `deployment_request.rs` offers the live missions whose latest approval names an artifact (the
  library's `approved_artifact_id`) and, optionally, an event mission of an operation scheduled on
  this server. `deployment_list.rs` shows each deployment and, opened, its detail: mission, artifact
  digest, document SHA-256, terrain, scenario, transition, requested via/by/at, deadline, state,
  bound seats, fleet command and its state, confirming runtime session, finish and failure reason.
  `deployment_refusal.rs` words every refusal code, listing each unbound seat and unseated slot of
  an `ORBAT_ARTIFACT_MISMATCH`.
- **`fleet_scenarios/`**: the fleet scenario sheet — every terrain's registered scenario header
  (`GET /fleet/scenarios`), the form that registers or replaces one
  (`PUT /fleet/scenarios/:terrainKey`, checked first against the contract's patterns and the
  backend's display-name rule), and removal (`DELETE …`, confirmed first).
- **`machine_credentials/`**: the selected server's credential sheet (administrators only).
  `mod.rs` holds its state — the list of `GET /servers/:id/credentials`, the in-flight flag, and
  the one secret an issue answered with — and the issue (`POST`) and revoke
  (`DELETE …/credentials/:credentialId?reason=`) requests; `credential_sheet.rs` renders the secret
  once, with a copy control and the warning that it cannot be shown again (closing the sheet
  discards it), the issue form (a label and the program: game runtime or host agent), and every
  credential with its label, program, who issued it and when, its last use, and — once revoked —
  who revoked it, when and why, the revoke control asking for the reason first;
  `credential_text.rs` words all of that and checks a label and a reason as the backend does.
- **`tests/server_control.rs`**: the server list read, the card's readings, and the state each card
  builds; each subfolder carries its own tests beside it.

## Not present in the legacy page
- **An RCON console**: the backend has no RCON route. Process control, the player list, broadcasts
  and kicks are fleet commands, and loading a mission is a deployment.
- **The running mission's name on the server card**: the server payload carries the current
  match id, not a mission, so the card shows that id.
