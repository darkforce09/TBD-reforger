# Staging support commands

The harness's read-only commands: `preflight`, `status` with its `--capacity` table, and
`fingerprints`.

## Contents

```text
tools/commands/staging_procedures/src/support_commands/
├── fingerprints.rs   the source and configuration digests a recording started now binds to
├── host_capacity.rs  `status --capacity`: load average, memory, each unit's memory and CPU
├── mod.rs            the module tree
├── preflight.rs      `PreflightCheck`, the harness's own checks, and the met or unmet report
├── status.rs         the resting state and setup content beside their expected values
└── tests/            unit tests for preflight, status, capacity and fingerprints
```

## How it works

`preflight` runs the harness checks (run discipline, the repository root, the database's read-only
session, the API's `/healthz`, every fleet unit active, `staging-fixtures` and the relay binary on
the host), then the fleet and load procedures' own checks, and with `--discord` the Discord
procedure's. It refuses a list holding a command that is not a read before running any check, and
prints `met` or `UNMET` with the evidence or reason per check.

`status` prints two tables: the resting state (synthetic accounts 0, `[Load fixture]` events 0,
the API outage drop-in absent, the relay disarmed, every fleet unit active), which decides the exit
code, and the setup content (active servers, live missions with artifacts, fleet scenario rows,
ballistics catalogs) for reference. An unreadable item shows `unreadable (<why>)`.

`fingerprints` prints `source_sha256=` and `configuration_sha256=` from the acceptance verifier's
own fingerprints, so the values match what `RecordingSession::begin` snapshots.

## Boundaries

- Depends on: `remote_observers/`, `remote_actions/` (the drop-in state and the relay status),
  `api_readiness_checks::operational_recording::current_fingerprints`.
- Used by: `tools/commands/staging_procedures/src/staging_dispatch.rs`; the procedures' `preflight_checks`.
- Rules: nothing here changes the host.
