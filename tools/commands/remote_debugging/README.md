# Remote debugging

The `remote_debugging` crate, behind the `cargo xtask debug` and `cargo xtask repro` groups and
`cargo xtask mod remote-logs`: the tools a developer runs by hand when something fails away from
the checkout. The debug probes find out why a client cannot join the staging game server, the
remote log verdict grades the boot of the [mod](/documentation/glossary/g_to_m.md#mod) from the
staging server's `console.log`, and the reproduction replays large
[mission](/documentation/glossary/g_to_m.md#mission) version uploads against a local
[API](/documentation/glossary/a_to_f.md#api).

## Contents

```text
tools/commands/remote_debugging/
├── Cargo.toml  the `remote_debugging` library package: `clap`, `deploy_settings`, `deployment`, `process_runner`, `repository_layout`, layout tier 6
└── src/        the debug group, the remote log verdict, the repro group and the errors
```

## How it works

`tools/xtask/src/cli/mod.rs` mounts `DebugCmd` as the `debug` group and `ReproCmd` as the
`repro` group; `tools/xtask/src/cli/dispatch.rs` hands them to `remote_debugging::debug::run` and
`remote_debugging::reproduction::run`, and `tools/commands/mod_operations/src/mod_dispatch.rs` sends
`mod remote-logs` to `remote_debugging::debug::remote_logs::run`. Every entry returns the
command's exit code; an `Error` means the command could not run, and xtask prints it as
`xtask: <cause>` with exit 1.

- The staging host, its ssh credentials and the single server's profile folder come from
  `deploy/deploy.env` through `deploy_settings`; `--instance N` names fleet instance N as
  `cargo xtask deploy staging` deploys it, read through `deployment::staging::fleet_instances`,
  so a fleet the deploy refuses is refused here too.
- Every child process (ssh, sshpass, ping, curl) runs through `process_runner`; a probe that
  cannot reach the host records that in its row or its verdict and never fails the command.
- `src/debug/README.md`, `src/debug/remote_logs/README.md` and `src/reproduction/README.md` give
  each command's synopsis, behaviour and exit codes.

## Boundaries

- Depends on: `deploy_settings` (the settings file, the deploy host and its folders),
  `deployment` (the staging fleet's instances, ports, folders and units), `process_runner`,
  `repository_root` (the checkout root), `verification_core` (`NotRun`, patterns and probes),
  `time_source` (the NDJSON rows' timestamps), `clap`, `serde_json`, `regex` and `thiserror`;
  `tool_test_support` in tests.
- Used by: the `debug`, `repro` and `mod` groups of `tools/xtask`; `cargo xtask deploy staging`,
  which runs `mod remote-logs` last; people diagnosing a failed join, boot or upload.
- Rules:
  - No test reaches a real host, the staging fleet or a running API: the probes are driven
    through `run_with` on throwaway checkouts and homes, with no staging host or a `.invalid` one
    (`clean_empty_home_writes_unknown_and_missing`,
    `direct_join_instance_probes_its_own_unit_ports_and_profile`).
  - An unreadable or absent log is ENVIRONMENT, never a zero count
    (`missing_file_is_environment`).
  - The remote log vocabulary is shared by hand with
    `tools/commands/enfusion_mcp/src/workbench_logs.rs`.

## Getting started

Run from the repository root:

```bash
cargo test -p remote_debugging   # the probes, the verdicts and the token extraction, offline
```

## Related documentation

- [Command crates](/tools/commands/README.md) — the command crates and their tiers.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  server the debug commands probe.
- [Boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md)
  — running `mod remote-logs`, its four outcomes and the log lines it matches.
- [Local development](/documentation/runbooks/local_development.md) — the database and the API
  the upload reproduction runs against.
