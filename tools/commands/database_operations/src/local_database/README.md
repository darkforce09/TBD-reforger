# Database lane operations

The three `cargo xtask db` commands with the most logic behind them: the isolated integration-test
run, the migration checksum repair and the lane's self-test, with the comparison plumbing the
self-test uses, the development compose project the compose commands run, and the recipe runner
every command echoes and spawns through. The command parse, the dispatch and the frozen recipe
constants live in `tools/commands/database_operations/src/local_database.rs`, which declares these
modules.

## Contents

```text
tools/commands/database_operations/src/local_database/
├── ab.rs                         the self-test's plumbing: bridged argv, make runs, scratch databases
├── api_test_packages.rs          the packages the API lanes cover: `api_server`, every `crates/api` member and their users, once each
├── development_compose.rs        the development compose project: folder, `-f` file, echoed line
├── recipe_execution.rs           the recipe runner: echo, API folder, runtime, exit status; compose, seed, registry import
├── recipes.rs                    the recipe lines the lane echoes, and a Makefile recipe reader
├── repair_migration_checksum.rs  `db repair-migration-checksum`: repoints comments-only edits
├── selftest.rs                   `db selftest`: six arms over the recipes, the guard and the cleanup
├── test_it.rs                    `db test-it`: one isolated database per run, then its cleanup
└── tests/                        unit tests for the checksum repair and test-it
```

## How it works

Every database call goes through the container helpers of
`tools/commands/database_operations/src/container_database/`: `psql` runs inside the Postgres container
(`TBD_DB_CONTAINER`, default `tbd_reforger_db`) as `TBD_DB_USER` (default `tbd`), through
whichever runtime `resolve_runtime` finds. The maintenance database `IT_MAINT_DB` is
`tbd_reforger`, which the commands connect to and never write.

- `recipe_execution` echoes each recipe line as make did, flushed before its child starts, with
  the runtime's logical name (`podman`, never the `distrobox-host-exec` prefix; `TBD_MK_TRACE=1`
  prints the real argv), spawns the child straight to the terminal and returns its own exit code,
  reporting a signalled child as a signal. `db up`, `db down`, `db logs`, `db seed` (each seed with
  `psql -v ON_ERROR_STOP=1`, stopping at the first failed file) and `db registry-import` run here.
  `db registry-import` runs `cargo run --bin import-item-registry` in the API crate folder, where
  the importer loads the developer's `deploy/api.env` from the checkout root, and names both envelopes by their absolute path from
  the repository root, never by a path that climbs out of that folder.
- `test_it::run` validates the label in `TBD_IT_BASE_DB`
  (default `rust_it`) against the scratch allow-list (`rust_it`, `tbd_gate*`, `*_cold`, `*_it`,
  `*_probe`, never `tbd_reforger`), and claims a fresh database named
  `i<first five label characters>_<32 random hex>_it` with `CREATE DATABASE`, so a collision
  fails without touching another run's database. It runs
  `cargo test --locked --no-fail-fast -p api_server -p <API crate>... -p <API crate user>... [--lib] [--test <binary>]... -- --show-output [<filter>]`
  in `crates/api/api_server` (a `--test` selection names `-p api_server` alone, the package that holds the
  integration binaries) with `TEST_DATABASE_URL` on port 5434 and `TBD_API_VERIFICATION=true`. The cleanup then always runs, whatever the tests did: it selects
  the run's database and every `<name>_<suite>_it` the harness in
  `crates/api/api_server/tests/common/database.rs`
  derived from it, re-checks each row's ownership
  and drops it with `FORCE`.
- `api_test_packages::api_test_packages` derives the API lanes' packages from the workspace:
  `api_server`, then every other member directly under `crates/api`, then every other member with
  a normal or development dependency on one of them (the staging fixtures tool in
  `tools/staging/staging_fixtures`, whose suites need the API's database), each group in
  member-path order and every package once (`api_server` itself sits under `crates/api`, and a
  package named twice would be built and tested twice), so an API crate and its users are tested,
  linted and built from the moment the root manifest names them. Its library-less package runs no case under `--lib`. An unreadable workspace or
  one without `api_server` is an error. The CI task table, the `mk` build lane and the wave gate's
  `test api` step derive their API lines through it.
- `development_compose::ComposeProject` names `deploy/compose.dev.yml` (`DEVELOPMENT_COMPOSE_FILE`
  in `repository_layout`) with `-f` and runs compose in `deploy/`, the folder its
  relative paths resolve against; it passes no `-p`, so the file's `name:` sets the project.
  `ComposeLine` renders the echoed `cd deploy && <runtime> compose -f compose.dev.yml …` line, and
  `recipes::rendered_recipes` renders the lane's lines through it. A seed read on stdin is shown,
  and opened, as `../crates/api/api_database/seeds/<file>` from that folder. `TBD_MK_WEB` swaps the folder for
  another one holding a `compose.dev.yml`.
- `repair_migration_checksum::run` reads `_sqlx_migrations` from `TBD_DB_NAME` (default
  `tbd_reforger`). For each applied version whose file in `crates/api/api_database/migrations/`
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

- Depends on: `super` (`WEB`, `SEEDS`, `seed_file`, `IT_BASE_DB`, `IT_MAINT_DB`); `repository_layout` (`DEVELOPMENT_COMPOSE_FILE`);
  `crate::container_database` (`ct_capture`, `db_user`, `db_container`,
  `resolve_runtime`, `is_safe_scratch_database_name`, `database_exists`);
  `process_runner::host_execution` for the bridge;
  `content_digest::sha384_hex`; `verification_core` for runs and verdicts; git,
  cargo and a container runtime.
- Used by: `run` in `tools/commands/database_operations/src/local_database.rs`, which runs the
  compose commands, `db seed` and `db registry-import` through `recipe_execution`; the ci `rust-test-it` task, which
  runs `test_it::run_complete_suite` in process; the remote checksum step of
  `cargo xtask deploy website`, which runs `db repair-migration-checksum --force` against the
  staging container.
- Rules: every compose call names the layout's file and runs in its folder, and a seed is opened through the path its line shows; a label outside the scratch allow-list
  is refused before any database is created
  (`the_guard_refuses_the_live_database_and_nonidentifiers` in `tests/test_it/tests.rs`); the
  cleanup drops only exact namespace matches (`cleanup_uses_an_exact_prefix_and_suffix`,
  `cleanup_rejects_prefix_lookalikes_and_malformed_query_rows`) and runs after every outcome
  (`cargo_spawn_error_still_runs_cleanup`); a statement change is never classed as comments-only
  (`a_changed_statement_is_never_comments_only`,
  `a_double_dash_inside_a_string_literal_is_not_a_comment` in
  `tests/repair_migration_checksum/tests.rs`); the frozen baseline covers every rendered recipe.

