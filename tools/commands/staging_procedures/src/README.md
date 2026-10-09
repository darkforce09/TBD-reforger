# Staging procedures source

The command line and routing of `cargo xtask staging`, the settings, the run identity, the
procedure engine, the journal and browser inbox, the observers, the environment identity, the
operator coordination, the host actions, the read-only support commands, the three procedures and
the errors they report. The commands, settings and exit codes are in the
[crate README](/tools/commands/staging_procedures/README.md).

## Contents

```text
tools/commands/staging_procedures/src/
├── discord_procedure/      the `staging_discord` procedure
├── environment_identity/   the host, build and load generator identities a receipt records
├── error.rs                `Error`, `Result`, the refusal macros and the step context
├── fleet_procedure/        the `staging_fleet` procedure
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── load_procedure/         the `staging_load` procedure and the local rehearsal
├── observation_journal/    the run's JSONL journal, raw artifacts by SHA-256, and the browser inbox
├── operator_coordination/  the numbered action lists and the `AWAIT` and outcome lines
├── prelude.rs              `StagingCmd`, its value types, `run`, `Error` and `Result` for glob import
├── procedure_runner/       the step vocabulary, the plan checks, the runner and the recorded run
├── remote_actions/         every command that changes the host: host tool, drop-in, relay, backup, update
├── remote_observers/       every read of the host and the ssh transport that runs it
├── run_identity.rs         the run folder `target/staging/<check>/<run>/` and the recorded command line
├── staging_command.rs      the `StagingCmd` clap tree: read-only, confirmed and recorded subcommands
├── staging_dispatch.rs     `run`: routes each subcommand; runs confirmed actions and their `--dry-run` plans
├── staging_settings.rs     the settings read from `deploy.env`, the fleet and API origin shared with `deploy staging`
└── support_commands/       `preflight`, `status` (and `--capacity`) and `fingerprints`
```

## How it works

- `staging_dispatch::run` finds the checkout, answers `fingerprints`, `action-list` and
  `load --rehearse-local` without a host, and otherwise loads `StagingSettings`, opens one
  `HostShell` and routes the command: preflight and status read, a confirmed action runs its
  `RemoteCommand` (or prints it under `--dry-run`), a recorded run hands the procedure to
  `procedure_runner::recording::record`.
- Each procedure implements `StagingProcedure`: its plan of steps and declared cases, its action
  and recovery lists, its preflight checks and its fixture identities; the runner drives the
  steps through the observers and the journal, and the recorder writes the receipt.
- `error.rs` keeps every printed text: a step context displays `<step>: <cause>` with no source,
  so the verdicts, manifests and the binary's `xtask: <cause>` line read as one sentence.

## Boundaries

- Depends on: the crates the [crate README](/tools/commands/staging_procedures/README.md) lists.
- Used by: the xtask binary through `run` and `StagingCmd`.
- Rules: the read-only commands and every probe send only read commands.
