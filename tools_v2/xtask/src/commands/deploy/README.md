# Deploy commands

The `cargo xtask deploy` group: the website deploy to the home server, the staging deploy of the
dedicated game server, and the database commands that back up, verify, restore and drill the
website's Postgres. The operator runs the deploys from a development machine; the systemd timers
on the host run the backup and the drill.

## Contents

```text
tools_v2/xtask/src/commands/deploy/
├── cli.rs                             the `DeployCmd` clap enum: `website`, `db` and `staging`
├── database_backup.rs                 `deploy db backup`: a verified `pg_dump -Fc` with retention by count
├── database_operations/               the runtime and container helpers, the allow-list and the dump verifier
├── database_operations.rs             the `DeployDbCmd` clap enum; declares the helpers and re-exports them
├── database_restore.rs                `deploy db restore`: guard, verify, then `pg_restore --clean --if-exists`
├── database_restore_drill/            the drill: restore into a scratch database, then the table and boot audits
├── database_restore_drill.rs          the scratch-drop guard; declares the drill and re-exports `run`
├── development_machine_only_paths.rs  the rsync excludes both deploys share: build folders and local tool state
├── dispatch.rs                        routes each `DeployCmd` to its entry
├── mod.rs                             the module tree
├── remote_rust_toolchain.rs           the PATH line every remote step runs before it calls cargo or trunk
├── staging/                           the staging deploy: settings, render, payloads, pipeline and boot verdict
├── staging.rs                         `deploy staging`: `Paths`, the flag parser and the mode order
├── tests/                             unit tests for backup, the helpers, the drill, the shared excludes, staging flags and website
├── website/                           the website deploy's pure steps: rsync argv, remote shells, probe, unit
└── website.rs                         `deploy website`: deploy.env, the refusals and the step runner
```

## How it works

`tools_v2/xtask/src/cli/dispatch.rs` passes the parsed `DeployCmd` to `dispatch::run`. The
`website`, `staging`, `db backup`, `db drill`, `db ct` and `db ct-i` commands take their arguments
raw and parse them themselves; the other `db` commands parse with clap.

Both deploys read `tools_v2/xtask/deploy/deploy.env` (or the file `DEPLOY_ENV` names), which is
gitignored and excluded from both rsyncs, through `crate::core::deploy_environment`: parsed as
`KEY=VALUE` lines and never executed, the file deciding every key it assigns and the process
environment filling only the others; a missing file exits 1 and names `deploy.env.example`. The
host is whatever `TBD_SSH_HOST` names, and the remote folders default under its user's
`/home/<user>`; ssh runs through sshpass when `TBD_SSH_PASS` is set, else with `-i` when
`TBD_SSH_IDENTITY_FILE` is, always with `StrictHostKeyChecking=no`.

Both rsyncs append the patterns of `development_machine_only_paths.rs` to their own exclusions:
anchored at the checkout root, they name what only a development machine holds, namely the
retired and hand-set cargo target folders beside `target/` (also those beside the app), the
retired gate app builds and the debug app build,
the vanilla compile baseline, slice and ticket worktrees, the wave gate's receipts, and the local
files of Claude Code, Codex and the MCP configuration. None of them is tracked, and the host needs
none of them.

```text
deploy website: asset probe ─▶ rsync --delete ─▶ compose postgres ─▶ API build ─▶ host tools build
                ─▶ app build ─▶ compose caddy, then its reload ─▶ checksum repair
                ─▶ uploads to the state folder ─▶ restart the unit ─▶ hints
deploy staging: settings check ─▶ website API check ─▶ secret files ─▶ single-instance check
                ─▶ rsync --delete ─▶ per instance: files and runtime smoke ─▶ units, restart
                ─▶ boot verdict per instance ─▶ relay ─▶ host agents ─▶ log check per instance
deploy db:      container helpers ─▶ backup | verify-dump | restore | drill
```

Every database command runs the Postgres tools inside the container through `<runtime> exec`; the
host needs a container runtime and no Postgres client. A restore or a drill targets only a
database on the scratch allow-list (`rust_it`, `tbd_gate*`, `*_cold`, `*_it`, `*_probe`) unless the
confirmation repeats the target's name, and never `tbd_reforger` without it.

## Commands

Each runs as `cargo xtask deploy <command>`; a clap usage error exits 2.

### website

- Synopsis: `cargo xtask deploy website [--dry-run] [--help]`
- Does: reads `TBD_SSH_HOST` (required, with the deploy user) and `TBD_REMOTE_DIR` (default
  `/home/<user>/tbd/repo`), refuses a host, remote folder or `TBD_PROFILE_DIR` that contains
  `prairielearn` in any case, a host without a user, and a remote folder outside
  `/home/<user>/tbd/` or holding `..`, then probes the server's map assets, rsyncs the checkout,
  and over ssh brings up the staging Postgres (`TBD_POSTGRES_HOST_PORT`, default 5432), builds the
  release API, the staging host tools `staging-fixtures` and `acknowledgement-dropping-relay`
  into `target/release/` and the app, starts the staging Caddy and reloads its Caddyfile, repoints
  comments-only migration checksums, moves uploads into the unit's state folder and restarts
  `TBD_WEBSITE_SYSTEMD_UNIT` (default `tbd-website-api.service`). Set to 1, `TBD_SKIP_COMPOSE`
  skips both compose steps (Postgres and Caddy), `TBD_SKIP_API_BUILD` both cargo builds and
  `TBD_SKIP_SPA_BUILD` the app build, after which Caddy serves the build already on the host. A
  failing step stops the deploy before the restart; a failed restart only warns and prints the
  unit's install command. It ends with two `curl` smoke hints. `--dry-run` prints the plan, with
  every rsync exclusion, and connects to nothing; it still needs a filled `deploy.env`.
- Exit codes: 0 deployed, or the plan printed; 1 no `deploy.env`, an unreadable or malformed one,
  a missing required value, a refused path or host, or a refused asset layout; 2 an unknown option; a failing
  rsync or ssh step's own code; 127 ssh, sshpass or rsync not installed.
- Example: `cargo xtask deploy website --dry-run`

### staging

- Synopsis: `cargo xtask deploy staging [--dry-run] [--migrate-single-instance] [--render-only
  <directory>] [--verify-boot <console.log>] [--verify-boot-selftest]`
- Does: checks that the website API answers `/healthz` at `TBD_BACKEND_URL` on the staging host,
  then deploys the checkout there and runs the fleet of `TBD_FLEET_INSTANCES` (at most 5)
  dedicated servers of the experimental server install (Steam app 1890870): instance N runs as
  `tbd-reforger@N.service` with `-addonsDir` and its own `-config` and `-profile` under
  `~/tbd/fleet/instance-N/`, beside its
  [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent)
  `fleet-host-agent@N.service`; the relay instance's agent polls the API through
  `acknowledgement-dropping-relay@N.service`. Each instance's boot is judged from its own log:
  the synced addon won, a room registered and the config loaded. The machine credentials are
  files `cargo xtask staging provision-fleet` writes on the host after `deploy website` built its
  tool, the RCON passwords are generated on the host, and the join password is the operator's
  `~/tbd/fleet/join-password`; the deploy refuses a missing one, and refuses the retired
  single-server settings such as `TBD_MOD_RUNTIME_CREDENTIAL` without printing their values.
  While the single-server units `tbd-reforger.service` and `fleet-host-agent.service` are
  installed it refuses, and `--migrate-single-instance` stops, disables and archives them first.
  It runs no compose command: the website stack on the host is `deploy website`'s.
  `--render-only` writes every instance's `instance-N/server.config.json` into a local directory
  after the settings check; `--verify-boot` judges a log you already have;
  `--verify-boot-selftest` proves the verdict can fail. The last two need no `deploy.env`.
- Exit codes: 0 deployed, rendered or judged healthy; 1 a missing, retired or refused setting
  (among them a `TBD_PUBLIC_ADDRESS` that is not IPv4, or none set and none resolved), a missing
  or malformed secret file, the single-server units still installed without
  `--migrate-single-instance`, a website API that does not answer on the host, a failed boot
  verdict or a failed final log check; 2 an unknown option or a flag without its value, or
  `--verify-boot` without `TBD_ADDONS_STAGING`; a failing step's own code; 127 a tool that is not
  installed.
- Example: `cargo xtask deploy staging --verify-boot-selftest`

### db

- Synopsis: `cargo xtask deploy db <command>`, with the commands:
  - `backup [--db <name>] [--out <dir>] [--keep <n>] [--min-rows <n>] [--verify-only <file>]`:
    dumps `--db` (else `TBD_BACKUP_DB`, else `tbd_reforger`) to
    `<out>/<db>-<UTC stamp>.dump.part`, verifies it, promotes it, sets mode 600, then keeps the
    newest `--keep` (else `TBD_BACKUP_KEEP`, else 14; at least 1) of `<db>-*.dump` in `--out`
    (else `TBD_BACKUP_DIR`, else `~/tbd-backups/website`); `--verify-only` checks an existing dump
    against the source database and takes none.
  - `verify-dump --file <path> [--min-rows <n>] [--expect-db <name>]`: the five checks alone,
    printing the row count; an empty `--expect-db` (the default) skips the identity check.
  - `restore (--db <name> | --url <postgres-url>) [--create] [--jobs <n>] [--min-rows <n>]
    [--expect-db <name>] [--i-understand-this-destroys <name>] <dump>`: refuses the target before
    contacting the container, verifies the dump against its source database (default
    `TBD_RESTORE_EXPECT_DB`, else `TBD_BACKUP_DB`, else `tbd_reforger`), then runs
    `pg_restore --clean --if-exists --no-owner --no-privileges --exit-on-error` and fails when a
    dump with rows restores to none.
  - `drill [--dump <file>] [--fresh] [--db <name>] [--out <dir>] [--scratch <name>]
    [--keep-scratch] [--lax-migrations]`: restores the newest or given dump into the scratch
    database and audits it.
  - Helpers the other commands share: `refuse-unsafe --db <name> [--confirm <name>]`,
    `is-safe-scratch --db <name>`, `database-name-from-url <url>`, `require-container`,
    `require-pg-tool <tool>`, `database-exists --db <name>`, `count-rows --db <name>`, and
    `ct <args>` and `ct-i <args>`, which run a command in the container without and with stdin.
- Exit codes: 0 done, or the predicate holds; 1 a refusal, a failed check, a failed dump or
  restore, a missing container or tool (`FATAL:`), or a predicate that does not hold; 2 usage
  (`backup`, `drill` and a `restore` with neither `--db` nor `--url`); `ct` and `ct-i` return the
  command's own code.
- Example: `cargo xtask deploy db is-safe-scratch --db rust_it`

## Boundaries

- Depends on: `crate::core::repository_root` and `crate::core::repository_layout` (the deploy
  files under `tools_v2/xtask/deploy/`); `verification_core` for runs and verdicts; ssh, sshpass
  and rsync on the development machine; a container runtime and the Postgres container; on the
  host, cargo, trunk, docker or podman compose, systemd user units and the dedicated server.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`;
  - `tools_v2/xtask/src/commands/db/`, which runs backup, verify, drill and restore in process and
    uses the container helpers;
  - the `tbd-website-backup` and `tbd-website-backup-drill` units in
    `tools_v2/xtask/deploy/systemd/`, which run `deploy db backup` and `deploy db drill`;
  - the operator, for both deploys.
- Rules: `deploy.env` is never executed and never rsynced (both exclude lists name `DEPLOY_ENV`);
  both rsyncs exclude every development-machine-only path, no such path matches a tracked file,
  and each pattern is anchored at the checkout root
  (`no_tracked_file_matches_a_development_machine_only_path` and
  `every_development_machine_only_path_is_anchored_and_uses_only_the_star_wildcard` in
  `tests/development_machine_only_paths/tests.rs`);
  documents name the host only as `TBD_SSH_HOST`; the scratch allow-list refuses `tbd_reforger`
  (`safe_scratch_allow_list_admits_scratch_names_and_refuses_the_live_database` in
  `tests/database_operations/tests.rs`); the website deploy refuses a remote folder outside the
  deploy user's `tbd` folder (`remote_prefix_rejects_escape_and_outside` in
  `tests/website/tests.rs`); an unknown
  staging option stops before `--help` (`unknown_option_short_circuits_before_help` in
  `tests/staging/tests.rs`).

## Related documentation

- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the home server, its
  API unit, Caddy and the website deploy.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  game server and its host agent.
- [Deployment templates](/tools_v2/xtask/deploy/README.md) — `deploy.env`, the Caddy site and the
  systemd units these commands read or print.
- [Database operations](/documentation_v2/runbooks/database_operations.md) — the `deploy db` backups,
  restore drills and restores, run by hand and by the backup timers.
