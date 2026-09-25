# Fleet host agent

The `fleet-host-agent` crate: the [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent)
that runs on each game host beside the Arma Reforger dedicated server. It polls the
[API](/documentation_v2/glossary.md#api) outbound over HTTPS for the
[fleet commands](/documentation_v2/glossary.md#fleet-command) addressed to its server, performs each
one through fixed process-control actions, [RCON](/documentation_v2/glossary.md#rcon) reads or a
[mission header](/documentation_v2/glossary.md#mission-header) switch in the server's JSON config,
and reports every step to the API's command ledger.

## Contents

```text
apps/fleet_host_agent/
├── Cargo.toml  the `fleet-host-agent` package: the `fleet_host_agent` library and the binary of that name
├── src/        the library modules and the binary's entry point
└── tests/      integration tests against a stand-in API, systemctl and BattlEye RCon server
```

## How it works

The API never connects to the host: a host behind NAT or a firewall needs only outbound HTTPS.
The agent authenticates with a `host_agent`
[machine credential](/documentation_v2/glossary.md#machine-credential) and runs one loop, one
command at a time:

1. `POST /api/v1/fleet-executor/commands/claim` with the body `{}`. A 204 means nothing is
   claimable, and the agent claims again after `poll_interval_seconds`; a 200 carries one command
   and the fencing token every later report of it carries. A failed claim backs off from 1 s to
   60 s with jitter.
2. The command's action and arguments are validated again by the API's rules. A command that
   fails, including one naming an action this host does not perform, is reported failed at once
   and nothing runs.
3. `POST /api/v1/fleet-executor/commands/{commandId}/executing`. Nothing runs until the API
   acknowledges it; transient failures (network, 408, 429, 5xx) are retried with backoff, and a
   409 `STALE_FENCING_TOKEN` or any other refusal abandons the command without acting.
4. The action runs and its outcome is observed.
5. `POST /api/v1/fleet-executor/commands/{commandId}/result` with `succeeded`, `outcome` and, for a
   failure, a `failure_reason` of at most 512 bytes, retried until the API acknowledges or refuses
   it. The effect itself is never repeated; an outcome that stays unreported leaves the ledger to
   mark the command indeterminate.

The host performs `start`, `stop` and `restart` of the game server's systemd user unit, judged by
the unit state read back after a dwell rather than by `systemctl`'s exit status; `list_players`
over RCON (`#players`); and `restart_with_mission`, a cross-terrain
[mission deployment](/documentation_v2/glossary.md#mission-deployment) that rewrites only
`game.scenarioId` in the server config and then restarts the unit. `broadcast`, `kick` and
`load_mission` run in the [game runtime](/documentation_v2/glossary.md#game-runtime), and the agent
refuses them. `src/command_execution/README.md` tables each action's success rule and outcome.

### Safety model

- No shell: `systemctl` runs directly with fixed argument vectors whose only variable is the unit
  name validated at startup, with a cleared environment (only `XDG_RUNTIME_DIR` and
  `DBUS_SESSION_BUS_ADDRESS` pass), no standard input and a timeout.
- No free text reaches the host: each claimed command is validated again, a mission header
  reaches the config only as a JSON string value, and RCON carries only the agent's fixed read
  commands.
- The server config changes atomically and surgically: only `game.scenarioId` changes, the file
  mode is kept, and a failure at any step leaves the original file and no partial file.
- Secrets live in files the configuration names, each readable by its owner alone, and go only to
  their own protocol: the credential in the sensitive `Authorization: Bearer` header of API
  requests, the RCON password in the login packet. Their type redacts itself in debug output, and
  configuration errors never quote them.
- The credential reaches only the configured origin: redirects are never followed, HTTPS is
  enforced for an https origin, and plain http is accepted only for a loopback API.
- Fencing: every report carries the claim's fencing token, and no effect runs without an
  acknowledged `executing` report.

## Getting started

Run these from the repository root:

```bash
cargo test -p fleet-host-agent --locked     # unit and integration tests; loopback sockets only
cargo build --release -p fleet-host-agent   # target/release/fleet-host-agent
cargo xtask deploy staging --dry-run        # prints the staging plan; host agent steps with TBD_INSTALL_HOST_AGENT=1
```

With `TBD_INSTALL_HOST_AGENT=1` in `tools_v2/xtask/deploy/deploy.env`, `cargo xtask deploy staging`
builds the agent on the host that `TBD_SSH_HOST` names, writes `~/.config/fleet-host-agent/`
(`agent.toml`, `machine-credential` and `rcon-password`, mode 600 in a mode 700 directory),
installs the user unit `tools_v2/xtask/deploy/systemd/fleet-host-agent.service`, enables lingering,
starts the unit and fails unless it is `active`. It needs `TBD_SERVER_MODE=config`, a
`TBD_HOST_AGENT_CREDENTIAL` and a `TBD_RCON_PASSWORD`; the rendered server config then gains a
loopback `rcon` block with monitor permission.

By hand, as the user that runs the game server's unit (so `systemctl --user` reaches that user's
manager): install the binary as `~/.local/bin/fleet-host-agent`, write the configuration and the
two secret files under `~/.config/fleet-host-agent/` with mode 600, copy the unit to
`~/.config/systemd/user/`, then:

```bash
systemctl --user daemon-reload
systemctl --user enable --now fleet-host-agent.service
loginctl enable-linger "$USER"                  # keep the user manager and both units running without a login
journalctl --user -u fleet-host-agent.service -f
```

The agent stops claiming on SIGTERM or SIGINT, finishes and reports a command in progress, sends
`@logout` over RCON and exits 0. The unit restarts it on failure after 5 s, except after exit 78,
and allows 200 s for a stop.

## Configuration

The binary takes one argument, the path of its TOML configuration file:

```toml
# The API origin: https, or http only on a loopback host.
api_base_url = "https://tbd.example.org"
# The host_agent machine credential (tbdm_...), issued with POST /api/v1/servers/{id}/credentials.
credential_file = "/home/tbd/.config/fleet-host-agent/machine-credential"
# Seconds between claims while nothing is queued (1 to 300, default 5).
poll_interval_seconds = 5

[game_server]
# The game server's systemd user unit.
systemd_user_unit = "tbd-reforger.service"
# The dedicated server's JSON config (its -config file), which restart_with_mission rewrites:
# an absolute path to an existing file the agent can write, in a directory it can write.
server_config_path = "/home/tbd/reforger/configs/server.json"
# Seconds to wait after start and restart before reading the unit's state (0 to 30, default 8).
start_dwell_seconds = 8
# Absolute path of systemctl (default /usr/bin/systemctl).
systemctl_program = "/usr/bin/systemctl"

[rcon]
# The address (an IP address) and port (default 19999) of the server config's rcon block.
address = "127.0.0.1"
port = 19999
# The server config's rcon.password.
password_file = "/home/tbd/.config/fleet-host-agent/rcon-password"
```

Loading rejects an unknown key and validates every value before the agent starts; each secret
file must be an absolute path to a regular file of at most 4 KiB with no group or other permission
bits. `src/agent_configuration/README.md` lists every rule. `RUST_LOG` sets the log filter
(`info` when unset); logs go to standard error, which the journal collects.

## Public surface

- The `fleet-host-agent` binary: `fleet-host-agent <configuration-file>` runs until SIGTERM or
  SIGINT; `--help` or `-h` prints the usage. Exit status 0 after a requested shutdown or `--help`,
  2 for a usage error, 78 for an invalid configuration, 1 when the RCON socket or the API client
  cannot be set up.
- The library `fleet_host_agent`, whose modules the integration tests drive; no other crate
  depends on it.
- The HTTP calls it makes: the three `/api/v1/fleet-executor/commands` routes above. It serves
  none.

## Boundaries

- Depends on: the API's executor routes in `apps/website/api_v2/src/server_infrastructure/`, with
  a `host_agent` machine credential; the wire contracts
  `contracts_v2/definitions/fleet-command.schema.json` and
  `contracts_v2/definitions/machine-credential.schema.json`; the calling user's systemd manager;
  the dedicated server's JSON config and its BattlEye RCon port.
- Used by: `cargo xtask deploy staging` (`tools_v2/xtask/src/commands/deploy/staging/host_agent.rs`),
  which builds, configures and installs it with the unit
  `tools_v2/xtask/deploy/systemd/fleet-host-agent.service`; over HTTP, the API's fleet command
  ledger, which Server Control and mission deployments feed.
- Rules: the tests in `tests/` need no network beyond the loopback sockets they open, and
  `tests/test_support/` holds their stand-ins; the claim, fencing and reporting rules hold under
  the `host_agent_ledger_*` tests, the systemctl argument vectors and verdicts under the
  `process_control_*` tests, and the RCON transport under the `rcon_transport_*` tests;
  `cargo xtask verify file-length` covers `src/` and `tests/`.

## Related documentation

- [Fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the API side: command states, leases, fencing and execution windows.
- [Machine credentials](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md)
  — issuing and revoking the credential the agent authenticates with.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — deploying the
  staging game server with the host agent.
- [Fleet command execution](/documentation_v2/fleet_host_agent/fleet_command_execution.md) — the
  design across the API, the agent and the game runtime, and the open work.
