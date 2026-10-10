# Game server host agent

The `game_server_host_agent` crate: the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent)
that runs beside each Arma Reforger dedicated server of a game host, one agent per fleet instance.
It polls the
[API](/documentation/glossary/a_to_f.md#api) outbound over HTTPS for the
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command) addressed to its server, performs each
one through fixed process-control actions, [RCON](/documentation/glossary/n_to_z.md#rcon) reads, an
operator's console line sent once over RCON, or a
[mission header](/documentation/glossary/g_to_m.md#mission-header) switch in the server's JSON config,
and reports every step to the API's command ledger.

## Contents

```text
crates/fleet/game_server_host_agent/
├── Cargo.toml  the `game_server_host_agent` package: the `game_server_host_agent` library and the binary of that name
├── src/        the library modules and the binary's entry point
└── tests/      integration tests against a stand-in API, systemctl and BattlEye RCon server
```

## How it works

The API never connects to the host: a host behind NAT or a firewall needs only outbound HTTPS.
The agent authenticates with a `host_agent`
[machine credential](/documentation/glossary/g_to_m.md#machine-credential) and runs one loop, one
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
over RCON (`#players`); `console_command`, one operator line for the game server's console,
transmitted once over RCON with its reply of at most 4096 bytes as the outcome; and
`restart_with_mission`, a cross-terrain
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) that rewrites only
`game.scenarioId` in the server config and then restarts the unit. `broadcast`, `kick` and
`load_mission` run in the [game runtime](/documentation/glossary/g_to_m.md#game-runtime), and the agent
refuses them. `src/command_execution/README.md` tables each action's success rule and outcome.

### Safety model

- No shell: `systemctl` runs directly with fixed argument vectors whose only variable is the unit
  name validated at startup, with a cleared environment (only `XDG_RUNTIME_DIR` and
  `DBUS_SESSION_BUS_ADDRESS` pass), no standard input and a timeout.
- Free text reaches the host only as a console line: each claimed command is validated again, a
  mission header reaches the config only as a JSON string value, and RCON carries the agent's
  fixed read commands and the console line. The line is one line of at most 256 bytes without
  control characters or line separators and never starts with `@`, so it cannot end the agent's
  own RCON login; it reaches only the game server's RCON port, in one packet that is never sent
  again, and a line that gets no reply is reported as possibly run.
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
cargo test -p game_server_host_agent --locked     # unit and integration tests; loopback sockets only
cargo build --release -p game_server_host_agent   # target/release/game_server_host_agent
cargo xtask deploy staging --dry-run              # prints the staging plan, the host agent of every fleet instance included
```

`cargo xtask deploy staging` runs one agent per fleet instance on the host that `TBD_SSH_HOST`
names: it builds the agent there, installs it as `~/.local/bin/game_server_host_agent`, writes
each instance's `~/.config/game_server_host_agent/instance-N/agent.toml` (mode 600 in a mode 700
directory), installs the template unit `deploy/systemd/game_server_host_agent@.service`, enables
lingering, restarts `game_server_host_agent@N.service` for every instance N from 1 to
`TBD_FLEET_INSTANCES` and fails unless each is `active`. Each configuration names its instance's
game server unit `tbd-reforger@N.service`, its `~/tbd/fleet/instance-N/server.config.json` and two
owner-only files under `~/tbd/fleet/instance-N/secrets/`: `host-agent-credential`, which
`cargo xtask staging provision-fleet` writes, and `rcon-password`, which the deploy generates on
the host. Each instance's rendered server config carries a loopback `rcon` block with admin
permission on port `TBD_FLEET_RCON_PORT_BASE + N`. Every agent polls `TBD_HOST_AGENT_API_URL`
(default `TBD_BACKEND_URL`) except the relay instance's (`TBD_FLEET_RELAY_INSTANCE`, instance 5
on staging), which polls the acknowledgement-dropping relay on `127.0.0.1:TBD_FLEET_RELAY_PORT`,
itself forwarding to the API.

By hand, as the user that runs the game server units (so `systemctl --user` reaches that user's
manager): install the binary as `~/.local/bin/game_server_host_agent`, write instance N's
configuration as `~/.config/game_server_host_agent/instance-N/agent.toml` and its two secret files
under
`~/tbd/fleet/instance-N/secrets/`, each mode 600 in a mode 700 directory, copy the template unit
to `~/.config/systemd/user/`, then, for instance 1:

```bash
systemctl --user daemon-reload
systemctl --user enable --now game_server_host_agent@1.service
loginctl enable-linger "$USER"                  # keep the user manager and the fleet's units running without a login
journalctl --user -u game_server_host_agent@1.service -f
```

The agent stops claiming on SIGTERM or SIGINT, finishes and reports a command in progress, sends
`@logout` over RCON and exits 0. The unit restarts it on failure after 5 s, except after exit 78,
and allows 200 s for a stop.

## Configuration

The binary takes one argument, the path of its TOML configuration file. The example is fleet
instance 1's, for a deploy user whose home is `/home/tbd`, with the optional keys at their
defaults:

```toml
# The API origin: https, or http only on a loopback host.
api_base_url = "https://tbd.example.org"
# The host_agent machine credential (tbdm_...), issued with POST /api/v1/servers/{id}/credentials.
credential_file = "/home/tbd/tbd/fleet/instance-1/secrets/host-agent-credential"
# Seconds between claims while nothing is queued (1 to 300, default 5).
poll_interval_seconds = 5

[game_server]
# The game server's systemd user unit: tbd-reforger@N.service for fleet instance N.
systemd_user_unit = "tbd-reforger@1.service"
# The dedicated server's JSON config (its -config file), which restart_with_mission rewrites:
# an absolute path to an existing file the agent can write, in a directory it can write.
server_config_path = "/home/tbd/tbd/fleet/instance-1/server.config.json"
# Seconds to wait after start and restart before reading the unit's state (0 to 30, default 8).
start_dwell_seconds = 8
# Absolute path of systemctl (default /usr/bin/systemctl).
systemctl_program = "/usr/bin/systemctl"

[rcon]
# The address (an IP address) and port (default 19999) of the server config's rcon block;
# fleet instance N's port is TBD_FLEET_RCON_PORT_BASE + N.
address = "127.0.0.1"
port = 19999
# The server config's rcon.password.
password_file = "/home/tbd/tbd/fleet/instance-1/secrets/rcon-password"
```

Loading rejects an unknown key and validates every value before the agent starts; each secret
file must be an absolute path to a regular file of at most 4 KiB with no group or other permission
bits. `src/agent_configuration/README.md` lists every rule. `RUST_LOG` sets the log filter
(`info` when unset); logs go to standard error, which the journal collects.

## Public surface

- The `game_server_host_agent` binary: `game_server_host_agent <configuration-file>` runs until
  SIGTERM or SIGINT; `--help` or `-h` prints the usage. Exit status 0 after a requested shutdown or `--help`,
  2 for a usage error, 78 for an invalid configuration, 1 when the RCON socket or the API client
  cannot be set up.
- The library `game_server_host_agent`, whose modules the integration tests drive; its prelude
  (`game_server_host_agent::prelude`) names the configuration, the command loop and executor, the
  three host channels, the typed ids and the crate's `Error` and `Result`; no other crate depends
  on it.
- The HTTP user agent `game_server_host_agent/<version>` on every request; nothing matches on it.
- The HTTP calls it makes: the three `/api/v1/fleet-executor/commands` routes above. It serves
  none.

## Boundaries

- Depends on: of the workspace, only the contracts crate `fleet_wire_contract`
  (`crates/contracts/fleet_wire_contract`), which carries the wire contracts
  `contracts/definitions/fleet-command.schema.json` and
  `contracts/definitions/machine-credential.schema.json` the API shares, and the foundation crate
  `newtype_ids`, which declares the typed ids; at run time the API's executor routes in
  `crates/api/api_server_infrastructure/src/`, with a `host_agent` machine credential, the calling
  user's systemd manager, the dedicated server's JSON config and its BattlEye RCon port.
- Used by: no workspace member depends on it. `cargo xtask deploy staging`
  (`tools/commands/deployment/src/staging/host_agent.rs`) builds the binary, configures one agent
  per fleet instance and runs each as an instance of `deploy/systemd/game_server_host_agent@.service`;
  over HTTP, the API's fleet command ledger, which Server Control and mission deployments feed.
- Rules: a fleet crate (`category = "crates/fleet"`) depends only on foundation crates built for
  every target and contracts crates, and speaks to the API over HTTP only
  (`cargo xtask verify crate-tiers`); the library holds the crate anatomy
  (`cargo xtask verify crate-anatomy`); the tests in `tests/` need no network beyond the loopback sockets they open, and
  `tests/test_support/` holds their stand-ins; the claim, fencing and reporting rules hold under
  the `host_agent_ledger_*` tests, the systemctl argument vectors and verdicts under the
  `process_control_*` tests, and the RCON transport, the console command's single transmission
  included, under the `rcon_transport_*` tests;
  `cargo xtask verify file-length` covers `src/` and `tests/`.

## Related documentation

- [Fleet command ledger](/documentation/crates/api/api_server/design_notes/fleet_command_ledger.md)
  — the API side: command states, leases, fencing and execution windows.
- [Machine credentials](/documentation/crates/api/api_server/design_notes/machine_credentials.md)
  — issuing and revoking the credential the agent authenticates with.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — deploying the
  staging game server with the host agent.
- [Fleet command execution](/documentation/crates/fleet/game_server_host_agent/fleet_command_execution.md) — the
  design across the API, the agent and the game runtime, and the open work.
