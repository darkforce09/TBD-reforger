# fleet-host-agent

The fleet host agent runs on each game host beside the Arma Reforger dedicated server. It polls
the platform API outbound over HTTPS for the fleet commands addressed to its server, performs
each one through fixed process-control actions, BattlEye RCon commands or a scenario switch in
the server's JSON config, and reports every step back to the API's command ledger. The API never connects to the host; a host behind NAT
or a firewall needs only outbound HTTPS.

The API side of the ledger is described in `documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md`;
the wire contract is `contracts_v2/definitions/fleet-command.schema.json` and the credential
format `contracts_v2/definitions/machine-credential.schema.json`.

## What it does

The agent runs one loop:

1. `POST {api}/api/v1/fleet-executor/commands/claim` with body `{}`. A 204 means nothing is
   claimable; the agent claims again after `poll_interval_seconds`. A 200 carries one claimed
   command with its fencing token.
2. The command's action and arguments are validated again (see the safety model). A command
   that fails validation, including one naming an action this host does not perform, is
   reported failed at once (`POST .../commands/{id}/result` with `succeeded: false`) and nothing
   runs.
3. `POST .../commands/{id}/executing` with the fencing token. Nothing runs until the API
   acknowledges this report. A transient failure (network error, 408, 429, 5xx) is retried with
   jittered backoff; a 409 `STALE_FENCING_TOKEN` means the claim was taken away, and the agent
   abandons the command without acting and never reports it again.
4. The action runs and its outcome is observed.
5. `POST .../commands/{id}/result` with `succeeded`, `outcome` and, for a failure,
   `failure_reason` (at most 512 bytes). The report is retried with backoff until the API
   acknowledges it or answers 409. The effect itself is never repeated: when the outcome of a
   non-idempotent command stays unreported, the ledger marks it `indeterminate` and an operator
   decides.

Commands run one at a time. A failed claim backs off from 1 s to 60 s with jitter. Every
transition is logged with the command id, action and fencing token.

## Actions

| Action | What the agent runs | Succeeded when | Outcome |
|:---|:---|:---|:---|
| `start` | `systemctl --user start <unit>`, dwell, state read | LoadState `loaded` and ActiveState `active` after the dwell | `unit`, `load_state`, `active_state`, `dwell_milliseconds`, `systemctl` |
| `stop` | `systemctl --user stop <unit>`, state read | LoadState `loaded` and ActiveState `inactive` | as above |
| `restart` | `systemctl --user restart <unit>`, dwell, state read | LoadState `loaded` and ActiveState `active` after the dwell | as above |
| `list_players` | RCON `#players` | the server answered | `{"players": [{"player_id", "arma_id", "name"}]}`, plus `raw_lines` when some line of the answer was not understood |
| `restart_with_mission` | set `game.scenarioId` in the server config, then as `restart` | the config was switched, and LoadState `loaded` and ActiveState `active` after the dwell | `{"scenario_id", "unit_active_state", "config_path"}` |

The state is read with `systemctl --user show --property=LoadState --value <unit>` and
`systemctl --user show --property=ActiveState --value <unit>`. The exit status of the verb is
recorded but does not decide the verdict: an Arma Reforger server that fails to start exits
with status 0 a few seconds after systemd reported the start as done, so start and restart wait
`start_dwell_seconds` and then read the unit's state. `systemctl show` reports a unit that does
not exist as inactive, so a unit that is not `loaded` never counts as a success. A failure
reason names the state observed, for example
`restart left tbd-reforger.service failed instead of active after waiting 8s; systemctl --user restart exited with status 0`.

`broadcast`, `kick` and `load_mission` run in the game runtime: Reforger's RCON has no
broadcast command, the runtime knows its players by identity, and it reloads its own scenario.
The host agent refuses them, as it refuses any action it does not perform.

## Mission restart

`restart_with_mission` is a cross-terrain mission deployment: the server restarts on the
scenario header of another terrain. Its arguments, built by the API and validated again here,
are exactly `deployment_id` and `artifact_id` (hyphenated UUIDs), `artifact_sha256` (64
lowercase hex digits) and `scenario_id`, a scenario header resource matching
`^\{[0-9A-F]{16}\}[A-Za-z0-9_./-]+\.conf$`, such as
`{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`. Any other key or value is refused before
anything runs.

After `executing` is acknowledged, exactly as for `restart`:

1. The dedicated server's JSON config (`game_server.server_config_path`) is read, and only the
   string value of `game.scenarioId` is replaced. Every other byte stays as it was: keys the
   agent does not know, their order, the spelling of numbers and the whitespace. The result is
   parsed again and must equal the original with `game.scenarioId` alone changed.
2. The new text replaces the file atomically: it is written to a new file in the same
   directory, given the original file's mode, flushed with fsync and renamed over the original
   (a symbolic link is followed, so the file it names is replaced). The path names the complete
   old file until it names the complete new one.
3. The unit restarts as for `restart`, with the same dwell and read-back verdict.

A config that cannot be read, is not JSON, has no single `game.scenarioId` string, or cannot be
replaced fails the command with the file unchanged and nothing restarted
(`config_updated: false`). A restart that fails after the switch fails the command too; its
reason and outcome (`config_updated: true`, `unit_active_state`, `unit_load_state`,
`systemctl`) say that the config already names the new scenario, which the next start runs.

The agent never fetches the artifact. The game runtime reads `GET /game-runtime/deployment` when
it boots, loads the artifact and reports it; that report, not this command, confirms the
deployment.

## RCON

Arma Reforger's own RCON server (the `rcon` block of the server config, UDP port 19999 by
default) speaks the BattlEye RCon protocol, which the agent implements from its specification:

- Every datagram reads `'B' 'E' | CRC32 (little-endian) | 0xFF | type | payload`, the CRC32
  covering every byte from `0xFF` onward. A datagram with a wrong checksum or header is dropped.
- Login (`0x00`) sends the password; the server answers accepted or refused. A refused password
  fails the command at once; an unanswered login is retransmitted, then reported.
- Commands (`0x01`) carry a one-byte sequence number that starts at 0 and wraps after 255, so
  numbers are reused. An unanswered command is retransmitted under the same number. A response
  split into several packets (`0x00 | parts | index` header) is reassembled in index order,
  whatever order the parts arrive in and however often.
- Server messages (`0x02`) are acknowledged with their sequence number, every copy of them;
  a repeated message is logged once.
- An empty command is sent whenever 30 s pass without a command, below the server's 45 s
  timeout. When a keep-alive or command goes unanswered the session counts as lost; an
  unanswered command is sent once more after a new login, which is safe because every command
  the agent sends over RCON is a read.
- On shutdown the agent sends `@logout`, which frees its RCON slot at once.

The commands are the ones Arma Reforger's RCON accepts (Bohemia Interactive wiki, "Arma
Reforger:Server Management"):

- `#players` lists the session's players and their playerId, and is permitted to RCON monitor
  clients as well as admin clients. Rows read `<playerId> ; <identity UID> ; <name>`; the
  identity UID is reported as `arma_id`.
- `@logout` de-authenticates the RCON client at once.

## Configuration

The agent takes one argument, the path of its TOML configuration file:

```toml
# The platform API origin. https is required unless the host is a loopback address.
api_base_url = "https://tbd.example.org"
# The host agent's machine credential (tbdm_...), issued by an administrator with
# POST /api/v1/servers/{id}/credentials and executor_kind "host_agent".
credential_file = "/home/tbd/.config/fleet-host-agent/machine-credential"
# Seconds between claims while nothing is queued (1 to 300, default 5).
poll_interval_seconds = 5

[game_server]
# The game server's systemd user unit.
systemd_user_unit = "tbd-reforger.service"
# The dedicated server's JSON config (its -config file), which restart_with_mission rewrites.
# Required; an absolute path to an existing file the agent can write, in a directory the agent
# can write (the file is replaced by rename).
server_config_path = "/home/tbd/reforger/configs/server.json"
# Seconds to wait after start and restart before reading the unit's state (0 to 30, default 8).
start_dwell_seconds = 8
# Absolute path of systemctl (default /usr/bin/systemctl).
systemctl_program = "/usr/bin/systemctl"

[rcon]
# The address and port of the server's rcon block.
address = "127.0.0.1"
port = 19999
# The rcon.password of the server config.
password_file = "/home/tbd/.config/fleet-host-agent/rcon-password"
```

Loading validates every key and stops the agent with a named error (exit status 78) when one
is wrong: an unknown key, a URL that is not https (or loopback http), carries credentials, a
query or a fragment, a poll interval or dwell out of range, a unit name that is not a systemd
`.service` unit, a relative `systemctl_program`, a `server_config_path` that is relative, missing,
not a regular file or not writable by the agent, an `address` that is not an IP address, port 0,
or a secret file that fails the checks below.

## Safety model

- **No shell.** Process control runs the configured `systemctl` program directly with fixed
  argument vectors: `--user start|stop|restart <unit>` and
  `--user show --property=LoadState|ActiveState --value <unit>`. The only variable element is
  the unit name, validated at startup (ASCII letters, digits and `:_.@\-`, ending in
  `.service`, never starting with `-`), so it cannot read as an option. The program runs with a
  cleared environment (only `XDG_RUNTIME_DIR` and `DBUS_SESSION_BUS_ADDRESS` pass through, to
  reach the user's systemd manager), no standard input, and a timeout after which it is killed.
- **No free text reaches the host.** Each claimed command is validated again, by the API's
  rules: an action this host does not perform is refused, each action accepts exactly its own
  argument keys, and a mission restart's scenario must be a scenario header resource. The
  scenario is written into the server config only as a JSON string value, never into a command
  line. An RCON command is one packet of text without control characters, and the agent sends
  only fixed ones.
- **The server config changes atomically and surgically.** Only `game.scenarioId` changes, the
  file mode is kept, and a failure at any step leaves the original file and no partial file.
- **Secrets live in files** named by the configuration, never in the configuration itself.
  Each must be an absolute path to a regular file of at most 4 KiB that no other user can read
  or write (no group or other permission bits, for example mode 600). The credential must match
  `tbdm_<32 hex>_<64 hex>`; the RCON password must have 3 to 256 bytes and no spaces, as Arma
  Reforger requires.
- **Secrets go only to their own protocol.** The credential is sent only in the
  `Authorization: Bearer` header of API requests (marked sensitive, so the HTTP client never
  logs it); the RCON password only in the RCON login packet. Neither is logged or printed:
  their type redacts itself in debug output, and configuration errors never quote them.
- **The credential reaches only the configured origin.** Redirects are never followed, HTTPS is
  enforced for an https origin, and plain http is accepted only for a loopback API.
- **Fencing.** Every report carries the claim's fencing token; a stale claim is abandoned
  without acting, and no effect runs without an acknowledged `executing` report.

## Running as a systemd user service

The agent runs as the same user as the game server's user unit, so `systemctl --user` reaches
that user's systemd manager.

```sh
cargo build --release -p fleet-host-agent
install -m 755 target/release/fleet-host-agent ~/.local/bin/fleet-host-agent
install -d -m 700 ~/.config/fleet-host-agent
install -m 600 /dev/null ~/.config/fleet-host-agent/machine-credential   # then write the tbdm_ secret into it
install -m 600 /dev/null ~/.config/fleet-host-agent/rcon-password        # then write the rcon.password into it
```

`~/.config/systemd/user/fleet-host-agent.service`:

```ini
[Unit]
Description=TBD fleet host agent
After=network-online.target

[Service]
ExecStart=%h/.local/bin/fleet-host-agent %h/.config/fleet-host-agent/agent.toml
Restart=on-failure
RestartSec=5
# An invalid configuration exits 78; restarting cannot fix it.
RestartPreventExitStatus=78
# A command in progress is performed and reported before the agent exits.
TimeoutStopSec=200

[Install]
WantedBy=default.target
```

```sh
systemctl --user daemon-reload
systemctl --user enable --now fleet-host-agent.service
loginctl enable-linger "$USER"    # keep the user manager, and both units, running without a login
journalctl --user -u fleet-host-agent.service -f
```

The agent stops claiming on SIGTERM or SIGINT, finishes and reports a command in progress,
sends `@logout` to the RCON server and exits 0. Exit status 2 is a usage error, 78 an invalid
configuration, 1 a failure to start. Logs go to standard error (the journal); `RUST_LOG`
overrides the default `info` level.

## Tests

```sh
cargo test -p fleet-host-agent --locked
```

- Unit tests (`src/**/tests/`) cover packet encoding and decoding against an independent
  CRC32, dropping of corrupted packets, sequence wrap-around, fragment reassembly, the
  `#players` reader, argument re-validation (the mission deployment included), the process
  verdict, locating and rewriting `game.scenarioId`, atomic file replacement, configuration
  validation and the classification of API failures.
- `tests/rcon_transport.rs` (`rcon_transport_*`) runs the RCON client against an in-process
  BattlEye RCon server that loses, corrupts, duplicates, reorders and fragments packets, drops
  idle logins and forgets them on restart.
- `tests/process_control.rs` (`process_control_*`) points `systemctl_program` at a small script
  each test writes, and checks the fixed argument vectors, the verdict logic, and the mission
  restart's config switch: unknown keys and every other byte kept, no partial file when the
  write fails, a malformed config refused without a restart, and a failed restart reported with
  the new scenario in place.
- `tests/host_agent_ledger.rs` (`host_agent_ledger_*`) runs the claim loop against a local
  stand-in of the API's executor routes: polling after 204, `executing` acknowledged before any
  effect, `STALE_FENCING_TOKEN` abandoning a command, result reports retried until acknowledged,
  commands refused without acting, and a mission restart whose config is unchanged at every
  `executing` report and untouched when the claim is taken away.

No test needs the network beyond the loopback sockets it opens.

## Layout

```text
src/
  main.rs                         Command line, logging, wiring, shutdown on SIGTERM/SIGINT.
  lib.rs                          Module tree.
  action_verdict.rs               The observed outcome of an action, in the ledger's shape.
  secret_text.rs                  Secrets that never print.
  agent_configuration/            The TOML file, its validation and the secret files.
  ledger_client/                  Wire messages, HTTP calls, backoff and the claim loop.
  command_execution/              Re-validation of claimed commands and the host's executor.
  dedicated_server_config/        The surgical game.scenarioId rewrite and atomic replacement.
  process_control/                systemctl invocations and the unit-state verdict.
  rcon/                           BattlEye RCon codec, session, reassembly and Reforger commands.
tests/
  rcon_transport.rs               rcon_transport_* against the in-process RCon server.
  process_control.rs              process_control_* against a stand-in systemctl.
  host_agent_ledger.rs            host_agent_ledger_* against a stand-in API.
  test_support/                   The stand-in RCon server, systemctl and API.
```
