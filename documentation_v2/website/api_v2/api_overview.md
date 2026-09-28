**Status:** live

# API overview

The cross-domain map of the website [API](/documentation_v2/glossary/a_to_f.md#api): how the `api`
binary boots, how a request travels through the shared middleware to one of eight domains, who
calls which part of `/api/v1`, and the layers every domain shares. Each domain's README states its
routes, models and rules exactly; this document is the map that leads to them.

## Where it lives

- Code: [`apps/website/api_v2/`](/apps/website/api_v2/), the package `website-api` (Axum and sqlx
  on Postgres). The crate [README](/apps/website/api_v2/README.md) is the atlas of the crate;
  [`apps/website/api_v2/src/core/http_router.rs`](/apps/website/api_v2/src/core/http_router.rs)
  assembles every route, mount and middleware layer.
- Entry: the `api` binary,
  [`apps/website/api_v2/src/bin/api.rs`](/apps/website/api_v2/src/bin/api.rs), which
  `cargo xtask mk rust-api` runs in development and the systemd unit
  `tools_v2/xtask/deploy/systemd/tbd-website-api.service` runs on the deploy host.
- Related: the [environment variable reference](/documentation_v2/website/api_v2/environment_variables.md),
  the [API decisions log](/documentation_v2/website/api_v2/decisions.md), the
  [verification evidence](/documentation_v2/website/api_v2/verification_evidence/README.md) the
  API is accepted against, and the [frontend documentation](/documentation_v2/website/frontend/README.md)
  of the pages that call it.

## Behaviour

### Boot

1. `Config::load` reads the environment and the first `.env` found upward from the working
   directory; a required variable that is empty, or a set value that cannot work, stops the boot
   (the [environment variable reference](/documentation_v2/website/api_v2/environment_variables.md)
   lists each rule).
2. `core::database::connect` opens the Postgres pool with the `TBD_DB_POOL_*` settings.
3. `core::database::migrate` applies the embedded migrations of `apps/website/api_v2/migrations/`
   and logs `migrations applied`, unless `SKIP_MIGRATE` is set.
4. `AppState::new` builds the one shared state: the pool, the configuration, the token manager,
   the realtime hub, the rate limiters and the Discord and webhook clients.
5. `background_workers::spawn_all` arms the twelve
   [background workers](/documentation_v2/glossary/a_to_f.md#background-workers) and logs the interval
   each got.
6. `http_router::router` builds the application, and the binary serves it on `0.0.0.0:$PORT`
   until SIGINT or SIGTERM, draining requests in flight; shutdown also closes every open event
   stream, so its clients reconnect and the audit log feed replays after their `Last-Event-ID`.

### Request path

Every request passes one middleware chain, outermost first:

```text
request ─▶ request id ─▶ access log ─▶ metrics ─▶ panic recovery ─▶ CORS ─▶ body limit
        ─▶ rate limit ─▶ /api/v1/*, /healthz, /metrics, /uploads, the built app
                     └─▶ /map-assets, /map-assets/glyphs (mounted below the rate limit)
```

The body limit is 1 MiB, raised per route for the CMS upload (6 MiB) and the
[mission](/documentation_v2/glossary/g_to_m.md#mission) version save (`MISSION_VERSION_MAX_BODY_BYTES`,
256 MiB by default). The rate limit keeps an in-memory bucket per client address and, on the
unauthenticated `/api/v1/auth/` family, a stricter one with a second bucket in Postgres that
survives a restart; a refusal is `429` with `Retry-After`. `/api/v1/game-runtime/` and
`/api/v1/ingest/` stay on the global bucket: every caller there is a game server with its own
machine credential, several servers can share one host address, and event batches arrive in
bursts. The [middleware README](/apps/website/api_v2/src/core/middleware/README.md)
gives the numbers and the client-address rule behind `TRUSTED_PROXIES`.

Authentication is not a layer. A route's access tier is the extractor its handler takes:

| Tier | Extractor | Credential | Refusal |
|---|---|---|---|
| public | none | none | — |
| member | `AuthUser` | `Authorization: Bearer <access token>` of a live session | 401 |
| role | `LeaderUser`, `MissionMakerUser`, `AdminUser` | the same, and a [role](/documentation_v2/glossary/n_to_z.md#role) rank at least `leader`, `mission_maker` or `admin` | 403 |
| machine | `MachineCaller` | `Authorization: Bearer tbdm_…`, a per-server [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential) of the executor kind the route needs | 401, or 403 for another server's resource |
| observability | `ObservabilityAuth` | `Authorization: Bearer <OBSERVABILITY_TOKEN>`, compared in constant time; every request fails while it is unset | 401 |

Role ranks run `guest`, `enlisted`, `leader`, `mission_maker`, `admin`, lowest first. A member's
role follows their Discord roles through the `discord_roles` mapping; the API never sets it by
hand.

Every refusal answers the `{error, details?}` envelope. A path segment that does not decode into
its type (a non-UUID id, say) is a 400 whose message names the parameter: every handler reads its
path through `PathParams`
([request-shape primitives](/apps/website/api_v2/src/core/http/README.md)).

### Callers

- The single-page app in `apps/website/frontend/` calls the member, role and public routes. In
  development its Trunk server on port 3000 proxies `/api` and `/map-assets` to the API on
  `127.0.0.1:8080`; on the deploy host, Caddy serves the built app and proxies API traffic to the
  same port (`tools_v2/xtask/deploy/Caddyfile.website`).
- The [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime), the mod's
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`, calls `/api/v1/game-runtime/*` with its
  `mod_runtime` credential: runtime sessions and heartbeats, the event roster, player
  [deployments](/documentation_v2/glossary/a_to_f.md#deployment), and the
  [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) it should run with the
  [artifact](/documentation_v2/glossary/a_to_f.md#artifact) bytes. The same credential authenticates
  `/api/v1/ingest/*`: the Arma link confirmation (`/ingest/link-confirm`) and the
  [match telemetry](/documentation_v2/glossary/g_to_m.md#match-telemetry) the mod's durable queue
  delivers (`/ingest/matches`, `/ingest/match-results`, `/ingest/match-events`).
- The [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) in
  `apps/fleet_host_agent/` polls `/api/v1/fleet-executor/*` outbound with its `host_agent`
  credential and runs the [fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command) it claims.
  No route reaches into a game host; the API only records commands for executors to claim.
- A monitoring scraper reads `/metrics` with the operator's `OBSERVABILITY_TOKEN` bearer;
  `/healthz` answers anyone with the status alone and adds the detail for the same bearer. No
  other route accepts that token, and no user session or machine credential reaches these two.

### Realtime streams

Two routes answer [SSE](/documentation_v2/glossary/n_to_z.md#sse) streams, and each re-checks the
viewer's session at least every five seconds, ending the stream with an `authorization_expired`
SSE event once it stops qualifying:

- `GET /api/v1/servers/{id}/status/stream` (member; before the stream opens, a malformed id is a
  400, an unknown server a 404, and an inactive server a 404 for anyone but an administrator): one
  server's live status, its telemetry queue reading included, published on
  the in-process hub's `server:{id}` topic by the heartbeat, the runtime-session expiry and the
  status publisher worker, which republishes the active servers only.
- `GET /api/v1/admin/audit-logs/stream` (administrator): committed audit rows in publication
  order, woken by Postgres `NOTIFY` and retried on a two-second timer. It opens with
  `event: ready` (`{resume_after, retained_after}`), then sends each row as an unnamed event whose
  `id` is its publication sequence and whose data is the list route's row. A reconnect replays
  after its `Last-Event-ID`; a cursor ahead of the tail or below the retention floor gets
  `event: reset` (`cursor_ahead` or `history_unavailable`) and the stream continues from the
  tail, and a malformed `Last-Event-ID` answers 400. The audit logs page reads it live and
  reloads its history on `reset`.

### Background workers

The binary arms twelve interval tasks, each of which calls a service of the domain that owns the
data: audit publication, Discord membership reconciliation, Discord role resync, the event
lifecycle sweep, event reservation re-evaluation, fleet command reconciliation, the leaderboard
refresh, mission deployment reconciliation, rate-limit bucket cleanup, runtime session expiry,
the server status republish and the refresh-token purge. The
[background workers README](/apps/website/api_v2/src/background_workers/README.md) gives each
one's interval and the service it calls; three intervals are set by environment variables.

## Data

### Routes by domain

Every public path is the literal written in the domain's `routes.rs` with `/api/v1` in front: the
router merges the eight tables without a prefix of its own. The domain README's Public surface
lists every route with its methods and tier.

| Domain | Paths under `/api/v1` | Tiers |
|---|---|---|
| [identity and access](/apps/website/api_v2/src/identity_and_access/README.md#public-surface) | `/auth/discord/login`, `/auth/discord/callback`, `/auth/refresh`, `/auth/logout`, `/auth/dev-login` (development only), `/me`, `/me/link`, `/me/link/status`, `/ingest/link-confirm` | public, member, machine |
| [administration](/apps/website/api_v2/src/administration/README.md#public-surface) | `/admin/users` (searched by `q`, paged by `page` and `per_page`), `/admin/users/{discordId}` with `/ban`, `/warnings` and `/membership-grace`, `/admin/roles/sync`, `/admin/audit-logs` with `/export.csv` and `/stream` (ready, rows, reset) | administrator; the grace route is member with administrator authority checked in its service |
| [operations](/apps/website/api_v2/src/operations/README.md#public-surface) | `/events` and `/events/{id}/…` (missions, access, access policy, reservation quotas, groups, fire missions), `/event-missions/{emid}/…` (ORBAT, register, slot assignment, squad reserve and release, waitlist promotion, squad and slot access policies), `/members`, `/me/deployments`, `/me/leave-requests`, `/admin/leave-requests`, `/fire-missions`, `/fire-missions/solve`, `/game-runtime/events/{id}/roster`, `/game-runtime/sessions/{sessionId}/deployments/…` | member, leader, administrator, machine |
| [missions](/apps/website/api_v2/src/missions/README.md#public-surface) | `/missions` and `/missions/{id}/…` (submit, reviews, review comments, artifacts, versions, armory, bookmark, export), `/registry`, `/registry/compat`, `/factions`, `/approvals`, `/admin/mission-default-overrides`, `/servers/{id}/deployments`, `/game-runtime/deployment`, `/game-runtime/deployments`, `/game-runtime/artifacts/{artifactId}`, `/game-runtime/missions` | member, mission maker, author or administrator, administrator, machine |
| [match telemetry](/apps/website/api_v2/src/match_telemetry/README.md#public-surface) | `/game-runtime/sessions/{sessionId}/heartbeats`, `/ingest/matches`, `/ingest/match-results`, `/ingest/match-events`, `/matches/{matchId}/events` | machine, member |
| [command center](/apps/website/api_v2/src/command_center/README.md#public-surface) | `/dashboard`, `/leaderboards`, `/users/{discordId}/stats` | member |
| [community content](/apps/website/api_v2/src/community_content/README.md#public-surface) | `/announcements`, `/wiki`, `/wiki/{slug}` with `/revisions` and `/revisions/{revision}`, `/vehicle-database`, `/vehicle-database/{id}`, `/modpacks` (with `/current` and `/set-current`), `/cms/announcements` (with `/push-discord`), `/cms/uploads` | member to read, administrator to write |
| [server infrastructure](/apps/website/api_v2/src/server_infrastructure/README.md#public-surface) | `/servers` and `/servers/{id}/…` (status, status stream, credentials, commands), `/fleet-executor/commands/…`, `/game-runtime/sessions`, `/game-runtime/sessions/{sessionId}/end`, `/fleet/scenarios` | member, administrator, machine |

A path prefix does not name its owner: `/servers/{id}/deployments` belongs to missions,
`/game-runtime/events/{id}/roster` to operations and the heartbeats route to match telemetry,
because each domain owns the data its routes write.

The configured fleet is the set of servers with `is_active = true`. `GET /servers` lists it to
members and every server, with `is_active`, to administrators; the status read and the status
stream of an inactive server are a 404 for anyone but an administrator; and `GET /dashboard`
answers `fleet {servers, totals}` over the active servers in place of a single server status. The
match-telemetry ingests answer a refusal the game runtime must act on as a 400 or 409 carrying
`details.code`, never a 404; the
[telemetry design](/documentation_v2/website/api_v2/verification_evidence/telemetry.md) lists the
codes, the results-revision rules and the event batch rules.

The administration and content routes answer these shapes; the
[administration and content design](/documentation_v2/website/api_v2/verification_evidence/administration_and_content.md)
holds the full rules:

- `GET /admin/users` answers one page of the roster, `{items, page, per_page, total}`, ordered by
  username. `page` defaults to 1 and `per_page` to 20, a `per_page` above 100 clamps to 100, a value
  below 1 or not a number is a 400, and a page past the end is empty with the real `total`.
- `/vehicle-database/{id}` reads one row to members; an administrator replaces it (`PUT`),
  changes some of its fields (`PATCH`: an absent field stays, `null` clears an optional one) or
  soft-deletes it (`DELETE`). Each write answers the stored row, all three write bodies refuse
  unknown fields, and a deleted row is a 404 and leaves the list.
- `PUT /wiki/{slug}` creates a page (`base_revision: null`, 201) or saves its next revision (the
  current `base_revision`, 200). A stale revision is a 409 `wiki_revision_conflict`, a body over
  262,144 bytes a 400 `wiki_body_too_large`, and an unsafe link or image URL, raw HTML or nesting
  deeper than 16 a 422 `wiki_markup_refused` with its findings. Articles and revisions carry the
  parsed `blocks`; `/revisions` pages the history newest first.
- Every vehicle write and wiki save appends its audit row in the same transaction.
- `POST /cms/uploads` stores one JPEG, PNG or WebP image of at most 5 MiB and answers 201
  `{url}`. A body or file over the limit is a 413 `request_too_large`, another extension or bytes
  that do not match it a 415, and a storage failure a 503 `storage_unavailable`; the file is
  renamed into place whole, so `/uploads` never serves part of one.

### Outside `/api/v1`

- `GET /healthz`: `{"status": …}` with 200 or 503 for anyone; the version, uptime, pool and
  migration detail for a caller with the `OBSERVABILITY_TOKEN` bearer
  (`apps/website/api_v2/src/core/observability/health_probe.rs`); a missing or wrong bearer gets
  the public answer, not an error.
- `GET /metrics`: the Prometheus exposition, `OBSERVABILITY_TOKEN` bearer only, 401 while the token
  is unset (`apps/website/api_v2/src/core/observability/observability_auth.rs`).
- `/uploads`: the files the CMS upload wrote into `UPLOAD_DIR`.
- `/map-assets` and `/map-assets/glyphs`: the terrain tree (`MAP_ASSETS_DIR`) and the glyph atlas
  (`GLYPH_ASSETS_DIR`) that the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
  streams, below the rate limit.
- The built app from `SPA_DIST_DIR`, with an `index.html` fallback and the cross-origin isolation
  headers, when that variable is set.

### Storage

- Postgres 18: locally the `db` service of `apps/website/api_v2/docker-compose.yml` on host port
  5434, which `cargo xtask db up` starts. The API owns the schema through the numbered migrations
  in `apps/website/api_v2/migrations/`, embedded at compile time; an applied migration never changes
  in its statements, and a comments-only edit needs `cargo xtask db repair-migration-checksum` on
  every database that applied it. The [migrations README](/apps/website/api_v2/migrations/README.md)
  has the rules.
- `apps/website/api_v2/seeds/`: the development data `cargo xtask db seed` applies once the API
  has migrated the database, and the Workbench [registry](/documentation_v2/glossary/n_to_z.md#registry)
  exports that `cargo xtask db registry-import` loads.
- The upload directory, the only files the API writes.

## Design

The crate is organised by domain, not by layer. `core` holds what every domain needs and names no
domain concept; `background_workers` holds the timers and calls domain services; each of the eight
domains owns its route table, handlers, services and models, and another domain reaches it only
through its services and models, never its handlers. The rules are executable:
`apps/website/api_v2/src/tests/architecture_rules.rs` checks the layout and
`apps/website/api_v2/src/tests/prose_rules.rs` the prose of comments, seeds and `.env.example`,
on every unit-test run; `cargo xtask verify route-tags` checks that every handler declares its
`/// @route`, and `cargo xtask schema citations` checks that the `@contract` tags of the models that project a
schema cite it correctly.

Contract parity follows Law 9 of `CLAUDE.md`: the domain models are the snake_case source of
truth, the contract types are generated from `contracts_v2/definitions/` by
`cargo xtask ci schema-codegen`, and the frontend DTOs in
`apps/website/frontend/src/v2/core/api/dto/` mirror the models under golden tests. The missions
domain's [contract layer](/apps/website/api_v2/src/missions/contract/README.md) validates every
mission document it accepts or serves against those schemas.

To verify a change from the repository root:

```bash
cargo xtask db up
cargo xtask mk rust-api            # stays in the foreground; wait for `migrations applied`
curl -sf http://localhost:8080/healthz
cargo xtask db test-it
cargo xtask verify route-tags
```

Then call the changed endpoint and compare its JSON with the domain's `models/` and the matching
DTO in `apps/website/frontend/src/v2/core/api/dto/`. Acceptance of the API as a whole is
`cargo xtask verify api-readiness`, which judges the receipts described in the
[verification evidence](/documentation_v2/website/api_v2/verification_evidence/README.md).

### Known discrepancies

- The server registry writes have no page: the API serves `POST /api/v1/servers` and `PATCH` and
  `DELETE /api/v1/servers/{id}` (`apps/website/api_v2/src/server_infrastructure/routes.rs`) and
  accepts `server_id` on event create and update, while the server control page only lists
  `/api/v1/servers`.
- The mortar page solves through the API: `POST /api/v1/fire-missions/solve` runs the map engine's
  ballistics on the server, so the page needs the API to answer.

## Open work

- [T-940 — Website platform: events, telemetry, admin, content](/documentation_v2/tickets/specs/t940_website_platform.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-940_plan.md)): the program whose open children
  follow.
- [T-940.10 — Mortar ballistics crate for API and offline frontend](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_10_plan.md)): one ballistics model serves
  both the API and the mortar page, so the page solves without the API.
- [T-940.13 — Combat, medical and vehicle telemetry events](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_13_plan.md)): the events schema
  (`contracts_v2/definitions/match-telemetry.schema.json`), `POST /api/v1/ingest/match-events` and
  `GET /api/v1/matches/{matchId}/events` exist; the after-action replay that plays them does
  not.
- [T-1022 — Add website admin UI to manage game servers](/.ai/tickets/T-1022.toml) (idea, no
  plan): a page calls the server registry writes and sends `server_id` with events.
- [T-952 — website-api set_var mutates a shared test process](/.ai/tickets/T-952.toml) (idea, no
  plan): the pool-setting tests in `apps/website/api_v2/src/core/database/tests/connection.rs`
  stop mutating the process environment and use `DbPoolConfig::from_lookup`.
- [T-1131 — Decide whether Markdown edits should invalidate API readiness evidence](/.ai/tickets/T-1131.toml)
  (idea, no plan): whether documentation stays in the readiness fingerprint.

## Decisions

The [API decisions log](/documentation_v2/website/api_v2/decisions.md) records each with its
context and consequences.

- The crate is organised by domain, and a route's tier travels with its handler's extractor:
  a domain can be read, changed and tested whole, and no router position grants access.
- `/map-assets` sits below the rate limiter: terrain streaming is bytes, not requests, and a cold
  Mission Creator boot fetches hundreds of files at once.
- Configuration fails closed: a required variable left empty, or a set value that cannot work,
  stops the boot instead of surfacing later as an outage.
- Game hosts are reached only through work they claim: the fleet host agent and the game runtime
  poll outbound with per-server machine credentials, and game servers fetch mission artifacts
  over HTTPS rather than from files staged on disk.
- Background workers own the schedule and nothing else: the work is a domain service, and only
  the binary imports the workers.
