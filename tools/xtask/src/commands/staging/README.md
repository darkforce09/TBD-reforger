# Staging acceptance harness

`cargo xtask staging`: the harness that records the three operational receipts of the API
acceptance register (`staging_fleet`, `staging_discord`, `staging_load`) against the staging host,
and the read-only and confirmed commands the runbooks run around those recordings. The design, the
witness rules and the case lists are in the
[staging design note](/documentation/website/api_v2/verification_evidence/staging.md); the
procedures to follow are in the
[staging verification runbooks](/documentation/runbooks/staging_verification/README.md).

## Contents

```text
tools/xtask/src/commands/staging/
├── cli.rs                  the `StagingCmd` clap tree: read-only, confirmed and recorded subcommands
├── discord_procedure/      the `staging_discord` procedure
├── dispatch.rs             routes each subcommand; runs confirmed actions and their `--dry-run` plans
├── environment_identity/   the host, build and load generator identities a receipt records
├── fleet_procedure/        the `staging_fleet` procedure
├── load_procedure/         the `staging_load` procedure and the local rehearsal
├── mod.rs                  the module tree
├── observation_journal/    the run's JSONL journal, raw artifacts by SHA-256, and the browser inbox
├── operator_coordination/  the numbered action lists and the `AWAIT` and outcome lines
├── procedure_runner/       the step vocabulary, the plan checks, the runner and the recorded run
├── remote_actions/         every command that changes the host: host tool, drop-in, relay, backup, update
├── remote_observers/       every read of the host and the ssh transport that runs it
├── run_identity.rs         the run folder `target/staging/<check>/<run>/` and the recorded command line
├── staging_settings.rs     the settings read from `deploy.env`, the fleet and API origin shared with `deploy staging`
├── support_commands/       `preflight`, `status` (and `--capacity`) and `fingerprints`
└── tests/                  unit tests for the settings
```

## How it works

| Subcommand | Kind | What it does |
|---|---|---|
| `preflight [--discord]` | read-only | the harness checks, then the fleet and load procedures' checks (`--discord` adds the Discord procedure's); met or unmet each |
| `status [--capacity]` | read-only | resting state and setup content beside their expected values; `--capacity` prints load, memory and each unit's `MemoryCurrent` and `CPUUsageNSec` |
| `fingerprints` | read-only | the source and configuration digests a recording started now binds to |
| `action-list <fleet\|discord\|load> [--cases\|--recovery]` | read-only | the procedure's numbered real actions, its declared cases, or its recovery actions |
| `backup --label <label>` | confirmed | a verified `pg_dump -Fc` under `~/tbd/backups/<date>/`; the answer must name the file |
| `update-game-server` | confirmed | steamcmd `app_update 1890870 validate` into `TBD_SERVER_DIR`, refused while a fleet game server runs; the answer must name the build |
| `provision-fleet` | confirmed | `staging-fixtures provision-fleet` for the fleet, with the operator as actor |
| `rotate-credential --instance N --executor host_agent\|mod_runtime --stage\|--promote` | confirmed | one half of a machine credential rotation |
| `seed-load --token-file <path>`, `clean-load` | confirmed | the synthetic population and the `[Load fixture]` events, seeded (the account file moved into the mode-600 token file) or cleaned |
| `fleet --record`, `discord --record`, `load --record --token-file <path>` | recorded | runs the procedure and writes its receipt |
| `load --rehearse-local` | local | the load path against the local stack; records nothing |

Every command but `fingerprints` and `load --rehearse-local` reads `deploy.env` (`TBD_SSH_HOST`,
`TBD_SSH_PASS` or `TBD_SSH_IDENTITY_FILE`, the `TBD_FLEET_*` keys, `TBD_STAGING_DB_CONTAINER`,
`TBD_STAGING_OPERATOR_DISCORD_ID`, `TBD_STAGING_PARTNER_{GUILD,ROLE}_ID`,
`TBD_LOAD_TARGET_ORIGIN`, `TBD_LOAD_SOURCE_ADDRESSES`) and reaches the host over ssh, one process
per command. A confirmed action runs when invoked, after the operator approved it over the action
list; `--dry-run` prints the command line and script instead and opens no connection.

A recorded run validates its plan (a plan the recorder cannot judge is refused before the earlier
receipt is touched), calls `RecordingSession::begin`, creates `target/staging/<check>/<run>/`
with `journal.jsonl`, `artifacts/` and `browser_inbox/`, reads the environment identities and the
procedure's fixture identities into the manifest, runs the steps, and hands the outcome to
`RecordingSession::finish`, which writes `target/api-readiness/<check>.{log,fixture.json,json}`.
While it runs it prints `AWAIT <step>: <instruction>` per step and never reads stdin.

Exit codes: 0 when every check is met, every resting item holds, the action succeeded, or the
recording passed; 1 otherwise; an error (no `deploy.env`, a refused plan) prints `xtask: <cause>`
and exits 1.

## Boundaries

- Depends on: `tools/xtask/src/core/` (`deploy_environment`, `secure_shell_transport`,
  `repository_root`); `crate::commands::deploy::staging::fleet_instances` for the fleet, its
  instance folders and units, and the API origin (`backend_url`);
  `crate::verifications::api_readiness::operational_recording` for the receipts; `ssh` (or
  `sshpass`) here, and bash, systemd user units, docker, curl and steamcmd on the host.
- Used by: `TopCmd::Staging` in `tools/xtask/src/cli/`; the staging verification runbooks.
- Rules: no secret enters an argument vector, a script, the journal or the receipt (the ssh
  password travels in `SSHPASS`, the `/metrics` bearer is read on the host and piped to curl);
  the read-only commands and every probe send only read commands; the unit tests are
  `cargo test -p xtask --locked commands::staging::`.
