**Status:** live

# Fleet command ledger

Design for the fleet requirements (`fleet_fleet_commands`, `fleet_process_control`,
`fleet_rcon_transport`, `fleet_kick_identity`, `fleet_command_receipts`,
`fleet_command_recovery`, `fleet_console_command`). It records the chosen semantics before
implementation; acceptance evidence is the command output recorded in progress_checkpoint.md.

## Why a ledger

Five staging servers run on more than one host, so the API cannot reach a host by a local socket
and a dead game server polls nothing. Each host runs a host agent and each game runtime runs the
mod; both poll the API outbound over HTTPS with their own `host_agent` or `mod_runtime` machine
credential (machine_credentials.md). The API never pushes to a host and never runs a shell.

## Commands

`fleet_commands` holds one row per accepted operator command: server, executor kind, action,
validated arguments, requester, request time, expiry, state, the claim (credential, fencing token,
lease), timestamps of each transition, and the outcome or failure reason.

| Action | Executor | Idempotent | Arguments |
|:---|:---|:---|:---|
| `start`, `stop`, `restart` | host agent | `start`/`stop` yes, `restart` no | none |
| `list_players` | host agent (RCON) | yes | none |
| `console_command` | host agent (RCON) | no | `line` (1 to 256 bytes, one line, no control characters, no leading `@`) |
| `broadcast` | mod runtime | no | `message` (1 to 256 bytes, no control characters) |
| `kick` | mod runtime | no | `arma_id`, `runtime_session_id`, optional `reason` |
| `load_mission` (same-terrain deployment) | mod runtime | no | `deployment_id`, `artifact_id`, `artifact_sha256`, `runtime_session_id` |
| `restart_with_mission` (cross-terrain deployment) | host agent | no | `deployment_id`, `artifact_id`, `artifact_sha256`, `scenario_id` |

Reforger's RCON has no broadcast command, so a broadcast runs in the game runtime. A command that
names a `runtime_session_id` (a kick, and a scenario restart, whose terrain was validated against
that session) fails when a game runtime claims from any other session; commands that name none
are claimed by whichever session of the server asks. `load_mission`
and `restart_with_mission` are issued only by mission deployments (mission_artifacts.md); the
operator command route refuses them. No argument reaches a shell: arguments are typed,
validated on the API and re-validated by the executor, and the host agent maps each action to a
fixed program invocation or RCON packet. The one free-text argument, a console command's line,
goes only to the server's RCON console.

A console command carries one administrator line for the dedicated server's RCON console. A line
starting with `@` is refused, because `@` begins Reforger's custom RCON commands for the RCON
session itself (`@logout`), and that session is the host agent's. The host agent transmits the
line at most once: it retries only the login, never the line, so a reconnect cannot send it
twice. It reports `{response, response_truncated}`, the server's reply cut on a character
boundary at 4096 bytes, and the API refuses a longer response with 400; when the server never
answers, the command fails with "no RCON response; the command may or may not have run". A
console line can stop or restart the server or its scenario, so the command is process-changing
and not idempotent, with a 30 s execution window. Its request audit row carries the line.

## States

`queued` → `claimed` → `executing` → `succeeded` | `failed`; `queued` → `cancelled` (operator) or
`expired` (not claimed before `expires_at`); a claim whose lease lapses before `executing`
returns to `queued` with a new fencing token; a lease that lapses during `executing` makes an
idempotent command `queued` again and a non-idempotent one `indeterminate`, which no executor
retries — an operator inspects the server and issues a new command. `succeeded` is written only
from an executor report carrying the current fencing token; a stale executor's report answers
409 and changes nothing.

## Rules

- Intent is persisted before any effect: `POST /servers/{id}/commands` (administrator) writes the
  row and answers 202 with the receipt; the executor reports `executing` before it acts.
- Claim revalidates the requester's authority on the claim transaction (an administrator
  demoted, banned or deleted since the request makes the command `cancelled` with that reason) and
  serializes incompatible commands: at most one process-changing command (`start`, `stop`,
  `restart`, `load_mission`, `restart_with_mission`, `console_command`) per server is claimed or
  executing; `list_players`, `broadcast` and `kick` do not wait for them.
- A kick runs in the game runtime, not over RCON. It names the Arma identity and the runtime
  session it was issued against, and a claim from any other session fails it; the runtime tells
  the player the reason, then kicks whoever holds the identity at that moment and fails when that
  player has disconnected, so a reconnect or a recycled player number cannot target another
  player.
- Every transition is audited; operator actions carry the operator, executor transitions the
  credential.
- A reconciliation worker expires, re-queues and marks indeterminate by lease and expiry times.

## Executors

- The host agent is a Rust crate under `apps/fleet_host_agent/`: it polls claims, runs fixed
  process actions without a shell (`systemctl --user` with a fixed argument vector, judged by the
  unit's state read back after a dwell), and speaks the BattlEye RCon protocol over UDP (CRC32
  checked login, command sequence numbers with reuse after 255, multi-packet responses, keep-alive
  and reconnection) for the player list and console commands. `restart_with_mission` rewrites
  only `game.scenarioId` in the dedicated server's configuration, atomically and byte for byte
  elsewhere, then restarts the unit. The RCON password and the machine credential stay in
  owner-only files on the host.
- The mod runtime claims its commands within its runtime session and reports outcomes through the
  same fenced routes: `API/FleetCommands/TBD_FleetCommandPoller.c` claims every 5 s,
  `TBD_FleetCommandExecution.c` re-validates the arguments (`TBD_FleetCommandArguments.c`) and
  reports `executing` before any effect, and `TBD_FleetPlayerActions.c` (`broadcast`, `kick`) and
  `TBD_FleetLoadMissionAction.c` (`load_mission`) run the effects.
- `cargo xtask deploy staging` installs one host agent per fleet instance: it builds the crate on
  the host, renders the loopback `rcon` block into each instance's server configuration, writes
  each `~/.config/fleet_host_agent/instance-N/agent.toml` naming that instance's owner-only secret
  files under `~/tbd/fleet/instance-N/secrets/`, installs
  `deploy/systemd/fleet_host_agent@.service` and fails the deploy unless every
  `fleet_host_agent@N` is active. The relay instance's agent (`TBD_FLEET_RELAY_INSTANCE`,
  instance 5 on staging) polls the API through the acknowledgement-dropping relay on loopback.
- Server Control (`pages/administration/server_control/fleet_commands/`) issues operator
  commands and follows each one's receipts to its final state; `cargo xtask mod playtest` cancels
  the transition command of its own deployment when no executor claimed it.

## Tests

Ledger transitions, fencing, serialization, authority revalidation, crash reconciliation, kick
binding and the console command's line gate, response bound and lapse are exercised against
PostgreSQL through the real routes; the RCON transport is exercised
against a protocol test server that injects loss, duplication and reordering.
