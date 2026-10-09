# Staging procedures

The `staging_procedures` crate: the `cargo xtask staging` group. It records the three staging
receipts (`staging_fleet`, `staging_discord`, `staging_load`) against the staging host, and runs the read-only and confirmed commands the runbooks run around
those recordings. The design, the witness rules and the case lists are in the
[staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md); the procedures
to follow are in the
[staging verification runbooks](/documentation/runbooks/staging_verification/README.md).

## Contents

```text
tools/commands/staging_procedures/
├── Cargo.toml  the `staging_procedures` library package: `deployment`, `deploy_settings`, `process_runner`, `staging_load_plan`, layout tier 6
└── src/        the command line, its routing, the settings, the procedure engine, the observers, the host actions and the three procedures
```

## How it works

`tools/xtask/src/cli/dispatch.rs` passes the parsed `StagingCmd` to `staging_procedures::run`,
which returns the exit code; an `Error` prints as `xtask: <cause>` and exits 1.

| Subcommand | Kind | What it does |
|---|---|---|
| `preflight [--discord]` | read-only | the harness checks, then the fleet and load procedures' checks (`--discord` adds the Discord procedure's); met or unmet each |
| `status [--capacity]` | read-only | resting state and setup content beside their expected values; `--capacity` prints load, memory and each unit's `MemoryCurrent` and `CPUUsageNSec` |
| `action-list <fleet\|discord\|load> [--cases\|--recovery]` | read-only | the procedure's numbered real actions, its declared cases, or its recovery actions |
| `backup --label <label>` | confirmed | a verified `pg_dump -Fc` under `~/tbd/backups/<date>/`; the answer must name the file |
| `update-game-server` | confirmed | steamcmd `app_update 1890870 validate` into `TBD_SERVER_DIR`, refused while a fleet game server runs; the answer must name the build |
| `provision-fleet` | confirmed | `staging-fixtures provision-fleet` for the fleet, with the operator as actor |
| `rotate-credential --instance N --executor host_agent\|mod_runtime --stage\|--promote` | confirmed | one half of a machine credential rotation |
| `seed-load --token-file <path>`, `clean-load` | confirmed | the synthetic population and the `[Load fixture]` events, seeded (the account file moved into the mode-600 token file) or cleaned |
| `fleet --record`, `discord --record`, `load --record --token-file <path>` | recorded | runs the procedure and writes its receipt |
| `load --rehearse-local` | local | the load path against the local stack; records nothing |

Every command but `load --rehearse-local` reads `deploy.env` (`TBD_SSH_HOST`,
`TBD_SSH_PASS` or `TBD_SSH_IDENTITY_FILE`, the `TBD_FLEET_*` keys, `TBD_STAGING_DB_CONTAINER`,
`TBD_STAGING_OPERATOR_DISCORD_ID`, `TBD_STAGING_PARTNER_{GUILD,ROLE}_ID`,
`TBD_LOAD_TARGET_ORIGIN`, `TBD_LOAD_SOURCE_ADDRESSES`) and reaches the host over ssh, one process
per command. A confirmed action runs when invoked, after the operator approved it over the action
list; `--dry-run` prints the command line and script instead and opens no connection.

A recorded run validates its plan (a plan the recorder cannot judge is refused before the earlier
receipt is touched), calls `RecordingSession::begin`, creates `target/staging/<check>/<run>/`
with `journal.jsonl`, `artifacts/` and `browser_inbox/`, reads the environment identities and the
procedure's fixture identities into the manifest, runs the steps, and hands the outcome to
`RecordingSession::finish`, which judges the outcome (every declared case ok, the check's
minimum of passing cases, the acceptance thresholds, the two-hour limit) and writes
`target/staging/receipts/<check>.{log,fixture.json,json}`.
While it runs it prints `AWAIT <step>: <instruction>` per step and never reads stdin. The load
procedure reads the committed workload and population in `tools/xtask/staging/` through
`repository_layout::tool_inputs` and runs the load generator as the `staging-load` binary of
`developer_tools` (plan JSON on stdin, report JSON on stdout).

Exit codes: 0 when every check is met, every resting item holds, the action succeeded, or the
recording passed; 1 otherwise; an error (no `deploy.env`, a refused plan) prints `xtask: <cause>`
and exits 1.

## Boundaries

- Depends on: `deployment` (`staging::fleet_instances` for the fleet, its instance folders and units, and the
  API origin), `deploy_settings` (the settings and the deploy host), `process_runner`
  (`secure_shell_transport`, `Run`), `staging_load_plan` (the load plan and report),
  `repository_layout`, `content_digest`, `time_source`, `clap`, `serde`, `serde_json`,
  `thiserror`; `ssh` (or `sshpass`) and
  curl here, and bash, systemd user units, docker, curl and steamcmd on the host.
- Used by: `TopCmd::Staging` in `tools/xtask/src/cli/`; the staging verification runbooks.
- Rules: tier 6 of `tools/commands` (`cargo xtask verify crate-tiers`); no tokio, axum or reqwest
  in its dependency tree (the load generator is a subprocess); no secret enters an argument
  vector, a script, the journal or the receipt (the ssh password travels in `SSHPASS`, the
  `/metrics` bearer is read on the host and piped to curl); the read-only commands and every probe
  send only read commands.
