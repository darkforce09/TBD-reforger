# Website API

The `website-api` crate: the Axum REST [API](/documentation_v2/glossary/a_to_f.md#api) and
[SSE](/documentation_v2/glossary/n_to_z.md#sse) streams behind the web platform. It serves `/api/v1` to
the single-page app, the game servers and the
[fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent), owns the Postgres schema
through its migrations, and serves uploads and terrain assets.

## Contents

```text
apps/website/api_v2/
├── .env.example         the template the gitignored `.env` is copied from, with development values
├── .gitignore           keeps `.env` and editor folders out of git
├── Cargo.toml           the `website-api` package: the library and its two binaries
├── docker-compose.yml   the local Postgres 18 service `db`, container `tbd_reforger_db`, port 5434
├── migrations/          the SQL schema migrations, embedded at compile time and applied at boot
├── rust-toolchain.toml  pins Rust 1.95.0 with rustfmt and clippy
├── rustfmt.toml         the formatting settings: edition 2024, 100-column lines
├── seeds/               the development seeds `cargo xtask db seed` applies, and hand-applied data
├── src/                 the library: `core`, the background workers, the eight domains, the binaries
└── tests/               integration suites against real Postgres, with their shared support
```

## How it works

`src/bin/api.rs` loads the configuration, opens the Postgres pool, applies `migrations/` unless
`SKIP_MIGRATE` is set, arms the [background workers](/documentation_v2/glossary/a_to_f.md#background-workers)
and serves the router on `0.0.0.0:$PORT` until SIGINT or SIGTERM. The router nests the eight
domains' route tables under `/api/v1` and wraps everything in one middleware chain, outermost
first: request id, access log, Prometheus metrics, panic recovery, CORS, body limit, rate limit.
The terrain and glyph mounts under `/map-assets` sit below the rate limit. Each domain under
`src/` owns its handlers, services, models and route table, and `core` imports no domain except
where the router and the application state compose them.

A route's access tier is the extractor its handler takes: a signed-in member, a member of at
least a given [role](/documentation_v2/glossary/n_to_z.md#role), or the `X-Service-Token` of game-server
ingest; the [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) and the fleet host agent
authenticate with per-server [machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential).
The [mission](/documentation_v2/glossary/g_to_m.md#mission) compiler and the mortar ballistics come from
`website-map-engine`, which the crate takes with its default `scenario` tier alone. Game servers
fetch compiled mission [artifacts](/documentation_v2/glossary/a_to_f.md#artifact) over HTTPS from
`/api/v1/game-runtime/artifacts/{artifactId}`; nothing is staged on disk for them.

The integration suites in `tests/` build one test binary per top-level file. A binary that needs
a database derives its own scratch database from `TEST_DATABASE_URL`, creates and migrates it,
and the suites that drive HTTP build the same router the `api` binary serves. Shared support
lives in `tests/common/` and the `*_support/` folders, which produce no binary of their own.

## Getting started

Copy `apps/website/api_v2/.env.example` to `apps/website/api_v2/.env` (a fresh git worktree has
none), then run these from the repository root, in this order:

```bash
cargo xtask db up        # Postgres 18 in the tbd_reforger_db container, host port 5434, detached
cargo xtask mk rust-api  # cargo run --bin api in this folder; migrates, stays in the foreground
cargo xtask db seed      # a second terminal, once the API logs `migrations applied`
cargo xtask db test-it   # the integration suites, each against its own scratch database
cargo xtask mk rust-test # the library and binary unit tests, source rules included; no database
```

`db seed` applies five development seeds in a fixed order to tables that only the API's
migrations create. `psql` carries on past a failed statement, so seeding before the API's first
boot loads nothing and still exits 0. `db test-it` needs only `db up`: it creates the scratch
databases, and each suite applies the migrations itself. `cargo xtask db registry-import` loads
the committed Workbench [registry](/documentation_v2/glossary/n_to_z.md#registry) exports into the local
database, `cargo xtask mk leptos` serves the single-page app on port 3000, proxying `/api` and
`/map-assets` to the API, `cargo xtask db down` stops Postgres and keeps its volume, and
`cargo xtask ci ci-local` replays the whole CI suite once `db up` has run.

With `APP_ENV=development`, the [dev login](/documentation_v2/glossary/a_to_f.md#dev-login)
`GET /api/v1/auth/dev-login?role=<role>` signs in without Discord
(`guest`, `enlisted`, `leader`, `mission_maker` or `admin`; any other value signs in as `admin`)
and answers with a 302 to the app's `/auth/callback`, the session's `access_token` in the URL
fragment; outside development the route answers 404.

## Configuration

`Config::load` in `src/core/configuration/mod.rs` reads the process environment, then the first
`.env` found from the working directory upward; an exported variable wins. `DATABASE_URL` and
`JWT_SECRET` are always required. Outside development, `DISCORD_CLIENT_ID`,
`DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` and an absolute `UPLOAD_DIR` are required too. A
value that is set but unusable stops the boot for `UPLOAD_DIR`, `DISCORD_BOT_TOKEN`,
`TRUSTED_PROXIES` and the pool settings. `.env.example` carries most variables with development
values; `TRUSTED_PROXIES`, `MISSION_VERSION_MAX_BODY_BYTES`, `SKIP_MIGRATE`, `RUST_LOG` and
`TEST_DATABASE_URL` are not in it.

| Variable | Default | Required | Read by |
|---|---|---|---|
| `PORT` | `8080` | no | `Config::load` |
| `APP_ENV` | `production`; `development` enables dev login and the development defaults | no | `Config::load` |
| `FRONTEND_URL` | `http://localhost:5173`; where sign-in redirects | no | `Config::load` |
| `ALLOWED_ORIGINS` | the value of `FRONTEND_URL`; a comma-separated CORS allow-list | no | `Config::load` |
| `TRUSTED_PROXIES` | empty, which trusts no `X-Forwarded-For`; comma-separated addresses and CIDR blocks | no | `Config::load` |
| `SPA_DIST_DIR` | empty; when set, the built app is served with an `index.html` fallback | no | `Config::load` |
| `MAP_ASSETS_DIR` | `../../../assets_v2/terrains`, relative to the working directory | no | `Config::load` |
| `GLYPH_ASSETS_DIR` | `../../../assets_v2/glyphs`, relative to the working directory | no | `Config::load` |
| `UPLOAD_DIR` | `../../../assets_v2/scratch/website-api/uploads` in development; the systemd unit sets its state directory | outside development, absolute | `Config::load` |
| `DATABASE_URL` | none | yes | `Config::load`; `import-registry` |
| `TBD_DB_POOL_MAX_CONNECTIONS` | `25` | no | `src/core/database/connection_pool.rs` |
| `TBD_DB_POOL_IDLE_TIMEOUT_SECS` | `300` | no | `src/core/database/connection_pool.rs` |
| `TBD_DB_POOL_MAX_LIFETIME_SECS` | `1800` | no | `src/core/database/connection_pool.rs` |
| `TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS` | `30` | no | `src/core/database/connection_pool.rs` |
| `MISSION_VERSION_MAX_BODY_BYTES` | 256 MiB; the body limit of the mission version save route alone | no | `Config::load` |
| `JWT_SECRET` | none; signs the access tokens | yes | `Config::load` |
| `JWT_ACCESS_TTL_MIN` | `15` | no | `Config::load` |
| `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` | empty | outside development | `Config::load` |
| `DISCORD_GUILD_ID` | empty; the guild whose roles decide members' roles | no | `Config::load` |
| `DISCORD_BOT_TOKEN` | empty, meaning no bot; a value holding whitespace stops the boot | no | `Config::load` |
| `DISCORD_WEBHOOK_URL` | empty, which disables announcement pushes | no | `Config::load` |
| `SERVICE_TOKEN` | empty, which refuses every `X-Service-Token` route | no | `Config::load` |
| `SERVER_STATUS_PUBLISH_INTERVAL_SECS` | `10` | no | `src/background_workers/server_status_publisher.rs` |
| `LEADERBOARD_REFRESH_INTERVAL_SECS` | `900` | no | `src/background_workers/leaderboard_refresher.rs` |
| `ROLE_RESYNC_INTERVAL_SECS` | `86400` | no | `src/background_workers/discord_role_synchronizer.rs` |
| `SKIP_MIGRATE` | unset; any value skips the migrations at boot, for a harness that migrates a shared database itself | no | `src/bin/api.rs` |
| `RUST_LOG` | `info`; the log filter | no | `src/bin/api.rs` |
| `TEST_DATABASE_URL` | none; `cargo xtask db test-it` sets it | for the database suites | `tests/common/database.rs` |

## Public surface

- The library `website_api` (`src/lib.rs`): `core`, `background_workers` and the eight domain
  modules. Its users are this crate's binaries and integration suites.
- The `api` binary: the server described above.
- The `import-registry` binary: imports Workbench registry envelopes (`--items`, `--compat`) into
  Postgres for the envelope's modpack, which `--modpack` overrides; `--prune` deletes that
  modpack's rows the envelope lacks.
- The HTTP surface: `/api/v1`, `/healthz`, `/metrics` (service token), `/uploads`, `/map-assets`
  and `/map-assets/glyphs`, and the built app as the fallback when `SPA_DIST_DIR` is set.

## Boundaries

- Depends on: `website-map-engine` with its default `scenario` tier, which compiles and validates
  missions and solves fire missions; the schemas in
  `contracts_v2/definitions/`, embedded at compile time; Postgres 18; Discord's OAuth2 and REST
  APIs and a channel webhook; and, at run time, the asset trees in `assets_v2/terrains/` and
  `assets_v2/glyphs/`.
- Used by:
  - the single-page app in `apps/website/frontend/`, whose Trunk server proxies `/api` and
    `/map-assets` to the API on `127.0.0.1:8080` in development;
  - the game servers, through the mod's `apps/mod/tbd-framework/Scripts/Game/TBD/API/`, and the
    fleet host agent in `apps/fleet_host_agent/`;
  - the `mk rust-api`, `db`, `ci` and `deploy website` commands of `tools_v2/xtask/`;
  - the release image that `apps/website/Dockerfile` builds, the optional `api` service of
    `apps/website/docker-compose.staging.yml`, and the systemd unit
    `tools_v2/xtask/deploy/systemd/tbd-website-api.service`.
- Rules: `core` imports no domain except in `src/core/application_state.rs` and
  `src/core/http_router.rs`, a domain's handlers never import another domain's handlers, and
  `background_workers` is imported only by `src/bin/api.rs` (`src/tests/architecture_rules.rs`
  checks all three); an applied migration never changes (`tests/migrations_are_immutable.rs`);
  `rust-toolchain.toml` pins the same toolchain as the workspace root's `rust-toolchain.toml`; the
  filled `.env` is never committed.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — the routes of every domain
  and the layers they share.
- [API environment variables](/documentation_v2/website/api_v2/environment_variables.md)
  — every variable the API reads, with
  its default, requirement and failure mode.
- [API decisions](/documentation_v2/website/api_v2/decisions.md) — the cross-domain design
  decisions and their consequences.
- [Local development](/documentation_v2/runbooks/local_development.md) — the full local setup,
  Discord sign-in included.
- [Database operations](/documentation_v2/runbooks/database_operations.md) — the integration
  tests, the migration checksum repair, sample data, backups and restores.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — building and running
  the API on the home server.
- [API completion and verification](/documentation_v2/website/api_v2/verification_evidence/completion_plan.md)
  — the acceptance contract and requirement register the API is verified against.
- [API verification evidence](/documentation_v2/website/api_v2/verification_evidence/README.md)
  — the index of the register, the program records and the design notes.
