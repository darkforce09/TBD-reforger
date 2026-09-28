# Server join debug commands

The `cargo xtask debug` group: probes for finding out why a client cannot join the staging game
server, and the NDJSON debug log they write. The folder also holds the staging console log reader
that `cargo xtask mod remote-logs` runs. Mod developers run both by hand.

## Contents

```text
tools_v2/xtask/src/commands/debug/
├── cli.rs          the `DebugCmd` clap enum: `a2s-probe`, `ndjson-append`, `direct-join-log`, `direct-join`
├── direct_join.rs  `direct-join`: Steam build ids, the addon symlink, the remote probe, ping and A2S of the host
├── dispatch.rs     routes each `DebugCmd` to its function; `a2s-probe`'s host default and port list
├── mod.rs          the module tree
├── probes.rs       the A2S query, the NDJSON row writer and the six-row direct-join block
├── remote_logs/    the verdict, the remote fetch and the self-test of `mod remote-logs`
├── remote_logs.rs  the log patterns of `mod remote-logs`; declares its two files and re-exports `run`
└── tests/          unit tests for the direct-join probes and the remote-logs verdicts
```

## How it works

`tools_v2/xtask/src/cli/dispatch.rs` passes the parsed `DebugCmd` to `dispatch::run`, and every
command returns 0 unless an error surfaces as `xtask: <error>` with exit 1. `direct-join` calls the
same functions as the three primitive commands, in process:

```text
direct_join::run(run_id)
  ├─ Steam build ids: appmanifest_1874880.acf (client) and appmanifest_1874900.acf (server)
  ├─ addon symlink:   ~/.local/share/tbd-server-addons/tbd-framework, resolved
  ├─ TBD_SSH_HOST from deploy.env, resolved once to its first IPv4 address
  ├─ remote probe:    ssh <TBD_SSH_HOST> bash -s   (the profile folder single-quoted)
  ├─ ping and probes::a2s_probe_json_for against that address, ports 2001 and 17777
  ├─ probes::cmd_direct_join_log ─▶ .cursor/debug-8fc1e0.log in the checkout (rows H1 to H6)
  └─ print the summary
```

Local probes are soft: a missing manifest reads `unknown`, a missing symlink `missing` and a failed
ping `fail`. The host, `TBD_SSH_PASS` and `TBD_PROFILE_DIR` (default `/home/<user>/tbd/profile`)
come from `tools_v2/xtask/deploy/deploy.env` through `crate::core::deploy_environment`: the file
decides every key it assigns and the environment fills the rest. With no host, or a file that does
not load, the remote, ping and A2S probes record `skipped`, one stderr line names the file, and the
command still exits 0. The remote probe reports the state of `tbd-reforger.service`, the UDP
listeners on 2001 and 17777, and the last listen, A2S and client lines of the newest `console.log`
under the profile folder. An absent ssh or sshpass reports `service=tool_absent`; any other ssh
failure leaves the remote section empty.

The `remote_logs/` README describes `mod remote-logs`.

## Commands

Each runs as `cargo xtask debug <command>`; a clap usage error exits 2.

### a2s-probe

- Synopsis: `cargo xtask debug a2s-probe [--host <host>] [--ports <port,port,...>]`; without
  `--host` the host is that of `TBD_SSH_HOST` in `deploy.env`, and the ports default to
  `2001,17777`.
- Does: resolves the host once to its first IPv4 address, sends an A2S_INFO query over UDP to each
  port with a 2 s timeout and prints one JSON object keyed `p<port>`, each with `ok` and either
  `address`, `bytes` and `from` or an `error` (a host with no IPv4 address puts the reason in every
  port's `error`).
- Exit codes: 0 printed, whatever the probes answered; 1 no port in `--ports` parses, or no
  `--host` and no usable `TBD_SSH_HOST` (`debug a2s-probe: pass --host, or set TBD_SSH_HOST in
  <path>`).
- Example: `cargo xtask debug a2s-probe --ports 17777`

### ndjson-append

- Synopsis: `cargo xtask debug ndjson-append --log <path> --hypothesis <id> --message <text>
  [--data <json>] [--run-id <id>]`
- Does: appends one row to the log with `sessionId`, `timestamp` (ms), `location`, `message`,
  `data`, `hypothesisId` and `runId`; `--data` that is not JSON is written as `{}`.
- Exit codes: 0 appended; 1 the log cannot be opened.
- Example: `cargo xtask debug ndjson-append --log /tmp/join.log --hypothesis H1 --message probe`

### direct-join-log

- Synopsis: `cargo xtask debug direct-join-log --log <path> --run-id <id> [--remote <text>]
  --client-build <id> --server-build <id> --symlink <path> --ping <ms> [--host <host>]
  [--address <ipv4>] --a2s-json <json>`
- Does: appends the six hypothesis rows: H1 service and ports parsed from `--remote`, H2 whether
  the build ids match, H3 the remote snippet, H4 the ping with the host and address it went to, H5
  whether the symlink exists, H6 the A2S result.
- Exit codes: 0 appended; 1 the log cannot be opened.
- Example: `cargo xtask debug direct-join-log --log /tmp/join.log --run-id r1 --client-build 1
  --server-build 1 --symlink missing --ping fail --a2s-json '{}'`

### direct-join

- Synopsis: `cargo xtask debug direct-join [<run-id>]`; the run id defaults to `user-repro`.
- Does: runs the probes above against the host of `TBD_SSH_HOST`, writes their six rows to
  `debug-8fc1e0.log` in the checkout's `.cursor/`, and prints the build ids, the symlink, the ping,
  the A2S answer and the remote section.
- Exit codes: 0 written, also with no host configured (the host's probes then read `skipped`); 1
  the log cannot be written.
- Example: `cargo xtask debug direct-join`

## Boundaries

- Depends on: `crate::core::repository_root`, `crate::core::deploy_environment` (the settings
  file, the deploy host, its IPv4 address and the profile folder) and
  `crate::core::test_environment::PathGuard`; `verification_core` for runs, patterns and verdicts;
  `serde_json` and `regex`; ssh, sshpass and ping on the development machine.
- Used by: `tools_v2/xtask/src/cli/dispatch.rs`; `tools_v2/xtask/src/commands/mod_ops/dispatch.rs`,
  for `mod remote-logs`; `cargo xtask deploy staging`, which runs `mod remote-logs` last; people.
- Rules: a local probe never aborts the summary, and a missing Steam manifest reads `unknown`
  while a `buildid` line with two fields reads empty
  (`steam_two_field_buildid_is_empty_not_unknown` in `tests/direct_join/tests.rs`); with no host
  the host's probes record `skipped` and reach no network
  (`clean_empty_home_writes_unknown_and_missing`); documents name the host only as `TBD_SSH_HOST`.

## Related documentation

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  server these commands probe.
- [Client join and mod updates](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md)
  — running `debug direct-join` around a failed Direct Join.
- [Boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md)
  — running `mod remote-logs`, its four outcomes and the log lines it matches.
