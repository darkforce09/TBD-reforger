# Backup restore drill

The body of `cargo xtask deploy db drill` and `cargo xtask db backup-drill`: it restores a backup
into a scratch database and proves the result is recoverable and would let the API boot.
`tools/commands/database_operations/src/restore_drill.rs` declares both files, holds the guard
that drops the scratch database on exit and re-exports `run`.

## Contents

```text
tools/commands/database_operations/src/restore_drill/
├── execution.rs   `run`: flags, the dump pick, the restore, the table and migration audits, the usage
└── sha384_hex.rs  helpers: a file's SHA-384 through `content_digest`, psql queries, the scratch drop
```

## How it works

```text
run(args)
  ├─ refuse a scratch name off the allow-list; require the container, pg_restore, psql
  ├─ --fresh: take a backup first (deploy db backup)
  ├─ pick --dump, or the newest <out>/<db>-*.dump; none is a FATAL refusal
  ├─ drop the scratch database, then deploy db restore --create into it,
  │    expecting the dump to come from the source database
  ├─ count tables, enums, indexes and rows; no tables fails; a source that exists must have as many
  ├─ boot audit against the migration folder: every row of _sqlx_migrations must have a file,
  │    have succeeded and match the file's SHA-384; newer files are reported as pending
  └─ DRILL PASS (0) or DRILL FAIL (1); the scratch database is dropped unless --keep-scratch
```

The source database is `--db`, else `TBD_BACKUP_DB`, else `tbd_reforger`; the dump folder is
`--out`, else `TBD_BACKUP_DIR`, else `~/tbd-backups/website`; the scratch database is `--scratch`,
else `TBD_DRILL_DB`, else `tbd_drill_probe`. The migration folder is `TBD_GATE_MIGRATION_DIR`, else
`crates/api/api_database/migrations` of the checkout. A restore without `_sqlx_migrations` fails the
drill, or only warns with `--lax-migrations` or `TBD_DRILL_STRICT_MIGRATIONS=0`, because sqlx
would try to apply the first migration over the restored tables. The restore takes its workers from
`TBD_RESTORE_JOBS` and its row minimum from `TBD_RESTORE_MIN_ROWS`, both default 1.

## Boundaries

- Depends on: `crate::container_database` (the guard, the container helpers, row counts),
  `crate::backup` (`--fresh`) and `crate::restore` (`RestoreArgs`, `run`); `content_digest`
  (the migration files' SHA-384); `repository_layout`.
- Used by: `tools/commands/database_operations/src/container_database/execution.rs` (`deploy db
  drill`); `tools/commands/database_operations/src/local_database.rs` (`db backup-drill`); the weekly
  `tbd-website-backup-drill` unit in `deploy/systemd/`.
- Rules: the drill never restores into a database off the scratch allow-list, and passes no
  confirmation; a migration's version is its file name's leading digits without leading zeros
  (`mig_ver_strips_leading_zeros` in
  `tools/commands/database_operations/src/tests/restore_drill/tests.rs`).
