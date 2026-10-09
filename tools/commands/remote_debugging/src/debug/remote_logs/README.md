# Staging console log reader

The body of `cargo xtask mod remote-logs`: it fetches the newest `console.log` of a staging game
server over ssh, the single server's or one fleet instance's, or reads a local one, and grades the boot of the
[mod](/documentation/glossary/g_to_m.md#mod) with one of four exit codes. The log patterns sit in
`tools/commands/remote_debugging/src/debug/remote_logs.rs`, which declares the three files and re-exports
`run`.

## Contents

```text
tools/commands/remote_debugging/src/debug/remote_logs/
├── execution.rs     `run`, the verdict over one log and the self-test
├── remote_fetch.rs  the log source (`--instance` or the single server), the newest-log script, the fetch and the ssh call
└── shell_quote.rs   the remote script's quoting, temp folders and fixture writers
```

## How it works

`run(file, selftest, instance)` takes one of three paths; `--instance` with `--file` or
`--selftest` is ENVIRONMENT, because nothing is fetched. `--selftest` writes five fixture logs to a temp
folder and checks each verdict. `--file <log>` grades that file. With neither, `cmd_remote` takes
`TBD_SSH_HOST`, `TBD_PROFILE_DIR` (default `/home/<user>/tbd/profile`), `TBD_SSH_PASS` and
`TBD_SSH_IDENTITY_FILE` from `deploy/deploy.env` (or the file `DEPLOY_ENV` names)
through `deploy_settings`: the file decides every key it assigns and the
environment fills the rest. With `--instance N` (1 to `TBD_FLEET_INSTANCES`, default 5) the
profile is fleet instance N's, `~/tbd/fleet/instance-N/profile`, and `TBD_PROFILE_DIR` is not read;
the instance comes from `cargo xtask deploy staging`'s own fleet settings, so fleet settings the
deploy refuses exit 1 as a refused setting, and an instance outside the fleet is ENVIRONMENT.
Without it the remote script first exits 10 when the host holds `~/tbd/fleet`, and the command
refuses with ENVIRONMENT and a message naming `--instance`, so a fleet host is never graded from
a retired single-server folder. It finds the
newest `console.log` under the profile's `logs/` or `profile/logs/` over ssh (sshpass with a
password, `-i` with an identity file), copies it to a temp file and grades that.

The verdict reads the log once. It prints the last 80 `[TBD]` and error lines, counts `[TBD][`
tagged lines (none is a stale build; fewer than `TBD_MIN_TAGGED`, default 20, only warns), requires
the mission-loaded, slot and LOBBY lines, fails on compile, unknown-class or spawn errors, and notes
the loadout lines. It exits 0 HEALTHY when a player was assigned a slot, 2 PARTIAL when the boot is
healthy and nobody has joined, 1 FAIL otherwise, and 3 ENVIRONMENT when no log could be read. A
missing or malformed setting (no `TBD_SSH_HOST`, a profile folder with no user to default it
under, a `deploy.env` that does not load) exits 1.

## Boundaries

- Depends on: the patterns, `SshOut` and module wiring in
  `tools/commands/remote_debugging/src/debug/remote_logs.rs`; `verification_core` (`Pattern`,
  `gate::probe_str`); `process_runner::Run`; `deploy_settings` and, through
  `tools/commands/remote_debugging/src/debug/staging_fleet_instance.rs`,
  `deployment::staging::fleet_instances`; ssh or sshpass.
- Used by: `tools/commands/mod_operations/src/mod_dispatch.rs` (`mod remote-logs`), through
  `remote_debugging::debug::remote_logs::run`; the last step of
  `cargo xtask deploy staging`, which runs `mod remote-logs` and reads 2 as a pass.
- Rules: an unreadable or absent log is ENVIRONMENT, never a zero count;
  a log without tagged lines fails; a healthy boot without a player is PARTIAL; `--instance N` reads only instance N's profile, and a fleet host without `--instance` is
  refused; the pattern vocabulary is shared by hand with
  `tools/commands/enfusion_mcp/src/workbench_logs.rs`, and a change here is made there too.
