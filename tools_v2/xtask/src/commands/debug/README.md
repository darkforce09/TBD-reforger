# Server join debug commands

The `cargo xtask debug` group: probes for finding out why a client cannot join the staging game
server, and the NDJSON debug log they write. The folder also holds the staging console log reader
that `cargo xtask mod remote-logs` runs. Mod developers run both by hand.

## Contents

```text
tools_v2/xtask/src/commands/debug/
├── cli.rs          the `DebugCmd` clap enum: `a2s-probe`, `ndjson-append`, `direct-join-log`, `direct-join`
├── direct_join.rs  `direct-join`: Steam build ids, the addon symlink, the remote probe, ping and A2S
├── dispatch.rs     routes each `DebugCmd` to its function and turns an empty port list into an error
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
  ├─ remote probe:    ssh <TBD_SSH_HOST> bash -s   (skipped when no host is set)
  ├─ ping and probes::a2s_probe_json against a fixed LAN address, ports 2001 and 17777
  ├─ probes::cmd_direct_join_log ─▶ .cursor/debug-8fc1e0.log in the checkout (rows H1 to H6)
  └─ print the summary
```

Local probes are soft: a missing manifest reads `unknown`, a missing symlink `missing` and a failed
ping `fail`. The remote probe reads `TBD_SSH_HOST` and `TBD_SSH_PASS` from the environment,
overridden by `tools_v2/xtask/deploy/deploy.env`; it reports the state of `tbd-reforger.service`,
the UDP listeners on 2001 and 17777, and the last listen, A2S and client lines of the newest
`console.log`. An absent ssh or sshpass reports `service=tool_absent`; any other ssh failure leaves
the remote section empty.

The `remote_logs/` README describes `mod remote-logs`.

## Commands

Each runs as `cargo xtask debug <command>`; a clap usage error exits 2.

### a2s-probe

- Synopsis: `cargo xtask debug a2s-probe [--host <host>] [--ports <port,port,...>]`; the host
  defaults to the staging server's LAN address and the ports to `2001,17777`.
- Does: sends an A2S_INFO query over UDP to each port with a 2 s timeout and prints one JSON
  object keyed `p<port>`, each with `ok` and either `bytes` and `from` or `error`.
- Exit codes: 0 printed, whatever the probes answered; 1 no port in `--ports` parses.
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
  --client-build <id> --server-build <id> --symlink <path> --ping <ms> --a2s-json <json>`
- Does: appends the six hypothesis rows: H1 service and ports parsed from `--remote`, H2 whether
  the build ids match, H3 the remote snippet, H4 the ping, H5 whether the symlink exists, H6 the
  A2S result.
- Exit codes: 0 appended; 1 the log cannot be opened.
- Example: `cargo xtask debug direct-join-log --log /tmp/join.log --run-id r1 --client-build 1
  --server-build 1 --symlink missing --ping fail --a2s-json '{}'`

### direct-join

- Synopsis: `cargo xtask debug direct-join [<run-id>]`; the run id defaults to `user-repro`.
- Does: runs every probe above, writes their six rows to `debug-8fc1e0.log` in the checkout's
  `.cursor/`, and prints the build ids, the symlink and the remote section.
- Exit codes: 0 written; 1 the log cannot be written.
- Example: `cargo xtask debug direct-join`

## Boundaries

- Depends on: `crate::core::repository_root`, `crate::core::repository_layout::DEPLOY_ENV` and
  `crate::core::test_environment::PathGuard`; `verification_core` for runs, patterns and verdicts;
  `serde_json` and `regex`; ssh, sshpass and ping on the development machine.
- Used by: `tools_v2/xtask/src/cli/dispatch.rs`; `tools_v2/xtask/src/commands/mod_ops/dispatch.rs`,
  for `mod remote-logs`; `cargo xtask deploy staging`, which runs `mod remote-logs` last; people.
- Rules: a local probe never aborts the summary, and a missing Steam manifest reads `unknown`
  while a `buildid` line with two fields reads empty
  (`steam_two_field_buildid_is_empty_not_unknown` in `tests/direct_join/tests.rs`); documents name
  the host only as `TBD_SSH_HOST`.

## Related documentation

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  server these commands probe.
