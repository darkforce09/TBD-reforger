# Staging console log reader

The body of `cargo xtask mod remote-logs`: it fetches the newest `console.log` of the staging game
server over ssh, or reads a local one, and grades the boot of the
[mod](/documentation_v2/glossary.md#mod) with one of four exit codes. The log patterns sit in
`tools_v2/xtask/src/commands/debug/remote_logs.rs`, which declares both files and re-exports `run`.

## Contents

```text
tools_v2/xtask/src/commands/debug/remote_logs/
├── execution.rs    `run`, the verdict over one log, the remote fetch, the self-test and the ssh call
└── shell_quote.rs  the remote script's quoting, the deploy.env reader, temp folders and fixture writers
```

## How it works

`run(file, selftest)` takes one of three paths. `--selftest` writes five fixture logs to a temp
folder and checks each verdict. `--file <log>` grades that file. With neither, `cmd_remote` takes
`TBD_SSH_HOST`, `TBD_PROFILE_DIR`, `TBD_SSH_PASS` and `TBD_SSH_IDENTITY_FILE` from the environment
first and `tools_v2/xtask/deploy/deploy.env` second, finds the newest `console.log` under the
profile's `logs/` or `profile/logs/` over ssh (sshpass with a password, `-i` with an identity file),
copies it to a temp file and grades that.

The verdict reads the log once. It prints the last 80 `[TBD]` and error lines, counts `[TBD][`
tagged lines (none is a stale build; fewer than `TBD_MIN_TAGGED`, default 20, only warns), requires
the mission-loaded, slot and LOBBY lines, fails on compile, unknown-class or spawn errors, and notes
the loadout lines. It exits 0 HEALTHY when a player was assigned a slot, 2 PARTIAL when the boot is
healthy and nobody has joined, 1 FAIL otherwise, and 3 ENVIRONMENT when no log could be read. A
missing `TBD_SSH_HOST` or `TBD_PROFILE_DIR` exits 1.

## Boundaries

- Depends on: the patterns, `SshOut` and module wiring in
  `tools_v2/xtask/src/commands/debug/remote_logs.rs`; `verification_core` (`Pattern`,
  `gate::probe_str`, `proc::Run`); `crate::core::repository_root` and
  `crate::core::repository_layout::DEPLOY_ENV`; ssh or sshpass.
- Used by: `tools_v2/xtask/src/commands/mod_ops/dispatch.rs` (`mod remote-logs`); the last step of
  `cargo xtask deploy staging`, which runs `mod remote-logs` and reads 2 as a pass.
- Rules: an unreadable or absent log is ENVIRONMENT, never a zero count
  (`missing_file_is_environment` in `tools_v2/xtask/src/commands/debug/tests/remote_logs/tests.rs`);
  a log without tagged lines fails (`stale_fails`); a healthy boot without a player is PARTIAL
  (`healthy_is_partial`); the pattern vocabulary is shared by hand with
  `tools_v2/xtask/src/commands/mcp/workbench_logs.rs`, and a change here is made there too.
