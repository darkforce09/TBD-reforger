# Staging fixtures host tool

The `staging_fixtures` crate: the `staging-fixtures` executable, the host tool that stages the
fixtures of a staging verification run (the fleet's servers and machine credentials, the load
population and its fixture events, the Discord procedure's aged snapshot and member reads) in the
staging database and on the staging host, behind one set of guards. It writes through the API's own
services, which makes it the one tool crate the crate-tier law lets depend on the api crates.

## Contents

```text
tools/staging/staging_fixtures/
├── Cargo.toml  the `staging_fixtures` binary package (`staging-fixtures`): the api crates, sqlx, reqwest and rustls, layout tier 10
└── src/        the subcommand table, the guards, the fleet, load and Discord subcommands and the secret files
```

## How it works

```text
argv ─▶ the subcommand's flags parse ─▶ the API env file's DATABASE_URL ─▶ api_database::connect
     ─▶ current_database() equals --confirm-database ─▶ the subcommand: a dry run, or its writes
        with --apply, through the api crates' services
```

`src/README.md` describes the guards, each subcommand and the secret files.

## Getting started

Run from the repository root:

```bash
cargo build --release -p staging_fixtures --bin staging-fixtures   # target/release/staging-fixtures, as the website deploy builds it on the host
```

## Configuration

Every setting of a run comes from the tool's flags and from the API's settings file,
`deploy/api.env` relative to the working directory unless `--api-env-file` names another, read without touching the process environment:

| Key | Default | Required | Read by |
|---|---|---|---|
| `DATABASE_URL` | none | yes | `src/main.rs` |
| `DISCORD_GUILD_ID` | empty | for the fleet actor check, the load population, the snapshot aging and the main-guild member reads | `src/guarded_context.rs`, `src/load_population/population_seeding.rs`, `src/membership_aging.rs`, `src/discord_member_probe/discord_target.rs` |
| `DISCORD_BOT_TOKEN` | empty | for the Discord member reads; both seedings refuse while it is set | `src/discord_member_probe/discord_target.rs`, `src/load_population/population_seeding.rs`, `src/load_fixture_events/fixture_seeding.rs` |

The process environment's `RUST_LOG` sets the filter of the services' error log on stderr (`warn`
when unset; `src/main.rs`). No feature.

## Public surface

The `staging-fixtures` executable, run on the staging host from the checkout root, where its
default API env file resolves; the staging procedures run it as
`cd <checkout> && ./target/release/staging-fixtures <subcommand> …`.

### staging-fixtures

- Synopsis: `staging-fixtures <subcommand> [flags] --confirm-database <name> [--apply]
  [--api-env-file <path>]`, and `staging-fixtures --help`, which lists the subcommands.
- Does: reads the API's settings file (`deploy/api.env` relative to the working directory
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

- Depends on: the api crates `api_audit_log`, `api_caller_identity`, `api_database`,
  `api_discord`, `api_foundation`, `api_http_layer`, `api_identifiers`,
  `api_identity_and_access`, `api_operations` and `api_server_infrastructure`; `fleet_wire_contract`
  and `mission_model`; `time_source` (the member reads' clock); `repository_layout` (the
  default API env file path); sqlx, reqwest, rustls, dotenvy,
  chrono, tokio and `tracing-subscriber`. The
  suites add `api_configuration`, `api_state`, `api_missions`, `api_equipment_datasets`, axum,
  tower and `tower-http`.
- Used by: the operator on the staging host; the staging procedures of `staging_procedures`
  (`src/remote_actions/host_fixture_commands.rs`, the local load rehearsal's
  `cargo run -p staging_fixtures --bin staging-fixtures`); `cargo xtask deploy website`, which
  builds it on the host (`tools/commands/deployment/src/website/remote_steps.rs`).
- Rules: tier 10 of `tools/staging`, the only tool crate with api crate edges
  (`STAGING_FIXTURES_PATH` in `tools/foundation/repository_laws/src/workspace_laws/crate_layout.rs`);
  it never depends on `crates/api/api_server`; the executable name `staging-fixtures` and its release path
  `target/release/staging-fixtures` are stable, because the staging procedures and the deploy call
  them; test functions start with `staging_fixtures_`, which the readiness register's
  `staging_fixture_tool` check counts.

## Related documentation

- [Staging tool crates](/tools/staging/README.md) — the staging crates side by side.
- [Staging design note](/documentation/crates/api/api_server/design_notes/staging.md) — the staging
  procedures and the cases the host tool's runs feed.
- [Machine credentials and mission deployment](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)
  — provisioning the fleet with `provision-fleet` and rotating credentials.
- [Website deployment](/documentation/runbooks/website_deployment.md) — the deploy step that builds
  the executable on the host.
