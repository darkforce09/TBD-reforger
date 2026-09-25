**Status:** live

# Fleet command execution on the game host

How an operator's server command reaches a self-hosted game host and becomes a process action, an
[RCON](/documentation_v2/glossary.md#rcon) read or a
[mission header](/documentation_v2/glossary.md#mission-header) switch, without the
[API](/documentation_v2/glossary.md#api) ever connecting to the host. The
[fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) is the host's half; the API's
[fleet command](/documentation_v2/glossary.md#fleet-command) ledger is the other. Operators, and
developers changing either half, read this for the design and its limits.

## Where it lives

- Code: [`apps/fleet_host_agent/`](/apps/fleet_host_agent/README.md), with the ledger client,
  command execution, process control, server config and RCON modules under
  [`src/`](/apps/fleet_host_agent/src/README.md).
- Entry: `fleet-host-agent <configuration-file>`, run as the systemd user unit
  `tools_v2/xtask/deploy/systemd/fleet-host-agent.service`, which `cargo xtask deploy staging`
  installs when `TBD_INSTALL_HOST_AGENT=1`.
- Related features: the API's [fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  and [machine credentials](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md);
  the [server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md),
  where operators issue commands; the [game runtime](/documentation_v2/glossary.md#game-runtime)'s own executor in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/`](/apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/README.md);
  the [game server staging runbook](/documentation_v2/runbooks/game_server_staging/README.md).

## Behaviour

### Who runs what

```text
operator ──▶ POST /api/v1/servers/{id}/commands ──▶ ledger row (queued)
                                                        │ claimed by the executor its action names
                 ┌──────────────────────────────────────┴──────────────────────────┐
     fleet host agent (host_agent credential)                  game runtime (mod_runtime credential)
     start, stop, restart, list_players,                       broadcast, kick, load_mission
     restart_with_mission
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
   read back after a dwell; `#players` over RCON; or, for a cross-terrain
   [mission deployment](/documentation_v2/glossary.md#mission-deployment), a surgical switch of
   `game.scenarioId` in the server config followed by a restart.
5. It reports the result, retrying the report, never the effect, until the API acknowledges or
   refuses it. An outcome that is never reported leaves the ledger to mark the command
   indeterminate.

The loop, its retry and backoff numbers and the success rule of each action are in the
[crate README](/apps/fleet_host_agent/README.md#how-it-works) and the
[command execution README](/apps/fleet_host_agent/src/command_execution/README.md#how-it-works).

### Safety model

The rules that keep a compromised API or a malformed command from widening what the host does
are listed in the crate README's [Safety model](/apps/fleet_host_agent/README.md#safety-model): no
shell, fixed argument vectors with a validated unit name and a cleared environment, no free text
reaching the host, an atomic and surgical config change, secrets in owner-only files that go only
to their own protocol, a credential that follows no redirect, and fencing on every report.

### RCON

The agent speaks BattlEye RCon over UDP to the loopback `rcon` block of the server config, one
command at a time, with login, retransmission, multi-part responses, server-message
acknowledgement and a keep-alive below the server's timeout; the
[RCON README](/apps/fleet_host_agent/src/rcon/README.md#how-it-works) gives the wire format and
the timings. Delivery is at least once, which is safe only because the agent sends reads:
`#players` and the keep-alive. Reforger's RCON has no broadcast command, which is one reason
broadcasts run in the game runtime.

### Known discrepancies

- `cargo xtask deploy staging` counts the RCON password's three-character minimum in bytes and
  rejects quotes and backslashes (`tools_v2/xtask/src/commands/deploy/staging/host_agent.rs:56-61`)
  — the agent counts characters, accepts quotes and says bytes in its error
  (`apps/fleet_host_agent/src/agent_configuration/secret_files.rs:84-95`), so a password can pass
  one and fail the other.

## Data

- `POST /api/v1/fleet-executor/commands/claim`, `…/{commandId}/executing` and
  `…/{commandId}/result` (the fleet executor handlers in
  `apps/website/api_v2/src/server_infrastructure/`): claim the next command for the credential's
  server with a lease and a fencing token, move it to `executing` inside its execution window, and
  record its outcome; a stale token is a 409 `STALE_FENCING_TOKEN`. The wire shapes are
  `contracts_v2/definitions/fleet-command.schema.json`.
- The agent's TOML configuration and its two secret files under `~/.config/fleet-host-agent/`,
  whose keys and rules are in the crate README's
  [Configuration](/apps/fleet_host_agent/README.md#configuration).
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
- The agent never fetches a mission [artifact](/documentation_v2/glossary.md#artifact): for a cross-terrain deployment it only points the
  server config at the new mission header, and the game runtime loads, verifies and reports the
  artifact when it boots, which confirms the deployment.

## Open work

- [T-940.11 — RCON kick and change-map through the host agent](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_11_plan.md)): the agent gains kick and
  change-map actions with a strict argument grammar. Its plan names `admin.rs` and
  `services/game_agent.rs`, which the API does not have, and `kick` runs in the game runtime
  today, so the plan needs checking against the code before work starts.
- [T-1146 — Decide one RCON password rule for staging and fleet agent](/.ai/tickets/T-1146.toml)
  (idea, no plan): the deploy and the agent apply one password rule.
- [T-1137 — Gate ticket-engine, verification-core, ticketboard and fleet agent tests and clippy](/.ai/tickets/T-1137.toml)
  (idea, no plan): CI runs the agent's tests and clippy.
- [T-086 — Server Control + RCON API](/.ai/tickets/T-086.toml) (deferred, no plan): a live server
  control panel wired to an RCON backend; the platform has no RCON console route.

## Decisions

- Outbound polling instead of an inbound control port: hosts need no exposed service, and the API
  stays the single place commands are recorded.
- One executor per action: the host agent owns process control, RCON reads and the config switch;
  the game runtime owns in-process actions. Neither can perform the other's commands.
- RCON carries reads only: its delivery is at least once, and only an idempotent command survives
  a retransmission unharmed.
- A restart is judged by the unit state read back, not by `systemctl`'s exit status: the exit
  status says the request was queued, not that the server runs.
