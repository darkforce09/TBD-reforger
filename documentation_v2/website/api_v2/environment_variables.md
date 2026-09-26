**Status:** live

# API environment variables

Every environment variable the website [API](/documentation_v2/glossary/a_to_f.md#api) reads, as the
code reads it: its default, when it is required, what an unusable value does, and the file that
reads it. Developers filling `apps/website/api_v2/.env` and operators setting up a host read it;
where `apps/website/api_v2/.env.example` and the code disagree, this reference follows the code.

## Where it lives

- Code: [`apps/website/api_v2/src/core/configuration/mod.rs`](/apps/website/api_v2/src/core/configuration/mod.rs)
  (`Config::load` and its validation), `proxy_network.rs` beside it (the `TRUSTED_PROXIES`
  parser), [`apps/website/api_v2/src/core/database/connection_pool.rs`](/apps/website/api_v2/src/core/database/connection_pool.rs)
  (the pool settings), three workers in
  [`apps/website/api_v2/src/background_workers/`](/apps/website/api_v2/src/background_workers/)
  (their intervals), and [`apps/website/api_v2/src/bin/api.rs`](/apps/website/api_v2/src/bin/api.rs)
  (`SKIP_MIGRATE`, `RUST_LOG`).
- Entry: the template [`apps/website/api_v2/.env.example`](/apps/website/api_v2/.env.example),
  copied to the gitignored `apps/website/api_v2/.env`.
- Related: the [configuration README](/apps/website/api_v2/src/core/configuration/README.md), the
  crate README's [Configuration](/apps/website/api_v2/README.md#configuration) table, the
  [local development runbook](/documentation_v2/runbooks/local_development.md) and the
  [website deployment runbook](/documentation_v2/runbooks/website_deployment.md).

## Behaviour

### How values are read

1. `Config::load` loads `.env` best-effort with `dotenvy`: the first `.env` found in the working
   directory or above it. A variable already exported in the process environment wins over the
   file. The `api` binary runs from `apps/website/api_v2/`, so that folder's `.env` is the one it
   reads; a fresh git worktree has none until it is copied in.
2. It reads each variable once, at boot. A string variable that is unset or empty takes its
   default; a number that does not parse takes its default.
3. `Config::validate` then refuses the boot on the rules below, naming the variable.
4. `core::database::connect` reads the four `TBD_DB_POOL_*` variables when it opens the pool,
   and `spawn_all` reads the three worker intervals when it arms the workers.

### What stops the boot

- `DATABASE_URL` or `JWT_SECRET` empty: `DATABASE_URL is required`, `JWT_SECRET is required`.
- Outside development, `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET` or `DISCORD_REDIRECT_URL`
  empty or only whitespace.
- `UPLOAD_DIR` empty outside development, with leading or trailing whitespace anywhere, or
  relative outside development.
- `DISCORD_BOT_TOKEN` set and holding whitespace.
- A `TRUSTED_PROXIES` entry that is not an address or a CIDR block written as its network address
  (`10.0.0.5/8` is refused); the error quotes the entry.
- A `TBD_DB_POOL_*` value set but not a whole number, or a pool ceiling below 1; the error quotes
  the value.

Every other variable degrades instead: an empty Discord or webhook setting turns off the path
that needs it, and a malformed worker interval or token lifetime falls back to its default.

### Development and production

`APP_ENV=development` is the only value that means development; any other value, or none,
behaves as production. Development:

- registers `GET /api/v1/auth/dev-login` (the [dev login](/documentation_v2/glossary/a_to_f.md#dev-login));
  outside development the route is not registered, and a session a dev login issued stops
  authorizing;
- drops `; Secure` from the `oauth_state` cookie, so Discord sign-in works over plain HTTP;
- keeps blank Discord credentials legal and fills the `UPLOAD_DIR` default;
- refuses to start the Discord flow when `FRONTEND_URL` and `DISCORD_REDIRECT_URL` name different
  hosts (`apps/website/api_v2/src/identity_and_access/handlers/oauth_host_guard.rs`); production
  logs one warning instead, since a split-host deployment is legitimate there.

### Known discrepancies

- `.env.example` says a blank `DISCORD_GUILD_ID` writes an `auth.role_sync_skipped` WARN audit
  row (`apps/website/api_v2/.env.example:122-124`) — the code writes none: with a blank guild,
  sign-in takes no membership lease and reads no guild membership, so the member's stored role
  stays as it is (`claim_membership_refresh` in
  `apps/website/api_v2/src/identity_and_access/services/discord_membership_cache.rs`, and
  `apps/website/api_v2/src/identity_and_access/handlers/discord_oauth.rs:173-180`).
- `.env.example` ties the `/map-assets` mount to `SPA_DIST_DIR`
  (`apps/website/api_v2/.env.example:40-41`) — the router always mounts `/map-assets` and
  `/map-assets/glyphs` (`apps/website/api_v2/src/core/http_router.rs`); `SPA_DIST_DIR` adds only
  the built app and the cross-origin isolation headers.
- `.env.example` omits five variables the code reads: `TRUSTED_PROXIES`,
  `MISSION_VERSION_MAX_BODY_BYTES`, `SKIP_MIGRATE`, `RUST_LOG` and `TEST_DATABASE_URL`.

## Data

Paths in defaults are relative to the working directory, which is `apps/website/api_v2/` for
`cargo xtask mk rust-api`. "Config" is `Config::load` in
`apps/website/api_v2/src/core/configuration/mod.rs`.

### Server and frontend

| Variable | Default | Required | Read by | Meaning |
|---|---|---|---|---|
| `PORT` | `8080` | no | Config | the port the API binds on `0.0.0.0` |
| `APP_ENV` | `production` | no | Config | `development` turns on the development behaviour above |
| `FRONTEND_URL` | `http://localhost:5173` | no | Config | the app's origin; sign-in and dev login redirect to its `/auth/callback` with the session in the URL fragment |
| `ALLOWED_ORIGINS` | the value of `FRONTEND_URL` | no | Config | comma-separated CORS allow-list; an `Origin` outside it gets no CORS headers |
| `TRUSTED_PROXIES` | empty: trust none | no | Config | comma-separated addresses and CIDR blocks whose `X-Forwarded-For` the rate limiter believes; a bare address is one host |
| `SPA_DIST_DIR` | empty: serve no app | no | Config | the built app to serve with an `index.html` fallback and the cross-origin isolation headers |

The template sets `FRONTEND_URL=http://localhost:3000` and both local origins in
`ALLOWED_ORIGINS`, matching the Trunk server of `cargo xtask mk leptos`.

### Assets and runtime storage

| Variable | Default | Required | Read by | Meaning |
|---|---|---|---|---|
| `MAP_ASSETS_DIR` | `../../../assets_v2/terrains` | no | Config; the default applies in `apps/website/api_v2/src/core/http_router.rs` | the terrain tree served at `/map-assets`; a missing directory logs a warning at boot |
| `GLYPH_ASSETS_DIR` | `../../../assets_v2/glyphs` | no | the same | the glyph atlas served at `/map-assets/glyphs` |
| `UPLOAD_DIR` | `../../../assets_v2/scratch/website-api/uploads` in development; none otherwise | outside development, absolute | Config | where the CMS upload writes and `/uploads` serves from; the router creates it at boot |

### Database

| Variable | Default | Required | Read by | Meaning |
|---|---|---|---|---|
| `DATABASE_URL` | none | yes | Config; `apps/website/api_v2/src/bin/import_registry.rs` | the Postgres URL; locally `postgres://tbd:tbd@localhost:5434/tbd_reforger?sslmode=disable` |
| `TBD_DB_POOL_MAX_CONNECTIONS` | `25` | no | `DbPoolConfig::from_env` in `connection_pool.rs` | pool ceiling, at least 1 |
| `TBD_DB_POOL_IDLE_TIMEOUT_SECS` | `300` | no | the same | seconds a connection may sit idle |
| `TBD_DB_POOL_MAX_LIFETIME_SECS` | `1800` | no | the same | seconds a connection may live |
| `TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS` | `30` | no | the same | seconds a caller waits for a connection |
| `MISSION_VERSION_MAX_BODY_BYTES` | 268435456 (256 MiB) | no | Config | the body limit of `POST /api/v1/missions/{id}/versions` alone; zero or a negative number means the default |

### Sign-in and Discord

| Variable | Default | Required | Read by | Meaning |
|---|---|---|---|---|
| `JWT_SECRET` | none | yes | Config | signs the access tokens; changing it invalidates every issued token |
| `JWT_ACCESS_TTL_MIN` | `15` | no | Config | access-token lifetime in minutes; a value that does not parse means 15 |
| `DISCORD_CLIENT_ID` | empty | outside development | Config | the OAuth2 application id; empty sends sign-in back with `#error=oauth_unconfigured` |
| `DISCORD_CLIENT_SECRET` | empty | outside development | Config | the OAuth2 secret for the token exchange; a wrong one ends sign-in with `#error=discord_unreachable` |
| `DISCORD_REDIRECT_URL` | empty | outside development | Config | the callback registered byte-exact in the Discord Developer Portal; the template's is `http://localhost:8080/api/v1/auth/discord/callback` |
| `DISCORD_GUILD_ID` | empty | no | Config | the guild whose members' roles decide website [roles](/documentation_v2/glossary/n_to_z.md#role) through `apps/website/api_v2/seeds/discord_roles.sql`; empty skips membership reads, enrolment and reconciliation |
| `DISCORD_BOT_TOKEN` | empty: no bot | no | Config, read only through `Config::require_discord_bot_token` | the bot token; an unset token is reported by name where a path needs it |
| `DISCORD_WEBHOOK_URL` | empty: pushing off | no | Config; `apps/website/api_v2/src/community_content/services/discord_webhook.rs` | the channel webhook announcements are pushed to; while empty the push route answers 400 "discord webhook not configured" and a publish that asks to push writes a CRIT `webhook.push_failed` audit row |

### Game servers

| Variable | Default | Required | Read by | Meaning |
|---|---|---|---|---|
| `SERVICE_TOKEN` | empty: every service route refuses | no | Config; `ServiceAuth` in `apps/website/api_v2/src/core/middleware/authentication.rs` | the shared `X-Service-Token` of `POST /api/v1/ingest/link-confirm`, `POST /api/v1/ingest/match-results`, `GET /metrics` and the detailed `GET /healthz`; the game servers send the same value as `TBD_GAME_SERVER_TOKEN` in `tools_v2/xtask/deploy/deploy.env` |

The [machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential) of the
[fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) and the
[game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) are not environment variables:
administrators issue them per server with `POST /api/v1/servers/{id}/credentials`, and the API
stores only their digests.

### Worker intervals

Each is read when the workers are armed; a value that is not a positive whole number of seconds
means the default, and the boot log states the interval each worker got.

| Variable | Default | Read by |
|---|---|---|
| `SERVER_STATUS_PUBLISH_INTERVAL_SECS` | `10` | `apps/website/api_v2/src/background_workers/server_status_publisher.rs` |
| `LEADERBOARD_REFRESH_INTERVAL_SECS` | `900` | `apps/website/api_v2/src/background_workers/leaderboard_refresher.rs` |
| `ROLE_RESYNC_INTERVAL_SECS` | `86400` | `apps/website/api_v2/src/background_workers/discord_role_synchronizer.rs` |

### Process

| Variable | Default | Read by | Meaning |
|---|---|---|---|
| `SKIP_MIGRATE` | unset | `apps/website/api_v2/src/bin/api.rs` | any value, even empty, skips the migrations at boot, for a harness that migrates a shared database itself |
| `RUST_LOG` | `info` | `apps/website/api_v2/src/bin/api.rs` | the `tracing` filter; an unparseable value means `info` |

### Tests

| Variable | Read by | Meaning |
|---|---|---|
| `TEST_DATABASE_URL` | `apps/website/api_v2/tests/common/database.rs` | the base URL each database suite derives its own scratch database from; `cargo xtask db test-it` sets it, and a suite without it fails rather than skips |
| `TBD_GATE_DB` | `apps/website/api_v2/tests/aar_replay_url_backfill.rs` | that suite's fallback when `TEST_DATABASE_URL` is unset |
| `PROPTEST_RNG_SEED` | `apps/website/api_v2/src/tests/property_evidence.rs` | the property-test seed, decimal digits; the suites fix a default |
| `PROPTEST_CASES` | the same | must stay unset: the property suites own their case counts |

### Where each deployment sets them

- Development: `apps/website/api_v2/.env`, copied from the template.
- The deploy host: the systemd unit `tools_v2/xtask/deploy/systemd/tbd-website-api.service`
  loads the server's own `apps/website/api_v2/.env` (the deploy never copies one there) and sets
  `MAP_ASSETS_DIR`, `GLYPH_ASSETS_DIR` and `UPLOAD_DIR` itself, the last under the unit's state
  directory.
- Staging: the `api` service of `apps/website/docker-compose.staging.yml` sets the variables in
  its `environment` block, `TRUSTED_PROXIES` defaulting to `127.0.0.1/32` and the asset and upload
  directories to absolute paths inside the container.

## Design

The configuration fails closed where a wrong value would pass for an outage and degrades where a
missing integration is a choice. A required value left empty, or a set value that can never work
(a token with a newline, a proxy entry that trusts more than was typed, a pool setting that does
not parse), stops the boot and names the variable. An optional integration left empty (the bot,
the webhook, the guild) turns its path off and reports that by name where it is used. A variable
is added to `Config` together with the code that reads it, so no setting looks configured while
doing nothing; the pool settings stay outside `Config` because the binary hands the pool a URL.

`cargo xtask verify api-readiness` binds its evidence to the configuration: its fingerprint
covers the three `.env` files, `tools_v2/xtask/deploy/deploy.env` and a fixed list of these
variables, without printing any value
(`tools_v2/xtask/src/verifications/api_readiness/fingerprint.rs`).

## Open work

- [T-1007 — Fix missing TRUSTED_PROXIES entry in api_v2 .env.example](/.ai/tickets/T-1007.toml)
  (idea, no plan): the template documents `TRUSTED_PROXIES`.
- [T-1025 — Fix missing MISSION_VERSION_MAX_BODY_BYTES, SKIP_MIGRATE, RUST_LOG in api_v2 .env.example](/.ai/tickets/T-1025.toml)
  (idea, no plan): the template documents the other three omitted variables.
- [T-1027 — Rewrite stale comments in api_v2 core, workers, seeds and tests](/.ai/tickets/T-1027.toml)
  (idea, no plan): the template and `Config` stop tying `/map-assets` to `SPA_DIST_DIR`.
- [T-138 — One-command self-host setup via xtask](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-138_plan.md)): a new xtask setup command
  writes the `.env` along with the database and seed steps, so no one copies the template by
  hand.

## Decisions

- Required and malformed values stop the boot; optional integrations degrade by name: a
  misconfiguration must never read as a remote outage
  ([decisions log](/documentation_v2/website/api_v2/decisions.md)).
- An empty `TRUSTED_PROXIES` ignores `X-Forwarded-For` entirely: a rate-limit key any client can
  forge limits nobody, while a shared key still limits everyone.
- `UPLOAD_DIR` is absolute outside development: the process working directory is a deployment
  detail, and the deploy rsyncs the checkout with `--delete`, so nothing the API writes may live
  in it.
