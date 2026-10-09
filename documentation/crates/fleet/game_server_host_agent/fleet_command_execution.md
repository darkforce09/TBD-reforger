**Status:** live

# Fleet command execution on the game host

How an operator's server command reaches a self-hosted game host and becomes a process action, an
[RCON](/documentation/glossary/n_to_z.md#rcon) read, a console line sent once over RCON, or a
[mission header](/documentation/glossary/g_to_m.md#mission-header) switch, without the
[API](/documentation/glossary/a_to_f.md#api) ever connecting to the host. The
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) is the host's half; the API's
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) ledger is the other. Operators, and
developers changing either half, read this for the design and its limits.

## Where it lives

- Code: [`crates/fleet/game_server_host_agent/`](/crates/fleet/game_server_host_agent/README.md), with the ledger client,
  command execution, process control, server config and RCON modules under
  [`src/`](/crates/fleet/game_server_host_agent/src/README.md).
- Entry: `game_server_host_agent <configuration-file>`, run once per fleet instance as the
  systemd user unit `game_server_host_agent@N.service`, an instance of
  `deploy/systemd/game_server_host_agent@.service` that reads
  `~/.config/game_server_host_agent/instance-N/agent.toml`. `cargo xtask deploy staging` installs the
  template and runs an agent for every instance, each with its own server's `host_agent`
  credential; the relay instance's agent (instance 5 on staging) polls the API through the
  acknowledgement-dropping relay on loopback.
- Related features: the API's [fleet command ledger](/documentation/crates/api/api_server/verification_evidence/fleet_command_ledger.md)
  and [machine credentials](/documentation/crates/api/api_server/verification_evidence/machine_credentials.md);
  the [server control page](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md),
  where operators issue commands; the [game runtime](/documentation/glossary/g_to_m.md#game-runtime)'s own executor in
  [`mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/`](/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/README.md);
  the [game server staging runbook](/documentation/runbooks/game_server_staging/README.md).

## Behaviour

### Who runs what

```text
operator ──▶ POST /api/v1/servers/{id}/commands ──▶ ledger row (queued)
                                                        │ claimed by the executor its action names
                 ┌──────────────────────────────────────┴──────────────────────────┐
     game server host agent (host_agent credential)            game runtime (mod_runtime credential)
     start, stop, restart, list_players,                       broadcast, kick, load_mission
     restart_with_mission, console_command
```

Each action has one executor. The agent refuses `broadcast`, `kick` and `load_mission` as game
runtime actions, and anything else as unsupported, so a command claimed by the wrong party can
never act.

### One command, start to finish

1. The agent claims with `POST /api/v1/fleet-executor/commands/claim`; a 204 means nothing is
   queued and it claims again after `poll_interval_seconds`.
2. It validates the action and arguments again by the API's own rules, so no text from the API
   widens what the host runs; a command that fails is reported failed and nothing runs.
3. It reports `executing` and waits for the API to acknowledge. A stale fencing token or any
   refusal abandons the command untouched.
4. It performs the action: a `systemctl --user` start, stop or restart judged by the unit state
   read back after a dwell; `#players` over RCON; the operator's console line over RCON,
   transmitted once; or, for a cross-terrain
   [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment), a surgical switch of
   `game.scenarioId` in the server config followed by a restart.
5. It reports the result, retrying the report, never the effect, until the API acknowledges or
   refuses it. An outcome that is never reported leaves the ledger to mark the command
   indeterminate.

The loop, its retry and backoff numbers and the success rule of each action are in the
[crate README](/crates/fleet/game_server_host_agent/README.md#how-it-works) and the
[command execution README](/crates/fleet/game_server_host_agent/src/command_execution/README.md#how-it-works).

### Safety model

The rules that keep a compromised API or a malformed command from widening what the host does
are listed in the crate README's [Safety model](/crates/fleet/game_server_host_agent/README.md#safety-model): no
shell, fixed argument vectors with a validated unit name and a cleared environment, free text
reaching the host only as a console line bound for the RCON port, an atomic and surgical config
change, secrets in owner-only files that go only to their own protocol, a credential that follows
no redirect, and fencing on every report.

### RCON

The agent speaks BattlEye RCon over UDP to the loopback `rcon` block of the server config, one
command at a time, with login, retransmission, multi-part responses, server-message
acknowledgement and a keep-alive below the server's timeout; the
[RCON README](/crates/fleet/game_server_host_agent/src/rcon/README.md#how-it-works) gives the wire format and
the timings. The agent's reads, `#players` and the keep-alive, are delivered at least once:
retransmitted while unanswered and sent again after a new login, which is safe only because they
change nothing.

A `console_command` carries one line an operator typed for the game server's console, which may
change the server, so the line is transmitted exactly once. The login may be retried and a held
login is confirmed with an empty command packet first, both safe to repeat; the line itself
leaves in one packet and is never sent again, not even after a new login. The line is 1 to 256
bytes, not blank, without control characters or line and paragraph separators, and never starts
with `@`, the prefix of the custom RCON commands such as `@logout`. The reply becomes the outcome
`response`, cut on a character boundary to at most 4096 bytes with `response_truncated` saying
whether it was cut. A line that gets no reply fails with "no RCON response; the command may or
may not have run"; the
[command execution README](/crates/fleet/game_server_host_agent/src/command_execution/README.md#how-it-works)
has the full rules. Reforger's RCON has no broadcast command, which is one reason broadcasts run
in the game runtime.

### Known discrepancies

- The agent counts the RCON password's three-character minimum in characters but says bytes in
  its error (`crates/fleet/game_server_host_agent/src/agent_configuration/secret_files.rs:67-79`). Only a
  password written by hand can meet the difference: `cargo xtask deploy staging` generates each
  fleet instance's password on the host as 32 lowercase hex digits and refuses an existing file of
  any other shape (`RCON_PASSWORD_SHAPE` in
  `tools/commands/deployment/src/staging/payloads.rs`).

## Data

- `POST /api/v1/fleet-executor/commands/claim`, `…/{commandId}/executing` and
  `…/{commandId}/result` (the fleet executor handlers in
  `crates/api/api_server_infrastructure/src/`): claim the next command for the credential's
  server with a lease and a fencing token, move it to `executing` inside its execution window, and
  record its outcome; a stale token is a 409 `STALE_FENCING_TOKEN`. The wire shapes are
  `contracts/definitions/fleet-command.schema.json`.
- The agent's TOML configuration under `~/.config/game_server_host_agent/` and the two secret
  files it names under `~/tbd/fleet/instance-N/secrets/`, whose keys and rules are in the crate
  README's
  [Configuration](/crates/fleet/game_server_host_agent/README.md#configuration).
- The dedicated server's JSON config, of which the agent changes only `game.scenarioId`, and its
  `rcon` block.

## Design

- The API never connects to a host: a host behind NAT or a firewall needs only outbound HTTPS,
  and no inbound port on the game host is opened for control.
- The agent performs a closed set of actions, each with its own argument grammar, and validates
  every command again; the API's validation is not trusted to be the only gate.
- Effects happen at most once: nothing runs before an acknowledged `executing` report, the report
  is retried and the effect never is, and a lost outcome becomes indeterminate for an operator to
  judge.
- The console line is the only free text that reaches the host: it reaches only the RCON port,
  as one line that cannot start a custom RCON command, in a single transmission.
- The agent never fetches a mission [artifact](/documentation/glossary/a_to_f.md#artifact): for a cross-terrain deployment it only points the
  server config at the new mission header, and the game runtime loads, verifies and reports the
  artifact when it boots, which confirms the deployment.

## Open work

- [T-940.11 — RCON kick and change-map through the host agent](/documentation/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation/tickets/plans/t-940_11_plan.md)): the agent gains kick and
  change-map actions with a strict argument grammar. Its plan names `admin.rs` and
  `services/game_agent.rs`, which the API does not have, and `kick` runs in the game runtime
  today, so the plan needs checking against the code before work starts.
- [T-1146 — Decide one RCON password rule for staging and fleet agent](/.ai/tickets/T-1146.toml)
  (idea, no plan): the deploy and the agent apply one password rule.
- [T-1137 — Gate ticket_engine, verification_core, ticketboard and fleet agent tests and clippy](/.ai/tickets/T-1137.toml)
  (idea, no plan): CI runs the agent's tests and clippy.
- [T-086 — Server Control + RCON API](/.ai/tickets/T-086.toml) (deferred, no plan): a live server
  control panel wired to an RCON backend; a console line runs today as a `console_command` fleet
  command, one line per command.

## Decisions

- Outbound polling instead of an inbound control port: hosts need no exposed service, and the API
  stays the single place commands are recorded.
- One executor per action: the host agent owns process control, RCON reads and the config switch;
  the game runtime owns in-process actions. Neither can perform the other's commands.
- RCON reads are retransmitted, and the console line is sent once: at-least-once delivery suits
  only a command that survives a retransmission unharmed, so a line that may change the server
  goes out a single time, and a missing reply is reported as possibly run instead of retried.
- A restart is judged by the unit state read back, not by `systemctl`'s exit status: the exit
  status says the request was queued, not that the server runs.
