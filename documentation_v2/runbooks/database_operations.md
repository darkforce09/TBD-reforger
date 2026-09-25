**Status:** live

# Database operations

The procedures on the website [API](/documentation_v2/glossary.md#api)'s Postgres database beyond
starting it: running SQL, loading and removing sample data, the isolated integration-test run,
repairing a migration checksum after a comments-only edit, and taking, verifying, drilling and
restoring backups, locally and on the home server. Starting the local database, the first seed
and the API are in [Local development](/documentation_v2/runbooks/local_development.md). Each
procedure takes seconds to minutes; a full integration-test run takes several minutes.

## Prerequisites

- A container runtime: `TBD_CONTAINER_RUNTIME`, else `podman`, `docker`, or
  `distrobox-host-exec podman` or `docker` from inside a distrobox container. No `psql`,
  `pg_dump` or `pg_restore` is needed on the machine: every command runs them inside the database
  container.
- The database container running. Locally that is `tbd_reforger_db` from `cargo xtask db up`;
  check with `cargo xtask deploy db require-container`, which exits 0 or names `db up`. On the
  home server, `cargo xtask deploy website` starts `tbd_staging_db` from
  `apps/website/docker-compose.staging.yml` on `127.0.0.1:${TBD_POSTGRES_HOST_PORT:-5432}`, and
  the commands reach it with `TBD_DB_CONTAINER=tbd_staging_db` exported. `TBD_DB_USER` (default
  `tbd`) names the role.
- For the server procedures: a shell on the host `TBD_SSH_HOST` names in
  `tools_v2/xtask/deploy/deploy.env`, in the checkout at `TBD_REMOTE_DIR`, with cargo installed.

The commands refuse to write to a database outside the scratch allow-list (`rust_it`,
`tbd_gate*`, `*_cold`, `*_it`, `*_probe`) unless a confirmation repeats its name, and
`tbd_reforger` is never on the list. `cargo xtask deploy db is-safe-scratch --db <name>` exits 0
for a name on it.

## Steps

Run every command from the repository root.

### Query the local database

1. Run one statement. `deploy db ct` runs a command inside the container, with no terminal.

   ```bash
   cargo xtask deploy db ct psql -U tbd -d tbd_reforger \
     -c "SELECT max(version), count(*) FROM _sqlx_migrations"
   ```

   Expected: psql's table, here the highest applied migration version and the number applied.

2. Pipe a SQL file in. `deploy db ct-i` passes standard input through, so it also applies a
   seed when `cargo xtask db seed` cannot run for want of a compose provider.

   ```bash
   cargo xtask deploy db ct-i psql -U tbd -d tbd_reforger \
     < apps/website/api_v2/seeds/discord_roles.sql
   ```

   Expected: psql's command tags, here `DELETE 0` and `INSERT 0 3`, and no `ERROR:` line. psql
   carries on past a failed statement, so read the output as well as the exit code.

### Load and remove the sample data

`apps/website/api_v2/seeds/mock_data.sql` holds sample users, modpacks and four
[missions](/documentation_v2/glossary.md#mission) with fixed ids. `cargo xtask db seed` never
applies it.

1. Apply it after the API has migrated the database.

   ```bash
   cargo xtask deploy db ct-i psql -U tbd -d tbd_reforger \
     < apps/website/api_v2/seeds/mock_data.sql
   ```

   Expected: psql's `INSERT` tags and no `ERROR:` line.

2. Remove the four missions. Their versions, armories and bookmarks go with them
   (`ON DELETE CASCADE` in `apps/website/api_v2/migrations/0018_foreign_keys.sql`); the sample
   users and modpacks stay.

   ```bash
   cargo xtask deploy db ct psql -U tbd -d tbd_reforger -c "DELETE FROM missions WHERE id IN (
     '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-c000-000000000002',
     '00000000-0000-4000-c000-000000000003', '00000000-0000-4000-c000-000000000004')"
   ```

   Expected: `DELETE 4`. An [event](/documentation_v2/glossary.md#event), an
   [artifact](/documentation_v2/glossary.md#artifact), a review or a
   [mission deployment](/documentation_v2/glossary.md#mission-deployment) that refers to one of
   them blocks the delete with a foreign-key error; remove that row first.

### Run the integration tests

1. Run the API's integration suites against a database of their own. The command needs the
   container, not the API.

   ```bash
   cargo xtask db test-it
   ```

   Expected: the property-test marker line, then `cargo test --locked --no-fail-fast` in
   `apps/website/api_v2` against a new database named `i<first five letters of the label>_<32
   hex>_it`, the label being `TBD_IT_BASE_DB` (default `rust_it`). Each suite derives its own
   scratch database from it. Afterwards the command drops every database of the run, whatever the
   tests did, and exits with the test run's code. Two runs never share a database.
   `--test <binary>` (repeatable), `--lib` and a name filter narrow the run, which then prints
   `test-selection: narrowed development run; not a readiness receipt`.

### Repair a migration checksum

sqlx records the SHA-384 of each applied migration file, comments included, and the API refuses to
boot with `migration N was previously applied but has been modified` once the file changes.
`apps/website/api_v2/tests/migrations_are_immutable.rs` pins every file's checksum, so the edit
fails the tests first. A statement change is never repaired: it is a new migration. The schema is
fine after a comments-only edit, so never reset the volume for this error.

1. On a development checkout, repoint the migration the error names.

   ```bash
   cargo xtask db repair-migration-checksum --version N
   ```

   Expected: `N (<file>): file does not match the applied checksum.`, then
   `comments only — the statements are unchanged:` with the comment diff and
   `repointed migration N to <checksum>`, and the summary `1 drifted, 1 repaired, 0 refused.`.
   The command recovers the bytes the database applied from git history
   (`git log --all --follow`) and compares both with comments and blank lines stripped. A
   statement difference prints `REFUSED: the statements themselves differ, not just comments.`
   and exits 1. Without `--version` it examines every applied migration; with nothing to do it
   prints `every applied migration examined matches its file. Nothing to repair.`

2. On the home server, after checking the edit on a development checkout with step 1. The deploy
   rsync leaves out `.git/`, so the server cannot prove the edit and needs `--force`, which
   repoints without the proof. `cargo xtask deploy website` runs this itself before it restarts
   the API; run it by hand only when a boot still refuses.

   ```bash
   TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force
   ```

   Expected: for each drifted migration,
   `--force: the applied bytes are not in this checkout's history, repointing anyway.` and
   `repointed migration N to <checksum>`, then the summary with `0 refused.`. `TBD_DB_NAME`
   (default `tbd_reforger`) names the database.

### Back up and verify

A backup is a `pg_dump -Fc` archive written as `<db>-<UTC stamp>.dump.part`, checked, then
renamed to `<db>-<YYYYMMDDTHHMMSSZ>.dump` with mode 600. The check reads the dump back through the
container and stops at the first failure: the file exists and is not empty; it starts with the
`PGDMP` magic; `pg_restore --list` reads its table of contents; the header's `dbname:` is the
source database and the contents list `_sqlx_migrations`; and `pg_restore --data-only` reads the
whole body with at least the minimum rows (default 1). The last check is the one that catches a
truncated or corrupt file, which passes `--list`. A dump that fails is deleted, and the earlier
backups stay.

1. Take a backup. `--db` (else `TBD_BACKUP_DB`, else `tbd_reforger`), `--out` (else
   `TBD_BACKUP_DIR`, else `~/tbd-backups/website`) and `--keep` (else `TBD_BACKUP_KEEP`, else 14)
   are optional; retention keeps the newest `--keep` dumps of that database and deletes the rest.

   ```bash
   cargo xtask db backup
   ```

   Expected: `==> VERIFIED  <out>/tbd_reforger-<stamp>.dump (<n> bytes, <rows> data rows)`, then
   `==> retention  kept newest 14, removed <n> older dump(s)` (or `nothing to prune`) and
   `==> done`. A failed check prints
   `FAIL: the dump did NOT verify — refusing to promote it to <path>.` and exits 1.

2. Re-check an existing dump without taking a new one.

   ```bash
   cargo xtask db backup-verify --dump ~/tbd-backups/website/<name>.dump
   ```

   Expected: `OK: <file> verified — <rows> data row(s), TOC and full archive body read back.`,
   or `FAIL: <file> did NOT verify.` and exit 1.

### Drill a restore

A drill proves a backup can be recovered and that the API would boot on it.

1. Restore the newest dump into the scratch database and audit it. `--fresh` takes a new backup
   first; `--db` and `--out` pick the source database and the dump folder as for a backup.

   ```bash
   cargo xtask db backup-drill
   ```

   Expected: `═══ backup restore drill ═══`, the restore into `tbd_drill_probe` (`TBD_DRILL_DB`),
   the table, enum, index and row counts, the boot audit (every `_sqlx_migrations` row has a
   file in `apps/website/api_v2/migrations/`, succeeded and matches the file's SHA-384; newer
   files are listed as pending), then
   `DRILL PASS — <dump> restored into 'tbd_drill_probe' with <rows> row(s) across <n> table(s), and is boot-ready.`
   The scratch database is dropped afterwards. `DRILL FAIL` exits 1; treat it as an incident,
   since the backups are not proven recoverable. `cargo xtask deploy db drill` is the same drill
   with `--dump <file>`, `--scratch <name>`, `--keep-scratch` and `--lax-migrations` besides.

### Restore a dump

A restore verifies the dump first, then runs
`pg_restore --clean --if-exists --no-owner --no-privileges --exit-on-error`, and fails when a dump
with rows restores to none.

1. Restore into a scratch database, for a look at the data.

   ```bash
   cargo xtask db restore --dump ~/tbd-backups/website/<name>.dump \
     --db tbd_restore_probe --create
   ```

   Expected: the five checks, the restore, exit 0. A target outside the allow-list is refused
   before the container is contacted, with exit 1; the command takes no confirmation.

2. Replace a live database. This destroys its current contents, so stop the API first
   (`systemctl --user stop tbd-website-api` on the home server, Ctrl-C locally) and take a
   backup with the procedure above.

   ```bash
   cargo xtask deploy db restore --db tbd_reforger \
     --i-understand-this-destroys tbd_reforger ~/tbd-backups/website/<name>.dump
   ```

   Expected: the checks against the source database (`--expect-db`, else `TBD_RESTORE_EXPECT_DB`,
   else `TBD_BACKUP_DB`, else `tbd_reforger`), then the restore and exit 0. Without the
   confirmation, or with one that differs from `--db`, it refuses with exit 1. `--jobs` sets the
   parallel restore workers (default 1, or `TBD_RESTORE_JOBS`).

### Schedule the home server's backups

The nightly backup (03:20) and the weekly drill (Sunday 04:10) are systemd user units in
`tools_v2/xtask/deploy/systemd/`, each running `cargo run -q -p xtask -- deploy db …` in the
checkout with `Persistent=true`, so a night the machine was off runs late instead of never. Run
these on the home server in the checkout; the
[systemd README](/tools_v2/xtask/deploy/systemd/README.md) describes every unit.

1. Render the backup service with the checkout's absolute path in place of the placeholder.

   ```bash
   sed "s|/TBD_REPO_DIR_PLACEHOLDER|$PWD|g" tools_v2/xtask/deploy/systemd/tbd-website-backup.service \
     > ~/.config/systemd/user/tbd-website-backup.service
   ```

   Expected: no output. Check that the rendered file's `TBD_DB_CONTAINER` names the container the
   host runs (`podman ps` or `docker ps`); the template says `tbd_reforger_db`, while the website
   deploy starts `tbd_staging_db`.

2. Render the drill service the same way, with the same container check.

   ```bash
   sed "s|/TBD_REPO_DIR_PLACEHOLDER|$PWD|g" \
     tools_v2/xtask/deploy/systemd/tbd-website-backup-drill.service \
     > ~/.config/systemd/user/tbd-website-backup-drill.service
   ```

   Expected: no output.

3. Copy both timers.

   ```bash
   cp tools_v2/xtask/deploy/systemd/tbd-website-backup.timer \
     tools_v2/xtask/deploy/systemd/tbd-website-backup-drill.timer ~/.config/systemd/user/
   ```

   Expected: no output.

4. Load and start them.

   ```bash
   systemctl --user daemon-reload && \
     systemctl --user enable --now tbd-website-backup.timer tbd-website-backup-drill.timer
   ```

   Expected: two `Created symlink` lines.

5. Keep user units running after logout.

   ```bash
   loginctl enable-linger "$USER"
   ```

   Expected: no output; without it the timers stop at logout.

### Reset the local volume

Only for a local volume that cannot start, such as one holding another Postgres major version's
data. It deletes every local row; the development data is reseedable.

1. Stop the container.

   ```bash
   cargo xtask db down
   ```

   Expected: compose stops and removes `tbd_reforger_db`.

2. Find the data volume: the compose project's name, then `_tbd_pgdata`.

   ```bash
   podman volume ls
   ```

   Expected: a line ending in `_tbd_pgdata`.

3. Remove it.

   ```bash
   podman volume rm <project>_tbd_pgdata
   ```

   Expected: the volume's name. Then
   [Local development](/documentation_v2/runbooks/local_development.md) steps 2 to 4 recreate,
   migrate and seed the database.

## Verify

```bash
cargo xtask deploy db count-rows --db tbd_reforger
```

Expected: the exact number of live rows across the database's user tables, exit 0. On the home
server, `journalctl --user -u tbd-website-backup.service -n 50` shows the last backup's
`VERIFIED` line, `journalctl --user -u tbd-website-backup-drill.service -n 60` the last
`DRILL PASS`, and `systemctl --user list-timers` both timers' next runs.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `FATAL:` naming `cargo xtask db up` | the container is missing or stopped, or `TBD_DB_CONTAINER` names another | start it: `cargo xtask db up` locally, or export the right `TBD_DB_CONTAINER` |
| `db restore` or `db backup-drill` refuses its target, exit 1 | the target is off the scratch allow-list | use a scratch name such as `*_probe`, or `deploy db restore` with `--i-understand-this-destroys` |
| `VERIFY FAIL:` on a dump that `pg_restore --list` reads | the body is truncated or corrupt, or the dump comes from another database | take a new backup; restore an older dump |
| `DRILL FAIL` about `_sqlx_migrations` | the dump has no migration table, so the API would re-apply the first migration over it | restore a dump taken from a migrated database; `--lax-migrations` only warns |
| `REFUSED: the bytes the database applied are not in this checkout's git history` | the checkout lacks the history, as on the server | check the edit on a development checkout (step 1), then `--force` |
| `REFUSED: the statements themselves differ, not just comments.` | an applied migration's statements changed | restore the file and add a new migration |
| `test-it` exits 1 before running anything | `TBD_IT_BASE_DB` is off the allow-list | unset it, or use a `*_it` name |
| `test-it` stops with `PROPTEST_CASES must be unset; each property test defines its own case count` | `PROPTEST_CASES` is exported | unset it; `PROPTEST_RNG_SEED` may stay |
| `FATAL: cleanup query failed` after `test-it` | the container stopped during the run, so the run's databases may remain | start the container and run `test-it` again; its cleanup takes only its own run's names |
| the backup unit fails at once with "Changing to the requested working directory" | the service was copied without the `sed` render | Schedule the home server's backups, steps 1 and 2 |
| the backup unit fails with the container missing | the unit's `TBD_DB_CONTAINER` is `tbd_reforger_db` while the host runs `tbd_staging_db` | set the rendered unit's `TBD_DB_CONTAINER` to the running container, then `systemctl --user daemon-reload` |

## Related

- [Local development](/documentation_v2/runbooks/local_development.md) — starting the database,
  the API and the first seed.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the home server, its
  database container and the deploy that repairs checksums.
- [Database commands](/tools_v2/xtask/src/commands/db/README.md) — every `cargo xtask db` command
  and its exit codes.
- [Deploy commands](/tools_v2/xtask/src/commands/deploy/README.md) — every `cargo xtask deploy db`
  command, the dump checks and the drill.
- [Database migrations](/apps/website/api_v2/migrations/README.md) and
  [database seeds](/apps/website/api_v2/seeds/README.md) — the schema files and the seed files.
