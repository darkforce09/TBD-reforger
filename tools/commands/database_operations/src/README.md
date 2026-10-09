# Database operations source

The local database lane, the deploy-side backup, restore and drill, the container layer they
share, the database source checks, the announcement seed, and the errors they report.

## Contents

```text
tools/commands/database_operations/src/
├── backup.rs                  `deploy db backup`: a verified `pg_dump -Fc` with retention by count
├── container_database/        the runtime and container helpers, the allow-list and the dump verifier
├── container_database.rs      the `DeployDbCmd` clap enum; declares the helpers and re-exports them
├── database_checks/           the no-select-star gate
├── database_checks.rs         declares the database source gate
├── error.rs                   `Error` (with the operator stop) and `Result`, the context trait, `refuse!` and `stop!`
├── lib.rs                     the crate root: module header, `mod` lines and the re-exports
├── local_database/            test-it, the checksum repair, the self-test, the recipe runner (compose, seeds, import)
├── local_database.rs          the `DbCmd` clap enum, `run`, the frozen recipe constants, the restore wrapper
├── milestone_announcement.rs  `mod seed-announcement`: inserts the pinned milestone announcement once
├── prelude.rs                 `DbCmd`, `DeployDbCmd`, `Error` and `Result` for glob import
├── restore.rs                 `deploy db restore`: guard, verify, then `pg_restore --clean --if-exists`
├── restore_drill/             the drill: restore into a scratch database, then the table and boot audits
├── restore_drill.rs           the scratch-drop guard; declares the drill and re-exports `run`
└── tests/                     unit tests for backup, the container layer and the drill
```

## How it works

- `local_database::run` and `container_database::run` are the two command entries; each returns
  the command's exit code.
- `backup`, `restore` and `restore_drill` call the `container_database` helpers directly and one
  another in process (the drill's `--fresh` runs the backup, then the restore).
- `error`: an `Error::Stop` is an operator stop (`FATAL: …` or the restore refusal banner) that
  passes through every added context unchanged; the xtask binary prints it bare and exits 1.

## Boundaries

- Depends on: `process_runner`, `repository_layout`, `verification_core`, `content_digest`,
  `api_readiness_checks`, `clap`, `regex` and `thiserror`.
- Used by: the crate root's re-exports and public modules, read by the `db`, `verify`, `ci`, `mk`
  and `mod` groups of `xtask` and by the `deployment` crate.
