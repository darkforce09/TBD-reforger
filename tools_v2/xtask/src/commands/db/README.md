# Database lane

The `cargo xtask db` group: the local Postgres container of the website
[API](/documentation_v2/glossary.md#api), its seeds and registry import, verified backups and
restores, the isolated integration-test run, the migration checksum repair and the lane's
self-test. Developers run it locally; the folder also holds the announcement seed that
`cargo xtask mod seed-announcement` runs.

## Contents

```text
tools_v2/xtask/src/commands/db/
├── milestone_announcement.rs  `mod seed-announcement`: inserts the pinned milestone announcement once
├── mod.rs                     the module tree
├── operations/                test-it, the checksum repair, the self-test and its comparison plumbing
├── operations.rs              the `DbCmd` clap enum, `run`, the compose lane, seeds, restore, import
└── tests/                     unit tests for the announcement seed and the lane's command names
```

## How it works

`tools_v2/xtask/src/cli/dispatch.rs` passes the parsed `DbCmd` to `operations::run`; this group
has no `cli.rs` or `dispatch.rs` of its own. Each command that shells out first prints the line it
runs, as a shell would echo it, then runs it and returns the child's exit code unchanged; a child
killed by a signal is reported as such and exits 1.

```text
DbCmd ──▶ operations::run
  ├─ up · down · logs · seed ─▶ <runtime> compose … in apps/website/api_v2 (service `db`)
  ├─ backup · restore · backup-drill · backup-verify ─▶ the deploy database commands, in process
  ├─ registry-import ─▶ cargo run --bin import-registry in apps/website/api_v2
  └─ test-it · repair-migration-checksum · selftest ─▶ operations/
```

The container runtime comes from `resolve_runtime` in
`tools_v2/xtask/src/commands/deploy/database_operations/`: `TBD_CONTAINER_RUNTIME` when set, then
`podman`, `docker`, and `distrobox-host-exec podman` or `docker` from inside a container; with none,
the command stops with `FATAL:` and exit 1. The echoed line names the runtime without the bridge
prefix, and `TBD_MK_TRACE=1` prints the real argv to stderr. `TBD_MK_WEB` points the compose
commands at another project folder, which the self-test uses so it never stops the shared
database. The compose file is `apps/website/api_v2/docker-compose.yml`, whose `db` service is the
`tbd_reforger_db` container on host port 5434.

`LANE_COMMANDS` in `operations.rs` repeats the command names for `cargo xtask help`, which prints
them as the database lane's index.

## Commands

Each runs as `cargo xtask db <command>`; `--help` on the group or a command prints clap's usage.
A clap usage error exits 2.

### up, down, logs

- Synopsis: `cargo xtask db up`, `cargo xtask db down`, `cargo xtask db logs`
- Does: `compose up -d db` starts Postgres in the background; `compose down` stops it and keeps
  the data volume; `compose logs -f db` follows the log and stays in the foreground until
  interrupted.
- Exit codes: the compose command's own code; 1 no container runtime.
- Example: `cargo xtask db up`

### seed

- Synopsis: `cargo xtask db seed`
- Does: applies `discord_roles.sql`, `registry_dev.sql`, `faction_library.sql`,
  `vehicle_database.sql` and `wiki_pages.sql` from `apps/website/api_v2/seeds/`, in that order
  (`registry_dev.sql` references the roles the first file seeds), each through
  `compose exec -T db psql -U tbd -d tbd_reforger`; it stops at the first failure. Needs `db up`.
- Exit codes: 0 all five applied; the failing `psql` run's code; 2 a seed file that cannot be
  opened.
- Example: `cargo xtask db seed`

### backup, backup-verify, backup-drill, restore

- Synopsis: `cargo xtask db backup [--db <name>] [--out <dir>] [--keep <n>]`;
  `cargo xtask db backup-verify --dump <file>`;
  `cargo xtask db backup-drill [--db <name>] [--out <dir>] [--fresh]`;
  `cargo xtask db restore --dump <file> --db <target> [--create]`
- Does: forwards to the same code as `cargo xtask deploy db backup`, `backup --verify-only`,
  `drill` and `restore`, in process: a verified `pg_dump -Fc` with retention by count (default
  database `tbd_reforger`, folder `~/tbd-backups/website`, 14 kept); a re-check of an existing
  dump; a restore of the newest dump into the scratch database `tbd_drill_probe` that proves it
  boots; and a restore that verifies the dump first. `db restore` accepts only a target on the
  scratch allow-list, since it passes no confirmation.
- Exit codes: 0 done; 1 a failed backup, verification, drill or refused target; 2 `restore`
  without both `--dump` and `--db`, or `backup-verify` without `--dump`, after printing the usage
  line.
- Example: `cargo xtask db backup-verify --dump ~/tbd-backups/website/<name>.dump`

### registry-import

- Synopsis: `cargo xtask db registry-import`
- Does: runs the API's `import-registry` binary in `apps/website/api_v2` over
  `contracts_v2/catalogs/registry-items.workbench.json` and
  `contracts_v2/catalogs/registry-compat.workbench.json`, loading the item
  [registry](/documentation_v2/glossary.md#registry) into the database `DATABASE_URL` names (the
  environment or `apps/website/api_v2/.env`), after applying pending migrations.
- Exit codes: cargo's own code.
- Example: `cargo xtask db registry-import`

### test-it

- Synopsis: `cargo xtask db test-it [--test <binary>]... [--lib] [<filter>]`
- Does: creates one database for this run, named from the label in `TBD_IT_BASE_DB` (default
  `rust_it`) plus random hex, runs the API's test suite against it with
  `cargo test --locked --no-fail-fast`, and drops the run's databases afterwards, whatever the
  tests did. A selection narrows the run and prints that it is no readiness receipt; the full
  suite runs without one. Needs `db up`.
- Exit codes: the test run's code when non-zero, else the cleanup's; 1 a label outside the scratch
  allow-list; 2 a malformed `--test` or filter.
- Example: `cargo xtask db test-it --test factions`

### repair-migration-checksum

- Synopsis: `cargo xtask db repair-migration-checksum [--version <n>] [--force]`
- Does: for each applied migration whose file hashes differently from the checksum recorded in
  `_sqlx_migrations`, recovers the applied bytes from git history and repoints the checksum when
  the two differ in comments only; `--version` limits it to one migration; `--force` repoints
  without the history proof.
- Exit codes: 0 nothing drifted, or every drift repaired; 1 a drift refused (a statement change,
  or bytes missing from history without `--force`), a `--version` that matches no applied row, or
  `_sqlx_migrations` unreadable (`xtask:` error).
- Example: `cargo xtask db repair-migration-checksum --version 21`

### selftest

- Synopsis: `cargo xtask db selftest`
- Does: runs six arms: the rendered recipes against a frozen baseline, two comparisons with a
  `Makefile` that report held when the checkout has none, the refusal of `tbd_reforger` as a
  test-it label, the cleanup of scratch suite databases, and the cleanup's failure when the
  container is down. Needs `db up`.
- Exit codes: 0 every arm held; 1 an arm failed; 2 an arm could not run.
- Example: `cargo xtask db selftest`

### mod seed-announcement

- Synopsis: `cargo xtask mod seed-announcement`, defined in the `mod` group and run from
  `milestone_announcement.rs`.
- Does: inserts the pinned "Milestone #1" website announcement unless one titled `Milestone #1%`
  exists. It takes `DATABASE_URL` from the environment, overridden by
  `apps/website/api_v2/.env` (read as `KEY=VALUE`, never executed), and runs `psql` with it; with
  no `psql`, it runs `podman exec -i tbdevent-postgres psql -U tbdevent -d tbdevent` when that
  container is running.
- Exit codes: 0 inserted or already present; 1 no `psql` and no such container, no
  `DATABASE_URL`, or an unreadable `.env`; `psql`'s own code; 127 a tool that is not installed.
- Example: `cargo xtask mod seed-announcement`

## Boundaries

- Depends on: `crate::commands::deploy` (the database helpers, backup, restore and drill);
  `crate::core::repository_root` and `crate::core::host_execution`;
  `crate::verifications::property_test_configuration`; `verification_core`; the compose file,
  seeds and migrations of `apps/website/api_v2/`; a container runtime, cargo and git.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs` and `tools_v2/xtask/src/commands/mod_ops/dispatch.rs`;
  - `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs`, which prints `LANE_COMMANDS`;
  - `verify wiki-seeds` and `verify faction-library-seeds`, which read `SEEDS`
    (`tools_v2/xtask/src/verifications/database/`);
  - the remote steps of `cargo xtask deploy website`, which run `db repair-migration-checksum`;
  - people, before `cargo xtask mk rust-api` and the integration tests.
- Rules: `LANE_COMMANDS` names exactly the `DbCmd` commands (`lane_commands_match_the_clap_enum`
  in `tests/operations/tests.rs`); `SEEDS` keeps its five files in order, and the two seed gates
  above check it; `tbd_reforger` is never a test-it label or a restore target
  (`is_safe_scratch_database_name` in the deploy helpers).

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — the database, the API and
  the app on a developer machine.
- [Database operations](/documentation_v2/runbooks/database_operations.md) — psql access, sample
  data, the integration tests, the checksum repair, backups, drills and restores.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the server database and
  its backups.
