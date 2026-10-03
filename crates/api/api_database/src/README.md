# API database source

Opening the Postgres pool the [API](/documentation/glossary/a_to_f.md#api) runs on, applying the
schema migrations compiled into the binary, and telling a constraint violation apart from any
other database failure.

## Contents

```text
crates/api/api_database/src/
├── connection.rs       `connect`, `connect_lazy` and `migrate`, which applies the embedded migrations
├── connection_pool.rs  `DbPoolConfig`: the pool settings read from the four `TBD_DB_POOL_*` variables
├── error.rs            `Error`: a pool, migration or pool-setting failure, and `Result`
├── lib.rs              the crate root: module header, `mod` lines and the re-exports
├── postgres_errors.rs  SQLSTATE predicates: unique and foreign-key violations, the violated constraint
├── prelude.rs          the three lifecycle functions and the two violation predicates
└── tests/              unit tests for the pool settings and the connect guard
```

## How it works

`connect` reads the pool settings first: `TBD_DB_POOL_MAX_CONNECTIONS` (25, at least 1),
`TBD_DB_POOL_IDLE_TIMEOUT_SECS` (300), `TBD_DB_POOL_MAX_LIFETIME_SECS` (1800) and
`TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS` (30), where unset or blank means the default and any other
value must be a whole number. A value that is not one fails before any connection is tried, as a
`sqlx::Error::Configuration` wrapping the `ConfigError` that names the variable, so the `api` and
`import-registry` binaries and the integration suites all stop on it. `connect` then tries the
database up to 10 times, waiting 250 ms longer after each failure, because Postgres can refuse
connections just after it reports ready.

`migrate` runs `sqlx::migrate!("./migrations")`, a path the macro reads from the crate folder:
the SQL files of `crates/api/api_database/migrations/` are embedded when the crate compiles, and each run applies the
ones the database has not recorded in `_sqlx_migrations`, in version order. `connect_lazy` builds
a pool that connects on first use, with the default ceiling and none of the environment settings,
for tests and harnesses whose code paths never reach the database.

A handler that expects a constraint violation asks `postgres_errors` about the error it got
(`is_unique_violation`, `is_foreign_key_violation`, `violated_constraint`) and answers with a 4xx;
any other `sqlx::Error` becomes the 500 of `api_foundation::error_handling`.

## Boundaries

- Depends on: `sqlx` with its Postgres driver and `migrate` macro;
  `api_configuration::configuration::ConfigError`; the migration files in
  `crates/api/api_database/migrations/`, embedded at compile time.
- Used by:
  - `apps/api/src/bin/api.rs` and `apps/api/src/bin/import_registry.rs`,
    which `connect` and then `migrate`;
  - `postgres_errors`, in the match ingest parsing of `api_match_telemetry`, the version save of
    `api_missions` and the attachment of a [mission](/documentation/glossary/g_to_m.md#mission) to an
    [event](/documentation/glossary/a_to_f.md#event) in `api_operations`;
  - the integration suites under `apps/api/tests/`, which `connect`, `migrate` and
    `connect_lazy`.
- Rules: the pool settings are read here alone and never through `Config`; a malformed pool
  setting stops startup instead of falling back (`connect_refuses_a_non_numeric_pool_var_naming_it`
  in `tests/connection.rs`); a binary applies exactly the migrations it was compiled with.
