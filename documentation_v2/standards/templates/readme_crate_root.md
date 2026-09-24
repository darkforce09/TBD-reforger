**Status:** live

# README template: crate, package or addon root

**When to use:** the folder that holds a `Cargo.toml`, a `package.json` or an Enfusion
`addon.gproj`. The [README standard](/documentation_v2/standards/readme_standard.md) defines every
rule this template follows; the root kind adds Getting started, Configuration and Public surface.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there.

````markdown
# <Product name, in plain words: no path, no backticks>

<One to three sentences: what the crate, package or addon is, with its package name, and who uses
it.>

## Contents

```text
<repository path of the folder>/
├── <manifest file>  <the package: name, targets and what it links>
├── <config file>    <what it configures>
├── src/             <what the source tree holds, in one phrase>
└── tests/           <what the integration tests cover>
```

## How it works

<What happens from entry point to running product: the boot or build sequence, the main layers of
the source tree, and the invariants that span them. Name each child's part in one clause.>

## Getting started

<The commands, run from the repository root, that build, run and test it, in the order they must
run, each with what to expect; say which stay in the foreground and what a later command waits
for. Prefer an xtask recipe wherever one exists. Link the runbook for the full procedure.>

## Configuration

<Every setting it reads: environment variables, config files and feature flags, each with its
default, whether it is required, and the file that reads it.>

## Public surface

- <library module, binary, route prefix or command>: <what it offers and who uses it>

## Boundaries

- Depends on: <the crates, schemas, services and files it uses>
- Used by: <every crate, tool, service or client that uses it, found with git grep>
- Rules: <the invariants a change here must keep, and the test or gate that checks each>

## Related documentation

- [<document title>](/documentation_v2/<path to the document>) — <what it covers>
````

## Worked sample

Written from `apps/website/api_v2/`. The sample sits in a fenced block, so no gate reads it as a
README; the folder's own README.md is written from the same code and may differ.

````markdown
# Website API

The `website-api` crate: the Axum REST API and Server-Sent Events hub behind the web platform. It
serves `/api/v1` to the single-page app, the game servers and the fleet host agent, owns the
Postgres schema through its migrations, and serves uploads and terrain assets.

## Contents

```text
apps/website/api_v2/
├── .env.example         the template the gitignored `.env` is copied from, with development values
├── .gitignore           keeps `.env` and editor folders out of git
├── Cargo.toml           the `website-api` package: its library and `api` and `import-registry` bins
├── docker-compose.yml   the local Postgres 18 service `db`, container `tbd_reforger_db`, port 5434
├── migrations/          the SQL schema migrations, embedded at compile time and applied at boot
├── rust-toolchain.toml  pins Rust 1.95.0 with rustfmt and clippy
├── rustfmt.toml         the formatting settings: edition 2024, 100-column lines
├── seeds/               the development seeds `cargo xtask db seed` applies, and hand-applied sets
├── src/                 the library: `core`, the workers, the eight domains, the binaries
└── tests/               integration suites against a real Postgres, with their shared support
```

## How it works

`src/bin/api.rs` loads the configuration, opens the Postgres pool, applies `migrations/` unless
`SKIP_MIGRATE` is set, arms the background workers and serves the router on `0.0.0.0:$PORT`. The
router nests the eight domains' route tables under `/api/v1` and wraps everything in one middleware
chain, outermost first: request id, access log, Prometheus metrics, panic recovery, CORS, body
limit, rate limit. The terrain and glyph mounts under `/map-assets` sit below the rate limit. Each
domain under `src/` owns its handlers, services, models and route table, and `core` imports no
domain except where the router and the application state compose them.

## Getting started

Copy `.env.example` to `.env`, then run these from the repository root, in this order:

```bash
cargo xtask db up        # Postgres 18 in the tbd_reforger_db container, host port 5434
cargo xtask mk rust-api  # cargo run --bin api in this folder; migrates, stays in the foreground
cargo xtask db seed      # a second terminal, once the API logs `migrations applied`
cargo xtask db test-it   # the integration suites against a scratch rust_it database
```

`db seed` applies the five development seeds in dependency order to tables that only the API's
migrations create. psql carries on past a failed statement, so seeding before the API's first boot
loads nothing and still exits 0. `db test-it` needs only `db up`: it creates the scratch database,
and each suite applies the migrations itself.

With `APP_ENV=development`, `GET /api/v1/auth/dev-login?role=<role>` signs in without Discord
(`guest`, `enlisted`, `leader`, `mission_maker` or `admin`; any other value signs in as `admin`) and
redirects to the app's `/auth/callback` with the session in the URL fragment.

## Configuration

`Config::load` in `src/core/configuration/mod.rs` reads the process environment, then `.env`; an
exported variable wins. `DATABASE_URL` and `JWT_SECRET` are always required. Outside development,
`DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` and an absolute `UPLOAD_DIR` are
required too, and a set `EQUIPMENT_DATA_DIR` or `EQUIPMENT_EXPORT_SOURCE_DIR` must be absolute.

| Group | Variables |
|---|---|
| Server | `PORT` (8080), `APP_ENV` (`production` unless set; `development` enables dev-login), `FRONTEND_URL`, `ALLOWED_ORIGINS`, `TRUSTED_PROXIES` |
| Database | `DATABASE_URL`, and the four `TBD_DB_POOL_*` pool settings |
| Files | `SPA_DIST_DIR` (serve the built app when set), `MAP_ASSETS_DIR`, `GLYPH_ASSETS_DIR`, `UPLOAD_DIR` |
| Equipment viewer | `EQUIPMENT_DATA_DIR` (the imported exports and their indexes; `../../../assets_v2/equipment` in development), `EQUIPMENT_EXPORT_SOURCE_DIR` (an optional Workbench publication folder the import worker checks; unset, the viewer browses what is already imported) |
| Request limits | `MISSION_VERSION_MAX_BODY_BYTES` (256 MiB), the body limit of the mission version save route alone |
| Sessions and Discord | `JWT_SECRET`, `JWT_ACCESS_TTL_MIN` (15), `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL`, `DISCORD_GUILD_ID`, `DISCORD_BOT_TOKEN`, `DISCORD_WEBHOOK_URL` |
| Service token | `SERVICE_TOKEN`, which game-server ingest, `/metrics` and the detailed `/healthz` check |
| Workers | `SERVER_STATUS_PUBLISH_INTERVAL_SECS`, `LEADERBOARD_REFRESH_INTERVAL_SECS`, `ROLE_RESYNC_INTERVAL_SECS` |
| Boot | `SKIP_MIGRATE` (skip the migrations), `RUST_LOG` (the log filter, `info` when unset) |

## Public surface

- The library `website_api` (`src/lib.rs`): `core`, `background_workers` and the eight domain
  modules. Its users are this crate's binaries and integration suites.
- The `api` binary: the server described above.
- The `import-registry` binary: ingests registry envelopes (`--items`, `--compat`) into Postgres for
  the envelope's modpack, which `--modpack` overrides; `--prune` deletes that modpack's rows the
  envelope lacks.
- The HTTP surface: `/api/v1`, `/healthz`, `/metrics` (service token), `/uploads`, `/map-assets` and
  `/map-assets/glyphs`, and the built app as the fallback when `SPA_DIST_DIR` is set.

## Boundaries

- Depends on: `website-map-engine` with its default `scenario` tier, which compiles and validates
  missions; the schemas in `contracts_v2/definitions/`, embedded at compile time; Postgres 18;
  Discord's OAuth2 and REST APIs; and, at run time, the asset trees in `assets_v2/terrains/` and
  `assets_v2/glyphs/`.
- Used by: the single-page app in `apps/website/frontend/`; the game servers, through the mod's
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the fleet host agent in `apps/fleet_host_agent/`;
  the `mk rust-api`, `db` and `deploy website` commands of `tools_v2/xtask/`; and the release image
  that `apps/website/Dockerfile` builds.
- Rules: `core` imports no domain except in `src/core/application_state.rs` and
  `src/core/http_router.rs`, a domain's handlers never import another domain's handlers, and
  `background_workers` is imported only by `src/bin/api.rs` (`src/tests/architecture_rules.rs`
  checks all three); an applied migration never changes (`tests/migrations_are_immutable.rs`).

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — the routes of every domain and
  the layers they share.
- [Local development](/documentation_v2/runbooks/local_development.md) — the full local setup.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — building and running the
  API on the home server.
````
