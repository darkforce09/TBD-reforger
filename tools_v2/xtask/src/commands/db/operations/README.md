# Database lane operations

The three `cargo xtask db` commands with the most logic behind them: the isolated integration-test
run, the migration checksum repair and the lane's self-test, with the comparison plumbing the
self-test uses. The rest of the lane lives in `tools_v2/xtask/src/commands/db/operations.rs`, which
declares these modules.

## Contents

```text
tools_v2/xtask/src/commands/db/operations/
├── ab.rs                         the self-test's plumbing: bridged argv, make runs, scratch databases
├── recipes.rs                    the recipe lines the lane echoes, and a Makefile recipe reader
├── repair_migration_checksum.rs  `db repair-migration-checksum`: repoints comments-only edits
├── selftest.rs                   `db selftest`: six arms over the recipes, the guard and the cleanup
├── test_it.rs                    `db test-it`: one isolated database per run, then its cleanup
└── tests/                        unit tests for the plumbing, the recipes, the repair and test-it
```

## How it works

Every database call goes through the container helpers of
`tools_v2/xtask/src/commands/deploy/database_operations/`: `psql` runs inside the Postgres container
(`TBD_DB_CONTAINER`, default `tbd_reforger_db`) as `TBD_DB_USER` (default `tbd`), through
whichever runtime `resolve_runtime` finds. The maintenance database `IT_MAINT_DB` is
`tbd_reforger`, which the commands connect to and never write.

- `test_it::run` prints the property-test marker, validates the label in `TBD_IT_BASE_DB`
  (default `rust_it`) against the scratch allow-list (`rust_it`, `tbd_gate*`, `*_cold`, `*_it`,
  `*_probe`, never `tbd_reforger`), and claims a fresh database named
  `i<first five label characters>_<32 random hex>_it` with `CREATE DATABASE`, so a collision
  fails without touching another run's database. It runs
  `cargo test --locked --no-fail-fast [--lib] [--test <binary>]... -- --show-output [<filter>]` in
  `apps/website/api_v2` with `TEST_DATABASE_URL` on port 5434, `TBD_API_VERIFICATION=true` and the
  marker's `PROPTEST_RNG_SEED`. The cleanup then always runs, whatever the tests did: it selects
  the run's database and every `<name>_<suite>_it` the harness in
  `apps/website/api_v2/tests/common/database.rs` derived from it, re-checks each row's ownership
  and drops it with `FORCE`.
- `repair_migration_checksum::run` reads `_sqlx_migrations` from `TBD_DB_NAME` (default
  `tbd_reforger`). For each applied version whose file in `apps/website/api_v2/migrations/`
  hashes to another value than the recorded SHA-384, it searches `git log --all --follow` for the
  blob that matches, then compares the two with `--` comments and blank lines stripped,
  quote-aware. A comments-only difference is repointed with an `UPDATE`; a statement difference is
  refused; bytes missing from the history are refused unless `--force`, which repoints without
  the proof.
- `selftest::run` reports each arm through a `verification_core::Report`, where an arm that could
  not reach its subject ranks above a failure. Arm 1 compares `recipes::rendered_recipes` with a
  frozen literal; arms 2 and 6 compare the lane with `make` and report held when the checkout has
  no `Makefile`; arm 3 runs `TBD_IT_BASE_DB=tbd_reforger db test-it` and asserts the refusal and
  that `tbd_reforger` survives; arm 4 creates two scratch suite databases under a `tbd_gate` base
  and asserts the cleanup drops them and spares a bystander; arm 5 asserts the cleanup fails,
  where a piped shell loop would exit 0, when the container is down.

## Boundaries

- Depends on: `super` (`WEB`, `IT_BASE_DB`, `IT_MAINT_DB`, `echo`, `runtime`, `web`,
  `finish_status`); `crate::commands::deploy::database_operations` (`ct_capture`, `db_user`,
  `db_container`, `resolve_runtime`, `is_safe_scratch_database_name`, `database_exists`);
  `crate::core::host_execution` for the bridge; `crate::verifications::property_test_configuration`;
  `developer_tools::content_digest::sha384_hex`; `verification_core` for runs and verdicts; git,
  cargo and a container runtime.
- Used by: `run` in `tools_v2/xtask/src/commands/db/operations.rs`; the remote checksum step of
  `cargo xtask deploy website`, which runs `db repair-migration-checksum --force` against the
  staging container.
- Rules: a label outside the scratch allow-list is refused before any database is created
  (`the_guard_refuses_the_live_database_and_nonidentifiers` in `tests/test_it/tests.rs`); the
  cleanup drops only exact namespace matches (`cleanup_uses_an_exact_prefix_and_suffix`,
  `cleanup_rejects_prefix_lookalikes_and_malformed_query_rows`) and runs after every outcome
  (`cargo_spawn_error_still_runs_cleanup`); a statement change is never classed as comments-only
  (`a_changed_statement_is_never_comments_only`,
  `a_double_dash_inside_a_string_literal_is_not_a_comment` in
  `tests/repair_migration_checksum/tests.rs`); the frozen baseline covers every rendered recipe
  (`baseline_covers_every_rendered_target` in `tests/selftest/tests.rs`).

