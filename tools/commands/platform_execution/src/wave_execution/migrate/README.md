# Wave gate migration checks

The migration step of the platform wave gate: the persistent database that checks every migration
against a database that already holds rows.

## Contents

```text
tools/commands/platform_execution/src/wave_execution/migrate/
├── persist_feed.rs            psql feeds, applying one migration, seeding, file versions and sha384
└── persist_migration_step.rs  the persist-database step: checksum audit, pending migrations, re-seed
```

## How it works

`tools/commands/platform_execution/src/wave_execution/migrate.rs` states the persist design and
re-exports the step function.

- `gate_db_migrate_persist(mode)` works on the database `tbd_gate_migrate_persist`
  (`TBD_GATE_MIGRATE_PERSIST_DB`), which no run drops, with migrations from
  `crates/api/api_database/migrations/` (`TBD_GATE_MIGRATION_DIR`). It audits every applied
  migration's sha384 against the file on disk, then applies the pending ones through `psql`, one
  transaction per migration with its bookkeeping row. Last it re-applies
  `crates/api/api_database/seeds/content_golden.sql` and checks a population floor, including a
  claimed [ORBAT](/documentation/glossary/n_to_z.md#orbat) seat.
- `audit` mode, from `platform wave gate --migrate-persist audit`, rolls each pending migration
  back. `advance` mode, from the wave gate on merged `main` and
  `--migrate-persist advance` under the gate lock, commits them.

A missing `sha384sum` or `psql`, an unreachable database, no migration files, an applied version
with no file, or a failed migration record all fail the step; none reads as a skip.

## Boundaries

- Depends on: `super::host` (bridged `podman exec tbd_reforger_db psql`), `super::lock::GateState`,
  `sha384sum`, and the local Postgres container `tbd_reforger_db`.
- Used by: `super::gate` (both gate drivers) and the `gate --migrate-persist` dispatch in
  `tools/commands/platform_execution/src/wave_execution/flush.rs`.
- Rules: only merged `main` advances the persist database, so its state is always a prefix of
  `main`'s migrations; the persist step never drops its database; the tests are in
  `tools/commands/platform_execution/src/wave_execution/tests/migrate/tests.rs`.

## Related documentation

- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — the `TBD_GATE_MIGRATION_0016` setting
  both gates need.
