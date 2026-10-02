# Database lane operations

The three `cargo xtask db` commands with the most logic behind them: the isolated integration-test
run, the migration checksum repair and the lane's self-test, with the comparison plumbing the
self-test uses, and the development compose project the compose commands run. The rest of the
lane lives in `tools/xtask/src/commands/db/operations.rs`, which declares these modules.

## Contents

```text
tools/xtask/src/commands/db/operations/
├── ab.rs                         the self-test's plumbing: bridged argv, make runs, scratch databases
├── development_compose.rs        the development compose project: folder, `-f` file, echoed line
├── recipes.rs                    the recipe lines the lane echoes, and a Makefile recipe reader
├── repair_migration_checksum.rs  `db repair-migration-checksum`: repoints comments-only edits
├── selftest.rs                   `db selftest`: six arms over the recipes, the guard and the cleanup
├── test_it.rs                    `db test-it`: one isolated database per run, then its cleanup
└── tests/                        unit tests for the plumbing, compose, recipes, repair and test-it
```

## How it works

Every database call goes through the container helpers of
`tools/xtask/src/commands/deploy/database_operations/`: `psql` runs inside the Postgres container
(`TBD_DB_CONTAINER`, default `tbd_reforger_db`) as `TBD_DB_USER` (default `tbd`), through
whichever runtime `resolve_runtime` finds. The maintenance database `IT_MAINT_DB` is
`tbd_reforger`, which the commands connect to and never write.

- `test_it::run` prints the property-test marker, validates the label in `TBD_IT_BASE_DB`
  (default `rust_it`) against the scratch allow-list (`rust_it`, `tbd_gate*`, `*_cold`, `*_it`,
  `*_probe`, never `tbd_reforger`), and claims a fresh database named
  `i<first five label characters>_<32 random hex>_it` with `CREATE DATABASE`, so a collision
  fails without touching another run's database. It runs
  `cargo test --locked --no-fail-fast [--lib] [--test <binary>]... -- --show-output [<filter>]` in
  `apps/api` with `TEST_DATABASE_URL` on port 5434, `TBD_API_VERIFICATION=true` and the
  marker's `PROPTEST_RNG_SEED`. The cleanup then always runs, whatever the tests did: it selects
  the run's database and every `<name>_<suite>_it` the harness in
  `apps/api/tests/common/database.rs` derived from it, re-checks each row's ownership
  and drops it with `FORCE`.
- `development_compose::ComposeProject` names `deploy/compose.dev.yml` (`DEVELOPMENT_COMPOSE_FILE`
  in `crate::core::repository_layout`) with `-f` and runs compose in `deploy/`, the folder its
  relative paths resolve against; it passes no `-p`, so the file's `name:` sets the project.
  `ComposeLine` renders the echoed `cd deploy && <runtime> compose -f compose.dev.yml …` line, and
  `recipes::rendered_recipes` renders the lane's lines through it. A seed read on stdin is shown,
  and opened, as `../apps/api/seeds/<file>` from that folder. `TBD_MK_WEB` swaps the folder for
  another one holding a `compose.dev.yml`.
- `repair_migration_checksum::run` reads `_sqlx_migrations` from `TBD_DB_NAME` (default
  `tbd_reforger`). For each applied version whose file in `apps/api/migrations/`
  hashes to another value than the recorded SHA-384, it searches `git log --all --follow` for the
  blob that matches, then compares the two with `--` comments and blank lines stripped,
  quote-aware. A comments-only difference is repointed with an `UPDATE`; a statement difference is
  refused; bytes missing from the history are refused unless `--force`, which repoints without
  the proof.
- `selftest::run` reports each arm through a `verification_core::Report`, where an arm that could
  not reach its subject ranks above a failure. Arm 1 compares `recipes::rendered_recipes` with a
  frozen literal; arms 2 and 6 compare the lane with `make` (arm 6 over a throwaway compose project
  in `target/db-selftest`) and report held when the checkout has
  no `Makefile`; arm 3 runs `TBD_IT_BASE_DB=tbd_reforger db test-it` and asserts the refusal and
  that `tbd_reforger` survives; arm 4 creates two scratch suite databases under a `tbd_gate` base
  and asserts the cleanup drops them and spares a bystander; arm 5 asserts the cleanup fails,
  where a piped shell loop would exit 0, when the container is down.

## Boundaries

- Depends on: `super` (`WEB`, `SEEDS`, `seed_file`, `IT_BASE_DB`, `IT_MAINT_DB`, `echo`, `runtime`,
  `web`, `finish_status`); `crate::core::repository_layout` (`DEVELOPMENT_COMPOSE_FILE`);
  `crate::commands::deploy::database_operations` (`ct_capture`, `db_user`, `db_container`,
  `resolve_runtime`, `is_safe_scratch_database_name`, `database_exists`);
  `crate::core::host_execution` for the bridge; `crate::verifications::property_test_configuration`;
  `developer_tools::content_digest::sha384_hex`; `verification_core` for runs and verdicts; git,
  cargo and a container runtime.
- Used by: `run` in `tools/xtask/src/commands/db/operations.rs`; the ci `rust-test-it` task, which
  runs `test_it::run_complete_suite` in process; the remote checksum step of
  `cargo xtask deploy website`, which runs `db repair-migration-checksum --force` against the
  staging container.
- Rules: every compose call names the layout's file and runs in its folder
  (`the_checkout_project_runs_compose_on_the_layout_file_in_its_folder` in
  `tests/development_compose/tests.rs`), and a seed is opened through the path its line shows
  (`every_seed_is_opened_through_the_path_its_line_shows`); a label outside the scratch allow-list
  is refused before any database is created
  (`the_guard_refuses_the_live_database_and_nonidentifiers` in `tests/test_it/tests.rs`); the
  cleanup drops only exact namespace matches (`cleanup_uses_an_exact_prefix_and_suffix`,
  `cleanup_rejects_prefix_lookalikes_and_malformed_query_rows`) and runs after every outcome
  (`cargo_spawn_error_still_runs_cleanup`); a statement change is never classed as comments-only
  (`a_changed_statement_is_never_comments_only`,
  `a_double_dash_inside_a_string_literal_is_not_a_comment` in
  `tests/repair_migration_checksum/tests.rs`); the frozen baseline covers every rendered recipe
  (`baseline_covers_every_rendered_target` in `tests/selftest/tests.rs`).

