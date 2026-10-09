**Status:** live

# README template: crate, package or addon root

**When to use:** the folder that holds a `Cargo.toml`, a `package.json` or an Enfusion
`addon.gproj`. The [README standard](/documentation/standards/readme_standard.md) defines every
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

- [<document title>](/documentation/<path to the document>) — <what it covers>
````

## Worked sample

Written from `apps/api/`. The sample sits in a fenced block, so no gate reads it as a
README; the folder's own README.md is written from the same code and may differ.

````markdown
# Website API

The `api` crate: the Axum REST API and Server-Sent Events streams behind the web platform,
assembled from the API crates under `crates/api/`. It serves `/api/v1` to the single-page app, the
game servers and the fleet host agent, applies the Postgres schema migrations of
`crates/api/api_database/` at boot, and serves uploads and terrain assets.

## Contents

```text
apps/api/
├── .env.example  the template the gitignored `.env` is copied from, with development values
├── Cargo.toml    the `api` package: the library and its two binaries
├── src/          the thin application: the router, the composition root, the binaries, the layout rules
└── tests/        integration suites against real Postgres, with their shared support
```

## How it works

`src/bin/api.rs` loads the configuration, opens the Postgres pool, applies the migrations unless
`SKIP_MIGRATE` is set, arms the background workers and serves the router on `0.0.0.0:$PORT`. The
router (`src/router.rs`) nests the eight domains' route tables under `/api/v1` and wraps
everything in one middleware chain, outermost first: request id, access log, Prometheus metrics,
panic recovery, CORS, body limit, rate limit. The terrain and glyph mounts under `/map-assets` sit
below the rate limit. The application holds no domain logic: each domain is an API crate
(`crates/api/api_<domain>/`) that owns its handlers, services, models and route table, the kernel
crates below them depend on no domain, and the background workers are their own crate
(`crates/api/api_background_workers/`), which only the `api` binary arms.

## Getting started

Copy `apps/api/.env.example` to `apps/api/.env`, then run these from the repository root, in this
order:

```bash
cargo xtask db up        # Postgres 18 in the tbd_reforger_db container, host port 5434
cargo xtask mk rust-api  # cargo run --bin api in this folder; migrates, stays in the foreground
cargo xtask db seed      # a second terminal, once the API logs `migrations applied`
cargo xtask db test-it   # the integration suites, each against its own scratch database
```

`db seed` applies the five development seeds in dependency order to tables that only the API's
migrations create. Each psql run stops at the first failed statement, so seeding before the API's
first boot stops at the first seed with psql's exit code 3. `db test-it` needs only `db up`: it
creates the scratch databases, and each suite applies the migrations itself.

With `APP_ENV=development`, `GET /api/v1/auth/dev-login?role=<role>` signs in without Discord
(`guest`, `enlisted`, `leader`, `mission_maker` or `admin`; any other value signs in as `admin`) and
redirects to the app's `/auth/callback` with the session in the URL fragment.

## Configuration

`Config::load` in `crates/api/api_configuration/src/configuration/mod.rs` reads the process
environment, then the first `.env` found from the working directory upward; an exported variable
wins. `DATABASE_URL` and `JWT_SECRET` are always required. Outside development,
`DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` and an absolute `UPLOAD_DIR`
are required too.

| Group | Variables |
|---|---|
| Server | `PORT` (8080), `APP_ENV` (`production` unless set; `development` enables dev-login), `FRONTEND_URL`, `ALLOWED_ORIGINS`, `TRUSTED_PROXIES` |
| Database | `DATABASE_URL`, and the four `TBD_DB_POOL_*` pool settings |
| Files | `SPA_DIST_DIR` (serve the built app when set), `MAP_ASSETS_DIR`, `GLYPH_ASSETS_DIR`, `UPLOAD_DIR` |
| Sessions and Discord | `JWT_SECRET`, `JWT_ACCESS_TTL_MIN` (15), `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL`, `DISCORD_GUILD_ID`, `DISCORD_BOT_TOKEN`, `DISCORD_WEBHOOK_URL` |
| Boot | `SKIP_MIGRATE` (skip the migrations), `RUST_LOG` (the log filter, `info` when unset) |

## Public surface

- The library `api` (`src/lib.rs`): `router::router`, the whole application with its middleware
  chain, and `composition::application_state`, the application state with its concrete services.
  Its users are this crate's binaries and integration suites.
- The `api` binary: the server described above.
- The `import-registry` binary: ingests registry envelopes (`--items`, `--compat`) into Postgres for
  the envelope's modpack, which `--modpack` overrides; `--prune` deletes that modpack's rows the
  envelope lacks.
- The HTTP surface: `/api/v1`, `/healthz`, `/metrics` (`OBSERVABILITY_TOKEN` bearer), `/uploads`,
  `/map-assets` and `/map-assets/glyphs`, and the built app as the fallback when `SPA_DIST_DIR` is set.

## Boundaries

- Depends on: the API crates of `crates/api/` (the eight domain crates, whose route tables the
  router merges; `api_background_workers`, which the `api` binary arms; the kernel crates the
  composition root assembles); through them, Postgres 18, Discord's OAuth2 and REST APIs, the
  schemas in `contracts/definitions/` and, at run time, the asset trees in `assets/terrains/` and
  `assets/glyphs/`.
- Used by: the single-page app in `apps/frontend/`; the game servers, through the mod's
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the fleet host agent in `apps/fleet_host_agent/`;
  the `mk rust-api`, `db` and `deploy website` commands of `tools/xtask/`; and the release image
  that `deploy/Dockerfile` builds.
- Rules: `src/` holds only the thin application; the kernel crates depend on no domain, no crate
  imports another domain's handlers, and only `src/bin/api.rs` names `api_background_workers`
  (`src/tests/architecture_rules.rs` checks all three); an applied migration never changes
  (`tests/migrations_are_immutable.rs`).

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — the routes of every domain and
  the layers they share.
- [Local development](/documentation/runbooks/local_development.md) — the full local setup.
- [Website deployment](/documentation/runbooks/website_deployment.md) — building and running the
  API on the home server.
````
