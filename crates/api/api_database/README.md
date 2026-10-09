# API database

The `api_database` crate: the Postgres connection lifecycle of the
[API](/documentation/glossary/a_to_f.md#api) and the schema it runs on. It opens the tuned pool,
applies the migrations embedded from `migrations/`, classifies a failed query by its SQLSTATE, and
keeps the development seeds `cargo xtask db seed` applies.

## Contents

```text
crates/api/api_database/
├── Cargo.toml   the package: `sqlx` with Postgres and migrations, `api_configuration`, layout tier 3
├── migrations/  the SQL schema migrations, embedded at compile time and applied at boot
├── seeds/       the development seeds `cargo xtask db seed` applies, and hand-applied data
└── src/         the pool, the migration run, the pool settings and the SQLSTATE predicates
```

## How it works

`connect` reads the four `TBD_DB_POOL_*` settings, refusing a malformed one before any connection
attempt, then tries the database up to 10 times with a growing wait. `migrate` applies the
migrations `sqlx::migrate!("./migrations")` embedded from this crate's folder when it compiled,
each version once, in order, refusing to start when an applied file has changed. A handler that
expects a constraint violation asks `postgres_errors` and answers with a 4xx instead of a 500.

The seeds are not compiled in: `cargo xtask db seed` pipes five of them into the development
database in a fixed order, and a few integration suites embed one with `include_str!`.

## Getting started

Run from the repository root:

```bash
cargo test -p api_database   # the pool settings and the connect guard, no database needed
cargo xtask db seed          # apply the development seeds once the API has migrated the database
```

A schema change is a new file in `migrations/` after the highest version; an applied file is
never edited.

## Configuration

The pool reads `TBD_DB_POOL_MAX_CONNECTIONS` (25), `TBD_DB_POOL_IDLE_TIMEOUT_SECS` (300),
`TBD_DB_POOL_MAX_LIFETIME_SECS` (1800) and `TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS` (30) when `connect`
opens it; unset or blank means the default. No feature.

## Public surface

- `connect`, `connect_lazy`, `migrate`, at the crate root and in `prelude`.
- `connection_pool`: `DbPoolConfig`, `pool_options` and the four variable names.
- `postgres_errors`: `is_unique_violation`, `is_foreign_key_violation`, `violated_constraint`.
- `Error` and `Result`.

## Boundaries

- Depends on: `api_configuration` (`ConfigError`, which names a malformed pool setting), `sqlx`
  and `tokio`; Postgres 18.
- Used by: the API's `api-server` and `import-item-registry` binaries, its services that map constraint
  violations, and its integration suites; `cargo xtask db seed`,
  `cargo xtask db repair-migration-checksum` and the wave migration gate read the SQL folders.
- Rules: an applied migration never changes (`crates/api/api_server/tests/http_infrastructure/migrations_are_immutable.rs`).

## Related documentation

- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
- [Database operations](/documentation/runbooks/database_operations.md) — backups, restores and
  the migration checksum repair.
