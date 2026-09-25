# Wave gate migration checks

The two migration steps of the platform wave gate: a pin on the claim body of migration 0016, and
the persistent database that checks every migration against a database that already holds rows.

## Contents

```text
tools_v2/xtask/src/commands/platform/wave_execution/migrate/
├── gate_db_migrate_claim_body.rs  the 0016 claim-body pin, SQL comment stripping, the persist-database step
└── persist_feed.rs                psql feeds, applying one migration, seeding, file versions and sha384
```

## How it works

`tools_v2/xtask/src/commands/platform/wave_execution/migrate.rs` states the persist design and
re-exports the two step functions.

- `gate_db_migrate_claim_body` reads migration 0016, strips SQL comments and requires the claim
  `UPDATE` of `public.match_player_stats` and its conditions to be present. Its default path is a
  0016 file name that the migrations folder does not hold (the tracked file is
  `apps/website/api_v2/migrations/0016_backfill_linked_match_stats.sql`). A missing file fails the
  step, so the step fails unless `TBD_GATE_MIGRATION_0016` names the tracked file.
- `gate_db_migrate_persist(mode)` works on the database `tbd_gate_migrate_persist`
  (`TBD_GATE_MIGRATE_PERSIST_DB`), which no run drops, with migrations from
  `apps/website/api_v2/migrations/` (`TBD_GATE_MIGRATION_DIR`). It audits every applied
  migration's sha384 against the file on disk, then applies the pending ones through `psql`, one
  transaction per migration with its bookkeeping row. Last it re-applies
  `apps/website/api_v2/seeds/content_golden.sql` and checks a population floor, including a
  claimed [ORBAT](/documentation_v2/glossary.md#orbat) seat.
- `audit` mode, from the slice gate, and `platform wave gate --migrate-persist audit` roll each
  pending migration back. `advance` mode, from the wave gate on merged `main` and
  `--migrate-persist advance` under the gate lock, commits them.

A missing `sha384sum` or `psql`, an unreachable database, no migration files, an applied version
with no file, or a failed migration record all fail the step; none reads as a skip.

## Boundaries

- Depends on: `super::host` (bridged `podman exec tbd_reforger_db psql`), `super::lock::GateState`,
  `sha384sum`, and the local Postgres container `tbd_reforger_db`.
- Used by: `super::gate` (both gate drivers) and the `gate --migrate-persist` dispatch in
  `tools_v2/xtask/src/commands/platform/wave_execution/flush.rs`.
- Rules: only merged `main` advances the persist database, so its state is always a prefix of
  `main`'s migrations; the persist step never drops its database; the tests are in
  `tools_v2/xtask/src/commands/platform/wave_execution/tests/migrate/tests.rs`.
