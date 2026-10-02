# API executables

The three binaries of the `api` crate: the `api` server behind the web platform,
`import-registry`, which loads the item [registry](/documentation/glossary/n_to_z.md#registry) that
Workbench exports into Postgres, and `staging-fixtures`, the host tool that stages the fixtures of a
staging verification run.

## Contents

```text
apps/api/src/bin/
├── api.rs              the `api` binary: the HTTP server, its migrations and background workers
├── import_registry.rs  the `import-registry` binary: loads registry envelopes into Postgres
└── staging_fixtures/   the `staging-fixtures` binary: fleet servers, credentials and load fixtures
```

## How it works

The `[[bin]]` tables of `apps/api/Cargo.toml` name the three binaries. Each `main` is
a Tokio entry point that calls into the `api` library. `api` and `import-registry` return
an `anyhow::Result`, so a failure prints `Error: <cause>` and exits 1; `staging-fixtures` prints
`staging-fixtures: refused: <reason>` (exit 2) or `staging-fixtures: failed: <reason>` (exit 1).
None uses an argument-parsing crate: `api` takes no arguments and reads everything from the
environment, `import-registry` walks its four flags by hand, and `staging-fixtures` hands its flags
to the parser of each subcommand in its table.

```text
api.rs              ──▶ Config::load ─▶ database::connect
                        ─▶ database::migrate, unless SKIP_MIGRATE is set
                        ─▶ AppState::new ─▶ background_workers::spawn_all
                        ─▶ http_router::router ─▶ axum::serve until SIGINT or SIGTERM
                        ─▶ process_shutdown begins: open SSE streams close, the drain ends
import_registry.rs  ──▶ database::connect ─▶ database::migrate
                        ─▶ registry_import::import_items and import_compat
staging_fixtures/   ──▶ the subcommand's flags parse ─▶ the API env file's DATABASE_URL
                        ─▶ database::connect ─▶ current_database() equals --confirm-database
                        ─▶ the subcommand: a dry run, or its writes with --apply
```

## Commands

Run `api` and `import-registry` from `apps/api/` as
`cargo run --bin <name> -- <arguments>`; both read `.env` there. Run `staging-fixtures` from the
checkout root, where its default API env file, `apps/api/.env`, resolves.

### api

- Synopsis: `api`, with no arguments; the environment and `.env` configure it.
- Does: sets the log filter from `RUST_LOG` (`info` when unset), loads the configuration, connects
  to Postgres, applies the pending migrations unless `SKIP_MIGRATE` is set and logs
  `migrations applied`, arms the
  [background workers](/documentation/glossary/a_to_f.md#background-workers), and serves every route
  on `0.0.0.0:$PORT`. It stays in the foreground until SIGINT or SIGTERM. The signal begins
  `core::process_lifecycle::process_shutdown`: the server stops accepting connections, every open
  [SSE](/documentation/glossary/n_to_z.md#sse) stream (the audit log feed, the server status
  streams) ends its body at once with no further event, and the requests in flight drain, so the
  process exits without waiting for the service manager's kill timeout. A client of the audit
  log feed reconnects with `Last-Event-ID` and misses no row. It needs Postgres running.
- Exit codes: 0 after a signal and a clean drain; 1 when the configuration, the database
  connection, a migration or the port bind fails.
- Example: `cargo xtask mk rust-api`, which runs `cargo run --bin api` in `apps/api/`
  with its own target directory, `target/dev-api` in the checkout.

### import-registry

- Synopsis: `import-registry [--items <path>] [--compat <path>] [--modpack <uuid>] [--prune]`
- Does: reads `DATABASE_URL`, connects, applies the pending migrations, then imports the item
  envelope (`--items`) and the compatibility-edge envelope (`--compat`), each checked against its
  schema in `contracts/definitions/`, into the registry tables of the envelope's `modpackId`,
  or of `--modpack` when given. Re-running an envelope updates rows in place; `--prune` also
  deletes that modpack's rows the envelope does not hold. It prints the total, unique, inserted,
  updated and pruned counts of each envelope. At least one of `--items` and `--compat` is
  required.
- Exit codes: 0 when every envelope imported; 1 on an unknown flag, a flag without its value, a
  malformed `--modpack`, neither envelope given, an unset `DATABASE_URL`, an unreadable file, or
  a failed import.
- Example: `cargo xtask db registry-import`, which imports
  `contracts/catalogs/registry-items.workbench.json` and
  `contracts/catalogs/registry-compat.workbench.json`.

### staging-fixtures

- Synopsis: `staging-fixtures <subcommand> [flags] --confirm-database <name> [--apply]
  [--api-env-file <path>]`, and `staging-fixtures --help`, which lists the subcommands.
- Does: reads the API env file (`apps/api/.env` relative to the working directory
  unless `--api-env-file` names another) without touching the process environment, connects to
  its `DATABASE_URL`, refuses unless `current_database()` equals `--confirm-database`, prints the
  count of accounts in the reserved synthetic range (Discord ids 9100000000000000000 to
  9100000000000099999), and runs the subcommand. Without `--apply` a subcommand runs every check,
  prints its plan and writes nothing. It never runs migrations and never prints a secret.
  - `provision-fleet --instances <n> --actor <discord id> --ip <address> --secrets-root <path>
    [--game-port-base <port>]`: registers the new servers "TBD Staging 1" to "TBD Staging <n>" at
    `<address>` on game ports base+1 to base+n (base 2000 by default) and issues each a
    `host_agent` and a `mod_runtime` credential, attributed to `--actor`, which must hold
    administrator authority in the `DISCORD_GUILD_ID` guild and lie outside the reserved range.
    Each secret goes to `<secrets root>/instance-<n>/secrets/host-agent-credential` or
    `mod-runtime-credential`, created exclusively with mode 600 in a mode-700 directory; the rows
    and their `server.create` and `server.credential_issued` audit rows commit only once every file
    is written. It refuses a name already registered, an existing file and a secrets directory
    with group or other permission bits. It prints `server instance=<n> id=<uuid> …` and
    `credential instance=<n> executor=<kind> id=<uuid> file=<path>` lines.
  - `rotate-credential --instance <n> --executor host_agent|mod_runtime --secrets-root <path>
    (--stage --actor <discord id> | --promote)`: `--stage` issues a new credential for the server
    "TBD Staging <n>" into `<credential file>.staged` and lists the credentials to revoke
    (`revoke-after-staging credential id=<uuid>`); the operator revokes them in the Server Control
    page. `--promote` checks that the staged secret is an unrevoked credential of that server and
    executor, lists any still active, and renames the staged file over the live one; the next
    restart of the program reads it.
  - `seed-load-fixture-events --mission <uuid>`: creates the open events "[Load fixture] 01" to
    "[Load fixture] 10", the first starting 14 days after the seeding minute and each later one an
    hour after, capped at 128 places and each attaching `<mission>` with a 2 × 8 × 8 ORBAT of 128
    slots, through the services the event routes use (with their `event.created` and
    `event.mission_attached` audit rows). The author is the lowest account of the reserved range,
    the load population's first. It refuses while `DISCORD_BOT_TOKEN` is set in the API env file,
    while no reserved account exists, while a fixture event of a reserved author exists, and for a
    missing, deleted or archived mission; it prints `fixture=<nn> event=<uuid>
    event_mission=<uuid> slots=128` lines.
  - `clean-load-fixture-events`: deletes the "[Load fixture]" events of reserved authors with
    their attachments, slots, registrations and registration history, and appends an
    `event.load_fixture_removed` audit row for each; it deletes no account and keeps every audit
    row.
  - `seed-load-population --accounts <n> --role <discord role name> --account-file <path>
    [--id-base <discord id>]`: creates `<n>` accounts with the Discord ids from the id base (the
    first reserved id by default), each "Load Member <k>" registered like a first sign-in, a
    verified member of the `DISCORD_GUILD_ID` guild holding the named Discord role, and holding a
    refresh-only session, then writes their refresh tokens to `<path>` as
    `{"accounts":[{"discord_id":"…","refresh_token":"…"},…]}` in account order, created
    exclusively with mode 600 in a directory only its owner can enter, and appends a
    `staging.load_population_seeded` audit row. It refuses while `DISCORD_BOT_TOKEN` is set in the
    API env file, while any account of the reserved range exists, for ids leaving the range, and
    for a role name that is not exactly one `discord_roles` row mapped to `enlisted` or to nothing.
    A failure deletes the accounts the run created and the file it wrote. It runs before
    `seed-load-fixture-events`, whose author is the population's first account.
  - `clean-load-population`: deletes every account of the reserved range with its registrations
    (and their history and participation rows), fire missions, link codes, sessions, tokens,
    membership rows and bookmarks, frees its slots, and appends a `staging.load_population_cleaned`
    audit row; the audit rows naming the accounts stay. It runs after `clean-load-fixture-events`.
  - `age-membership-snapshot --discord-id <id> --hours <h>`: sets `verified_at` of the account's
    `DISCORD_GUILD_ID` membership snapshot to `<h>` hours (1 to 168) before now, with a
    `staging.membership_snapshot_aged` audit row, leaving its status, roles, revision and refresh
    schedule as they are. It refuses an account without a verified member snapshot and a snapshot
    whose refresh lease is live.
  - `observe-discord-member --discord-id <id> --guild main|partner [--partner-guild-id <id>]
    [--discord-api-base <url>]`: reads the member once with the bot token of the API env file
    (`GET /guilds/<guild>/members/<id>`) and prints one `discord-member-read {json}` line: the
    request number, guild, ids, send and answer times in Unix milliseconds, the HTTP status, the
    outcome (`member` with its `roles`, `nonmember`, `rate_limited` with `retry_after_ms` and
    `global`, or `unavailable` with its `reason`) and the `rate_limit` headers (`bucket`, `limit`,
    `remaining`, `reset_after_ms`, `scope`). It exits 1 when the read observed no membership.
    `--discord-api-base` is the test-only seam for a fake Discord and accepts an http(s) URL on a
    loopback IP address only; without it the read goes to `https://discord.com/api/v10`.
  - `spend-discord-member-bucket`, with the same target flags and `--start-at-unix-ms <ms>
    --hold-seconds <s> --max-requests <n>`: from the start time (refused when more than 250 ms
    past or more than 15 minutes ahead) reads the member until Discord reports the Get Guild Member
    bucket spent, then reads again only after each reset until the hold (1 to 10 s) ends, never
    more than `<n>` requests (1 to 50). It prints a `discord-member-read` line per request and a
    `discord-bucket-spend {json}` summary (`requests`, `rate_limited_responses`, `bucket_spent`,
    `first_spent_at_unix_ms`, `started_at_unix_ms`, `hold_until_unix_ms`, `stopped_because`), and
    exits 1 when it never saw the bucket spent or Discord answered with neither a membership nor a
    429.
- Exit codes: 0 when the run completed, a dry run included; 1 when an operation failed, with the
  transaction rolled back and the files it wrote removed (the message names any file that stayed);
  2 when the arguments or a guard refused the run and nothing was written, including an unknown
  subcommand or flag, a missing `--confirm-database`, a mismatched database, an unreadable API env
  file, and an actor without administrator authority.
- Example: `target/release/staging-fixtures provision-fleet --instances 5 --actor <operator id>
  --ip 192.0.2.10 --secrets-root ~/tbd/fleet --confirm-database tbd_reforger`, then the same with
  `--apply`.

## Boundaries

- Depends on: the `api` library: `core::configuration`, `core::database`,
  `core::application_state`, `core::http_router` and `core::process_lifecycle` for `api`, with
  `background_workers`;
  `core::database` and `missions::services::registry_import` for `import-registry`;
  `core::database`, `server_infrastructure::services::{server_registration, machine_credentials}`,
  `identity_and_access::services::{account_authority, account_registration,
  discord_membership_cache, session_issuance, discord_client}`,
  `operations::services::event_authoring::{event_creation, mission_attachment}` and
  `administration::services::required_audit` for `staging-fixtures`.
- Used by: `cargo xtask mk rust-api` and `cargo xtask db registry-import`
  (`tools/xtask/src/commands/build/recipes/shell_word.rs` and
  `tools/xtask/src/commands/db/operations.rs`); the `editor-api-boot` task of `cargo xtask ci`;
  the release image built by `deploy/Dockerfile`, whose entry point is `api`; the systemd
  unit `deploy/systemd/tbd-website-api.service`, which runs the release `api`;
  `apps/api/tests/audit_replay_shutdown.rs`, which starts the `api` binary and stops
  it with SIGTERM; `apps/api/tests/staging_fixtures_fleet.rs`,
  `apps/api/tests/staging_fixtures_fixture_events.rs`,
  `apps/api/tests/staging_fixtures_population.rs` and
  `apps/api/tests/staging_fixtures_discord.rs`, which run the `staging-fixtures`
  binary.
- Rules: `api.rs` is the one file that arms `background_workers`
  (`background_workers_used_only_by_the_binary` in
  `apps/api/src/tests/architecture_rules.rs`); the binary names are stable, because the
  xtask recipes, the Dockerfile and the systemd unit call them by name; a new binary adds its
  `[[bin]]` table and its file in the same change.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — running the
  [API](/documentation/glossary/a_to_f.md#api) and importing the registry locally.
- [Website deployment](/documentation/runbooks/website_deployment.md) — building and running
  the release `api` on the home server.
