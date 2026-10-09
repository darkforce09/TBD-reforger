# Database operations

The `database_operations` crate: the database side of the repository tooling. It runs the
`cargo xtask db` group (the local Postgres container of the website
[API](/documentation/glossary/a_to_f.md#api), its seeds and registry import, the isolated
integration-test run, the migration checksum repair and the lane's self-test), the
`cargo xtask deploy db` verbs (the verified backup, the guarded restore, the restore drill and the
container helpers they share), the database source gates `cargo xtask verify wiki-seeds`,
`faction-library-seeds` and `no-select-star`, and the announcement seed that
`cargo xtask mod seed-announcement` runs. Developers run the `db` group locally; the systemd
timers on the website host run the backup and the drill.

## Contents

```text
tools/commands/database_operations/
├── Cargo.toml  the `database_operations` library package: `process_runner`, `repository_checks`, `repository_laws`, `api_readiness_checks`, layout tier 4
└── src/        the local database lane, backup, restore, drill, the container layer, the source checks and the errors
```

## How it works

`tools/xtask/src/cli/dispatch.rs` passes a parsed `DbCmd` to `local_database::run`, and the
`deployment` crate's `deploy` dispatch passes a `DeployDbCmd` to `container_database::run`. Each
`db` command that shells out first prints the line it runs, as a shell would echo it, then runs
it and returns the child's exit code unchanged.

```text
DbCmd ──▶ local_database::run
  ├─ up · down · logs · seed ─▶ <runtime> compose -f compose.dev.yml … in deploy (service `db`)
  ├─ backup · restore · backup-drill · backup-verify ─▶ backup, restore, restore_drill, in process
  ├─ registry-import ─▶ cargo run --bin import-item-registry in crates/api/api_server
  └─ test-it · repair-migration-checksum · selftest ─▶ local_database/
DeployDbCmd ──▶ container_database::run ─▶ backup | verify-dump | restore | drill | helpers
```

Every database command runs the Postgres tools inside the container through `<runtime> exec`;
the host needs a container runtime and no Postgres client. The runtime comes from
`resolve_runtime`: `TBD_CONTAINER_RUNTIME` when set, then `podman`, `docker`, and
`distrobox-host-exec podman` or `docker` from inside a container. A missing runtime, container or
tool, a bad argument value and a refused restore target are operator stops: the crate returns
`Error::Stop`, and the xtask binary prints its report bare (`FATAL: …`, or the refusal banner)
and exits 1. Every other error prints as `xtask: <step>: <cause>` with exit 1.

A restore or a drill targets only a database on the scratch allow-list (`rust_it`, `tbd_gate*`,
`*_cold`, `*_it`, `*_probe`) unless the confirmation repeats the target's name, and never
`tbd_reforger` without it. A dump is promoted or restored only after the five checks of
`verify_dump` hold. Every child runs through `process_runner::Run`: a capture for output read
as text, `Run::terminal` for those that share the terminal (the compose commands, `cargo test`,
`cargo run`, `ct`, `ct-i`), `Run::binary_output` for the archive fed to `pg_restore` and
`Run::output_to_files` for the dump written to files.

## Commands

Each runs from the repository root; `--help` on a group or a command prints clap's usage, and a
clap usage error exits 2.

### db up, down, logs, seed

- Synopsis: `cargo xtask db up`, `cargo xtask db down`, `cargo xtask db logs`,
  `cargo xtask db seed`
- Does: `compose up -d db` starts Postgres (`tbd_reforger_db`, host port 5434) in the background;
  `compose down` stops it and keeps the data volume; `compose logs -f db` follows the log until
  interrupted; `seed` applies `discord_roles.sql`, `registry_dev.sql`, `faction_library.sql`,
  `vehicle_database.sql` and `wiki_pages.sql` from `crates/api/api_database/seeds/`, in that order, each through
  `psql -v ON_ERROR_STOP=1`, stopping at the first failed file. Seeding needs the tables the API's
  boot migrations create: start `cargo xtask mk rust-api` once first. Each compose command names
  `deploy/compose.dev.yml` and runs in `deploy/`; `TBD_MK_WEB` points it at another folder, and
  `TBD_MK_TRACE=1` prints the real argv.
- Exit codes: the compose or `psql` run's own code; 2 a seed file that cannot be opened; 1 no
  container runtime (`FATAL:`).
- Example: `cargo xtask db up`

### db backup, backup-verify, backup-drill, restore

- Synopsis: `cargo xtask db backup [--db <name>] [--out <dir>] [--keep <n>]`;
  `cargo xtask db backup-verify --dump <file>`;
  `cargo xtask db backup-drill [--db <name>] [--out <dir>] [--fresh]`;
  `cargo xtask db restore --dump <file> --db <target> [--create]`
- Does: the same code as `deploy db backup`, `backup --verify-only`, `drill` and `restore` below,
  in process. `db restore` passes no confirmation, so it accepts only a scratch target.
- Exit codes: as the `deploy db` verbs; 2 `restore` without both `--dump` and `--db`, or
  `backup-verify` without `--dump`, after printing the usage line.
- Example: `cargo xtask db backup-verify --dump ~/tbd-backups/website/<name>.dump`

### db registry-import

- Synopsis: `cargo xtask db registry-import`
- Does: runs the API server's `import-item-registry` binary in `crates/api/api_server` over the two
  committed `contracts/catalogs/registry-*.workbench.json` envelopes (their paths anchored on the
  repository root), loading the item
  [registry](/documentation/glossary/n_to_z.md#registry) into the database `DATABASE_URL` names.
- Exit codes: cargo's own code.
- Example: `cargo xtask db registry-import`

### db test-it

- Synopsis: `cargo xtask db test-it [--test <binary>]... [--lib] [<filter>]`
- Does: creates one database for the run, named from the label in `TBD_IT_BASE_DB` (default
  `rust_it`) plus random hex, runs the API's suite against it with
  `cargo test --locked --no-fail-fast -p api_server -p <every other crates/api package>` (a
  `--test` selection names `-p api_server` alone, which holds the integration binaries), and
  drops the run's databases afterwards whatever the tests did. A selection narrows the run and prints that it is no
  readiness receipt. Needs `db up`.
- Exit codes: the test run's code when non-zero, else the cleanup's; 1 a label off the scratch
  allow-list; 2 a malformed `--test` or filter.
- Example: `cargo xtask db test-it --test factions`

### db repair-migration-checksum, db selftest

- Synopsis: `cargo xtask db repair-migration-checksum [--version <n>] [--force]`;
  `cargo xtask db selftest`
- Does: the repair repoints the `_sqlx_migrations` checksum of each applied migration whose file
  differs from the applied bytes in comments only, proving that from git history (`--force` skips
  the proof); the selftest runs six arms over the rendered recipes, the label guard and the
  cleanup, each with its own red proof. Both need `db up`.
- Exit codes: repair 0 nothing drifted or all repaired, 1 a drift refused; selftest 0 every arm
  held, 1 an arm failed, 2 an arm could not run.
- Example: `cargo xtask db repair-migration-checksum --version 21`

### deploy db

- Synopsis: `cargo xtask deploy db <command>`, with the commands:
  - `backup [--db <name>] [--out <dir>] [--keep <n>] [--min-rows <n>] [--verify-only <file>]`:
    dumps `--db` (else `TBD_BACKUP_DB`, else `tbd_reforger`) to
    `<out>/<db>-<UTC stamp>.dump.part`, verifies it, promotes it, sets mode 600, then keeps the
    newest `--keep` (else `TBD_BACKUP_KEEP`, else 14; at least 1) of `<db>-*.dump` in `--out`
    (else `TBD_BACKUP_DIR`, else `~/tbd-backups/website`); `--verify-only` checks an existing dump.
  - `verify-dump --file <path> [--min-rows <n>] [--expect-db <name>]`: the five checks alone,
    printing the row count; an empty `--expect-db` skips the identity check.
  - `restore (--db <name> | --url <postgres-url>) [--create] [--jobs <n>] [--min-rows <n>]
    [--expect-db <name>] [--i-understand-this-destroys <name>] <dump>`: refuses the target before
    contacting the container, verifies the dump against its source database, then runs
    `pg_restore --clean --if-exists --no-owner --no-privileges --exit-on-error`.
  - `drill [--dump <file>] [--fresh] [--db <name>] [--out <dir>] [--scratch <name>]
    [--keep-scratch] [--lax-migrations]`: restores the newest or given dump into a scratch database
    and audits its tables, enums and migration checksums.
  - helpers: `refuse-unsafe`, `is-safe-scratch`, `database-name-from-url`, `require-container`,
    `require-pg-tool`, `database-exists`, `count-rows`, and `ct` / `ct-i`, which run a command in
    the container without and with stdin.
- Exit codes: 0 done, or the predicate holds; 1 a refusal, a failed check, dump or restore, a
  missing container or tool (`FATAL:`), or a predicate that does not hold; 2 usage; `ct` and
  `ct-i` return the command's own code.
- Example: `cargo xtask deploy db is-safe-scratch --db rust_it`

### verify wiki-seeds, faction-library-seeds, no-select-star

- Synopsis: `cargo xtask verify wiki-seeds`, `cargo xtask verify faction-library-seeds`,
  `cargo xtask verify no-select-star`
- Does: the seed list really applies the wiki and faction-library seeds, which hold their pinned
  rows, and the wave gate runs the faction check; the API's SQL never reads `*` from a table.
  None needs a running database.
- Exit codes: 0 held; 1 findings; 2 a check could not run.
- Example: `cargo xtask verify wiki-seeds`

### mod seed-announcement

- Synopsis: `cargo xtask mod seed-announcement`, defined in the `mod` group.
- Does: inserts the pinned "Milestone #1" website announcement unless one exists, through `psql`
  with `DATABASE_URL` from the environment or `crates/api/api_server/.env`, else through
  `podman exec -i tbdevent-postgres psql` when that container runs.
- Exit codes: 0 inserted or present; 1 no `psql` and no container, or no `DATABASE_URL`; `psql`'s
  own code; 127 a tool that is not installed.
- Example: `cargo xtask mod seed-announcement`

## Boundaries

- Depends on: `process_runner`, `repository_layout`, `verification_core`, `content_digest`,
  `repository_checks` (the wave gate's seed sources), `api_readiness_checks` (the property-test
  seed), `clap`, `regex`, `thiserror`; `deploy/compose.dev.yml`, the seeds and migrations of
  `crates/api/api_database/`; a container runtime, cargo and git.
- Used by: the `db`, `verify`, `ci`, `mk` and `mod` groups of `xtask`; the `deployment` crate
  (`deploy db`); the `tbd-website-backup` and `tbd-website-backup-drill` units in
  `deploy/systemd/`; the remote steps of `cargo xtask deploy website`, which run
  `db repair-migration-checksum`.
- Rules: tier 4 of `tools/commands` (`cargo xtask verify crate-tiers`); `LANE_COMMANDS` names
  exactly the `DbCmd` commands (`lane_commands_match_the_clap_enum`); `SEEDS` keeps its five files
  in order and the two seed gates read it; `tbd_reforger` is never a test-it label or a scratch
  target (`safe_scratch_allow_list_admits_scratch_names_and_refuses_the_live_database`).

## Related documentation

- [Database operations](/documentation/runbooks/database_operations.md) — psql access, sample
  data, the integration tests, the checksum repair, backups, drills and restores.
- [Local development](/documentation/runbooks/local_development.md) — the database, the API and
  the app on a developer machine.
- [Website deployment](/documentation/runbooks/website_deployment.md) — the server database and
  its backups.
- [Deployment](/tools/commands/deployment/README.md) — the `deploy` group that carries `deploy db`.
