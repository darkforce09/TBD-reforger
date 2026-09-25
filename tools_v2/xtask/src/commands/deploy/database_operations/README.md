# Database container helpers

The shared layer of every database command: finding a container runtime, running Postgres tools
inside the database container, the scratch-database allow-list, and the five-check dump
verifier. `tools_v2/xtask/src/commands/deploy/database_operations.rs` declares both files, holds the
`DeployDbCmd` clap enum and re-exports the helpers.

## Contents

```text
tools_v2/xtask/src/commands/deploy/database_operations/
├── execution.rs    `run` for `deploy db`, the runtime and container, container exec, the target guard
└── verify_dump.rs  the five-check dump verifier, the COPY row counter, live row counts, database lookup
```

## How it works

Nothing here needs `psql`, `pg_dump` or `pg_restore` on the host: every call runs
`<runtime> exec [-i] <container> <tool> …`. `resolve_runtime` takes `TBD_CONTAINER_RUNTIME` when
set (split on spaces, its first word must be on `PATH`), then `podman`, then `docker`, then
`distrobox-host-exec podman` or `docker` when the host has them, and otherwise stops with `FATAL:`
and exit 1. The container is `TBD_DB_CONTAINER` (default `tbd_reforger_db`) and the role
`TBD_DB_USER` (default `tbd`). `require_container` refuses a container that is missing or not
running, and `require_pg_tool` a tool the image lacks, both naming `cargo xtask db up`.

`is_safe_scratch_database_name` admits `rust_it`, `tbd_gate*`, `*_cold`, `*_it` and `*_probe`, and
never `tbd_reforger`, the same list as `is_safe_test_database_name` in
`apps/website/api_v2/tests/common/database.rs`. `refuse_unsafe_restore_target` passes a plain ASCII
name on that list, or any name when the confirmation equals it, and otherwise prints the refusal
and exits 1.

`verify_dump` reads the file back through the container and fails with `VERIFY FAIL:` at the first
check that breaks:

1. the file exists, is a regular file and is not empty;
2. it starts with the `PGDMP` magic of `pg_dump -Fc`;
3. `pg_restore --list` reads its table of contents;
4. with an expected database, the archive header's `dbname:` matches it and the contents list
   `_sqlx_migrations`; with none, a `VERIFY NOTE` says the identity went unchecked;
5. `pg_restore --data-only` reads the whole body, and its COPY blocks hold at least the minimum
   rows; a truncated or corrupt file passes the table of contents and fails here.

## Boundaries

- Depends on: `verification_core` (`Pattern`, `gate`); a container runtime and the Postgres
  container that `cargo xtask db up` starts, or the one `TBD_DB_CONTAINER` names.
- Used by: `run` in `tools_v2/xtask/src/commands/deploy/dispatch.rs`; `database_backup.rs`,
  `database_restore.rs` and `database_restore_drill/` beside this folder; the `db` lane in
  `tools_v2/xtask/src/commands/db/`, for its runtime, container exec and allow-list.
- Rules: `tbd_reforger` is never on the allow-list
  (`safe_scratch_allow_list_admits_scratch_names_and_refuses_the_live_database` in
  `tools_v2/xtask/src/commands/deploy/tests/database_operations/tests.rs`); the row count reads COPY
  data lines only (`count_copy_rows_counts_data_not_headings`); the allow-list matches the
  integration harness's, which no test compares, so a change is made in both places.
