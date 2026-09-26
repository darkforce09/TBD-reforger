# Database migrations

The Postgres schema of the [API](/documentation_v2/glossary/a_to_f.md#api), as the ordered SQL migrations
that build it. The binaries embed this folder when they compile and apply what a database lacks
before they use it.

## Contents

```text
apps/website/api_v2/migrations/
└── *.sql  one schema migration each, `NNNN_<what_it_changes>.sql`, applied in version order
```

## How it works

`crate::core::database::migrate` runs `sqlx::migrate!("./migrations")`, which embeds every file of
this folder at compile time. The version of a file is its leading number, and sqlx applies the
versions a database has not recorded in its `_sqlx_migrations` table in ascending order, each in
its own transaction, recording each with the SHA-384 of the whole file. On every later run it
compares the recorded checksum of each applied version with the file's, comments included, and
refuses to start with `migration N was previously applied but has been modified` when they
differ.

`0001_initial_schema.sql` is the baseline: tables, enums, indexes and views. Every later file
changes that schema (tables, constraints, triggers, backfills) and never rewrites an earlier file.
No file holds versions 0022, 0023 and 0024, and none may.

## Format

- Encoding: UTF-8 Postgres SQL, one migration per file, named with a four-digit, zero-padded
  version and a lowercase snake_case summary (`0053_mission_deployments.sql`). Its `--` comments
  say what the migration does and why, and follow the crate's prose rules: no ticket ids, no
  delivery-process vocabulary, no paths to files the crate does not have.
- Schema: plain DDL and DML that sqlx runs in one transaction per file; no file opts out with
  sqlx's `-- no-transaction` marker. A migration must apply over a populated database, not only an
  empty one.
- Adding a file: give it the next version above the highest on disk, add its
  `(version, sha384)` row to `PINNED` in `apps/website/api_v2/tests/migrations_are_immutable.rs`
  (`sha384sum` prints the digest), and run `cargo xtask db test-it`. A comments-only edit to an
  applied file updates its pin and needs `cargo xtask db repair-migration-checksum` on every
  database that applied it; any other change to an applied file is a new migration instead.

## Producers and consumers

- Producers: developers, by hand; no tool writes these files.
- Consumers:
  - `crate::core::database::migrate` in `apps/website/api_v2/src/core/database/mod.rs`, called by
    the `api` binary at boot (unless `SKIP_MIGRATE` is set) and by `import-registry` before it
    imports;
  - the integration suites under `apps/website/api_v2/tests/`: the shared harness migrates each
    suite's scratch database, `migrations_are_immutable.rs` pins every file's checksum and the
    version sequence, `db_migrate.rs` pins the resulting schema, `durable_rate_limit.rs` pins
    `0021_rate_limit_buckets.sql` against `RATE_LIMIT_BUCKETS_DDL`, and the `*_migration.rs`
    suites check individual data migrations;
  - `cargo xtask db repair-migration-checksum`
    (`tools_v2/xtask/src/commands/db/operations/repair_migration_checksum.rs`), which repoints a
    recorded checksum only after proving from git history that the statements are unchanged;
  - the migration step of `cargo xtask platform wave gate`
    (`tools_v2/xtask/src/commands/platform/wave_execution/migrate.rs`), which audits the recorded
    checksums and applies pending migrations to a database it never drops;
  - `GET /healthz`, which turns red when `_sqlx_migrations` records a failed migration;
  - the prose rules in `apps/website/api_v2/src/tests/prose_rules.rs`, which read the comment
    lines.

## Boundaries

- Depends on: Postgres 18, the image `apps/website/api_v2/docker-compose.yml` runs locally.
- Used by: the consumers above; the seeds in `apps/website/api_v2/seeds/` and every query of the
  crate run against the schema these files build.
- Rules: an applied migration is never edited (`every_migration_on_disk_matches_its_pinned_checksum`)
  or deleted (`every_pin_still_has_its_migration_on_disk`); versions strictly increase, new ones
  append after the highest, and 0022 to 0024 stay empty
  (`migration_versions_preserve_the_historical_gap_and_append_after_the_head`), all in
  `apps/website/api_v2/tests/migrations_are_immutable.rs`.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — recovering a local
  database that refuses to start over a modified migration.
