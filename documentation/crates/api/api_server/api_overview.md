**Status:** live

# API overview

The cross-domain map of the website [API](/documentation/glossary/a_to_f.md#api): how the
`api-server` binary boots, how a request travels through the shared middleware to one of eight domains, who
calls which part of `/api/v1`, and the layers every domain shares. Each domain's README states its
routes, models and rules exactly; this document is the map that leads to them.

## Where it lives

- Code: [`crates/api/api_server/`](/crates/api/api_server/), the package `api_server` (Axum and
  sqlx on Postgres), the top crate of the [API crates](/crates/api/README.md) in `crates/api/`,
  which assembles the other 23 (the [crate map](#crate-map) below). The crate
  [README](/crates/api/api_server/README.md) is the atlas of the server;
  [`crates/api/api_server/src/router.rs`](/crates/api/api_server/src/router.rs)
  assembles every route, mount and middleware layer.
- Entry: the `api-server` binary,
  [`crates/api/api_server/src/bin/api_server.rs`](/crates/api/api_server/src/bin/api_server.rs), which
  `cargo xtask mk rust-api` runs in development and the systemd unit
  `deploy/systemd/tbd-website-api.service` runs on the deploy host.
- Related: the [environment variable reference](/documentation/crates/api/api_server/environment_variables.md),
  the [API decisions log](/documentation/crates/api/api_server/decisions.md), the
  [verification evidence](/documentation/crates/api/api_server/verification_evidence/README.md) the
  API is accepted against, and the [frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md)
  of the pages that call it.

## Behaviour

### Boot

1. `Config::load` reads the environment and the first `.env` found upward from the working
   directory; a required variable that is empty, or a set value that cannot work, stops the boot
   (the [environment variable reference](/documentation/crates/api/api_server/environment_variables.md)
   lists each rule).
2. `api_database::connect` opens the Postgres pool with the `TBD_DB_POOL_*` settings.
3. `api_database::migrate` applies the embedded migrations of `crates/api/api_database/migrations/`
   and logs `migrations applied`, unless `SKIP_MIGRATE` is set.
4. `composition::application_state` builds the one shared state: it constructs the session
   authority, the Discord and webhook clients and the equipment datasets, and `AppState::new`
   adds the pool, the configuration, the token manager, the realtime hub and the rate limiters.
5. `api_background_workers::spawn_all` arms the thirteen
   [background workers](/documentation/glossary/a_to_f.md#background-workers) and logs the interval
   each got.
6. `router::router` builds the application, and the binary serves it on `0.0.0.0:$PORT`
   until SIGINT or SIGTERM, draining requests in flight; shutdown also closes every open event
   stream, so its clients reconnect and the audit log feed replays after their `Last-Event-ID`.

### Request path

Every request passes one middleware chain, outermost first:

```text
request ─▶ request id ─▶ access log ─▶ metrics ─▶ panic recovery ─▶ CORS ─▶ body limit
        ─▶ rate limit ─▶ /api/v1/*, /healthz, /metrics, /uploads, the built app
                     └─▶ /map-assets, /map-assets/glyphs (mounted below the rate limit)
```

The body limit is 1 MiB, raised per route for the CMS upload (6 MiB), the ballistics catalog
upload (17 MiB + 64 KiB, a 16 MiB calibration part and a 1 MiB catalog part) and the
[mission](/documentation/glossary/g_to_m.md#mission) version save (`MISSION_VERSION_MAX_BODY_BYTES`,
256 MiB by default). The rate limit keeps an in-memory bucket per client address and, on the
unauthenticated `/api/v1/auth/` family, a stricter one with a second bucket in Postgres that
survives a restart; a refusal is `429` with `Retry-After`. `/api/v1/game-runtime/` and
`/api/v1/ingest/` stay on the global bucket: every caller there is a game server with its own
machine credential, several servers can share one host address, and event batches arrive in
bursts. The [middleware README](/crates/api/api_http_layer/src/middleware/README.md)
gives the numbers and the client-address rule behind `TRUSTED_PROXIES`.

Authentication is not a layer. A route's access tier is the extractor its handler takes:

| Tier | Extractor | Credential | Refusal |
|---|---|---|---|
| public | none | none | — |
| member | `AuthUser` | `Authorization: Bearer <access token>` of a live session | 401 |
| role | `LeaderUser`, `MissionMakerUser`, `AdminUser` | the same, and a [role](/documentation/glossary/n_to_z.md#role) rank at least `leader`, `mission_maker` or `admin` | 403 |
| machine | `MachineCaller` | `Authorization: Bearer tbdm_…`, a per-server [machine credential](/documentation/glossary/g_to_m.md#machine-credential) of the executor kind the route needs | 401, or 403 for another server's resource |
| observability | `ObservabilityAuth` | `Authorization: Bearer <OBSERVABILITY_TOKEN>`, compared in constant time; every request fails while it is unset | 401 |

Role ranks run `guest`, `enlisted`, `leader`, `mission_maker`, `admin`, lowest first. A member's
role follows their Discord roles through the `discord_roles` mapping; the API never sets it by
hand.

Every refusal answers the `{error, details?}` envelope. A path segment that does not decode into
its type (a non-UUID id, say) is a 400 whose message names the parameter: every handler reads its
path through `PathParams`
([request-shape primitives](/crates/api/api_foundation/src/http/README.md)).

### Callers

- The single-page app in `crates/frontend/shell/frontend_application/` calls the member, role and public routes. In
  development its Trunk server on port 3000 proxies `/api` and `/map-assets` to the API on
  `127.0.0.1:8080`; on the deploy host, Caddy serves the built app and proxies API traffic to the
  same port (`deploy/caddy/Caddyfile`).
- The [game runtime](/documentation/glossary/g_to_m.md#game-runtime), the mod's
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`, calls `/api/v1/game-runtime/*` with its
  `mod_runtime` credential: runtime sessions and heartbeats, the event roster, player
  [deployments](/documentation/glossary/a_to_f.md#deployment), and the
  [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) it should run with the
  [artifact](/documentation/glossary/a_to_f.md#artifact) bytes. The same credential authenticates
  `/api/v1/ingest/*`: the Arma link confirmation (`/ingest/link-confirm`) and the
  [match telemetry](/documentation/glossary/g_to_m.md#match-telemetry) the mod's durable queue
  delivers (`/ingest/matches`, `/ingest/match-results`, `/ingest/match-events`).
- The [game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) in
  `crates/fleet/game_server_host_agent/` polls `/api/v1/fleet-executor/*` outbound with its `host_agent`
  credential and runs the [fleet commands](/documentation/glossary/a_to_f.md#fleet-command) it claims.
  No route reaches into a game host; the API only records commands for executors to claim.
- A monitoring scraper reads `/metrics` with the operator's `OBSERVABILITY_TOKEN` bearer;
  `/healthz` answers anyone with the status alone and adds the detail for the same bearer. No
  other route accepts that token, and no user session or machine credential reaches these two.

### Realtime streams

Two routes answer [SSE](/documentation/glossary/n_to_z.md#sse) streams, and each re-checks the
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

The binary arms thirteen interval tasks, each of which calls a service of the domain that owns the
data: audit publication, Discord membership reconciliation, Discord role resync, the equipment
export import, the event lifecycle sweep, event reservation re-evaluation, fleet command reconciliation, the leaderboard
refresh, mission deployment reconciliation, rate-limit bucket cleanup, runtime session expiry,
the server status republish and the refresh-token purge. The
[background workers README](/crates/api/api_background_workers/src/README.md) gives each
one's interval and the service it calls; three intervals are set by environment variables.

## Data

### Routes by domain

Every public path is the literal written in the domain's `routes.rs` with `/api/v1` in front: the
router merges the eight tables without a prefix of its own. The domain README's Public surface
lists every route with its methods and tier.

| Domain | Paths under `/api/v1` | Tiers |
|---|---|---|
| [identity and access](/crates/api/api_identity_and_access/src/README.md#public-surface) | `/auth/discord/login`, `/auth/discord/callback`, `/auth/refresh`, `/auth/logout`, `/auth/dev-login` (development only), `/me`, `/me/link`, `/me/link/status`, `/ingest/link-confirm` | public, member, machine |
| [administration](/crates/api/api_administration/src/README.md#public-surface) | `/admin/users` (searched by `q`, paged by `page` and `per_page`), `/admin/users/{discordId}` with `/ban`, `/warnings` and `/membership-grace`, `/admin/roles/sync`, `/admin/audit-logs` with `/export.csv` and `/stream` (ready, rows, reset) | administrator; the grace route is member with administrator authority checked in its service |
| [operations](/crates/api/api_operations/src/README.md#public-surface) | `/events` and `/events/{id}/…` (missions, access, access policy, reservation quotas, groups, fire missions), `/event-missions/{emid}/…` (ORBAT, register, slot assignment, squad reserve and release, waitlist promotion, squad and slot access policies), `/members`, `/me/deployments`, `/me/leave-requests`, `/admin/leave-requests`, `/fire-missions`, `/ballistics-catalogs` and `/ballistics-catalogs/{catalogId}/versions/{version}`, `/game-runtime/events/{id}/roster`, `/game-runtime/sessions/{sessionId}/deployments/…` | public (the catalog reads), member, leader, administrator, machine |
| [missions](/crates/api/api_missions/src/README.md#public-surface) | `/missions` and `/missions/{id}/…` (submit, reviews, review comments, artifacts, versions, armory, bookmark, export), `/registry`, `/registry/compat`, `/factions`, `/approvals`, `/admin/mission-default-overrides`, `/servers/{id}/deployments`, `/game-runtime/deployment`, `/game-runtime/deployments`, `/game-runtime/artifacts/{artifactId}`, `/game-runtime/missions` | member, mission maker, author or administrator, administrator, machine |
| [match telemetry](/crates/api/api_match_telemetry/src/README.md#public-surface) | `/game-runtime/sessions/{sessionId}/heartbeats`, `/ingest/matches`, `/ingest/match-results`, `/ingest/match-events`, `/matches/{matchId}/events` | machine, member |
| [command center](/crates/api/api_command_center/src/README.md#public-surface) | `/dashboard`, `/leaderboards`, `/users/{discordId}/stats` | member |
| [community content](/crates/api/api_community_content/src/README.md#public-surface) | `/announcements`, `/wiki`, `/wiki/{slug}` with `/revisions` and `/revisions/{revision}`, `/vehicle-database`, `/vehicle-database/{id}`, `/modpacks` (with `/current` and `/set-current`), `/cms/announcements` (with `/push-discord`), `/cms/uploads` | member to read, administrator to write |
| [server infrastructure](/crates/api/api_server_infrastructure/src/README.md#public-surface) | `/servers` and `/servers/{id}/…` (status, status stream, credentials, commands), `/fleet-executor/commands/…`, `/game-runtime/sessions`, `/game-runtime/sessions/{sessionId}/end`, `/fleet/scenarios` | member, administrator, machine |

A path prefix does not name its owner: `/servers/{id}/deployments` belongs to missions,
`/game-runtime/events/{id}/roster` to operations and the heartbeats route to match telemetry,
because each domain owns the data its routes write.

The configured fleet is the set of servers with `is_active = true`. `GET /servers` lists it to
members and every server, with `is_active`, to administrators; the status read and the status
stream of an inactive server are a 404 for anyone but an administrator; and `GET /dashboard`
answers `fleet {servers, totals}` over the active servers in place of a single server status. The
match-telemetry ingests answer a refusal the game runtime must act on as a 400 or 409 carrying
`details.code`, never a 404; the
[telemetry design](/documentation/crates/api/api_server/verification_evidence/telemetry.md) lists the
codes, the results-revision rules and the event batch rules.

The administration and content routes answer these shapes; the
[administration and content design](/documentation/crates/api/api_server/verification_evidence/administration_and_content.md)
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
  (`crates/api/api_http_layer/src/observability/health_probe.rs`); a missing or wrong bearer gets
  the public answer, not an error.
- `GET /metrics`: the Prometheus exposition, `OBSERVABILITY_TOKEN` bearer only, 401 while the token
  is unset (`crates/api/api_http_layer/src/observability/observability_auth.rs`).
- `/uploads`: the files the CMS upload wrote into `UPLOAD_DIR`.
- `/map-assets` and `/map-assets/glyphs`: the terrain tree (`MAP_ASSETS_DIR`) and the glyph atlas
  (`GLYPH_ASSETS_DIR`) that the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
  streams, below the rate limit.
- The built app from `SPA_DIST_DIR`, with an `index.html` fallback and the cross-origin isolation
  headers, when that variable is set.

### Storage

- Postgres 18: locally the `db` service of `deploy/compose.dev.yml` on host port
  5434, which `cargo xtask db up` starts. The API owns the schema through the numbered migrations
  in `crates/api/api_database/migrations/`, embedded at compile time; an applied migration never changes
  in its statements, and a comments-only edit needs `cargo xtask db repair-migration-checksum` on
  every database that applied it. The [migrations README](/crates/api/api_database/migrations/README.md)
  has the rules.
- `crates/api/api_database/seeds/`: the development data `cargo xtask db seed` applies once the API
  has migrated the database, and the Workbench [registry](/documentation/glossary/n_to_z.md#registry)
  exports that `cargo xtask db registry-import` loads.
- The upload directory (`UPLOAD_DIR`) and the equipment data directory (`EQUIPMENT_DATA_DIR`),
  the only files the API writes.

## Design

The API is organised by domain, not by layer, as crates under `crates/api/`. The kernel crates
(among them `api_http_layer`, the middleware and extractors, and `api_state`, the application
state) hold what every domain needs and name no domain concept; `api_background_workers` holds the
timers and calls domain services; each of the eight domain crates owns its route table, handlers,
services and models, and another domain reaches it only through its services and models, never its
handlers. The API server (`crates/api/api_server`) only assembles them: its router merges the route tables and
its composition root builds the state. The rules are executable:
`crates/api/api_server/src/tests/architecture_rules.rs` checks the layout and
`crates/api/api_server/src/tests/prose_rules.rs` the prose of comments, seeds and `.env.example`,
on every unit-test run; `cargo xtask verify route-tags` checks that every handler declares its
`/// @route`, and `cargo xtask schema citations` checks that the `@contract` tags of the models that project a
schema cite it correctly.

### Crate map

Every API crate sits in `crates/api/` and declares its tier (1 plus the highest tier it depends
on); each links its README. The API server `api_server` depends on the crates its router,
composition root and binaries name, and reaches `api_foundation`, `api_audit_log`, `api_mission_vocabulary`,
`api_member_activity`, `api_failpoints` and `api_property_evidence` only as dev-dependencies of
its tests.

| Group | Crate | Tier | Owns |
|---|---|---|---|
| infrastructure | [`api_identifiers`](/crates/api/api_identifiers/README.md) | 1 | the typed ids ([Ids](#ids)) |
| infrastructure | [`api_foundation`](/crates/api/api_foundation/README.md) | 1 | the `{error, details?}` envelope (`ApiError`), JSON wire formats, text policies, `PathParams` |
| infrastructure | [`api_failpoints`](/crates/api/api_failpoints/README.md) | 2 | `fail_point!` and the failpoint catalogue ([Failpoints](#failpoints)) |
| infrastructure | [`api_configuration`](/crates/api/api_configuration/README.md) | 2 | `Config`, trusted proxies, the process shutdown signal |
| infrastructure | [`api_database`](/crates/api/api_database/README.md) | 3 | the pool, `migrations/`, `seeds/`, SQLSTATE predicates |
| infrastructure | [`api_http_layer`](/crates/api/api_http_layer/README.md) | 3 | access tokens, the middleware chain and the tier extractors, rate limiters, `/healthz` and `/metrics`, the realtime hub |
| infrastructure | [`api_property_evidence`](/crates/api/api_property_evidence/README.md) | 1 | dev-only: the property run recorder |
| kernel | [`api_mission_vocabulary`](/crates/api/api_mission_vocabulary/README.md) | 0 | `TerrainType`, `GameMode` |
| kernel | [`api_audit_log`](/crates/api/api_audit_log/README.md) | 2 | the audit severity and the audit line appends every domain writes |
| kernel | [`api_equipment_datasets`](/crates/api/api_equipment_datasets/README.md) | 2 | the equipment dataset imports, index and reads behind community content's viewer routes |
| kernel | [`api_member_activity`](/crates/api/api_member_activity/README.md) | 3 | member statistics, the leaderboard view, attendance attribution, the re-evaluation queue |
| kernel | [`api_discord`](/crates/api/api_discord/README.md) | 4 | the Discord OAuth2, guild-member and webhook clients (`WebhookAnnouncement`) |
| kernel | [`api_caller_identity`](/crates/api/api_caller_identity/README.md) | 4 | `UserRole`, session and account authority, cached membership permissions, `MachineCaller` |
| kernel | [`api_state`](/crates/api/api_state/README.md) | 5 | `AppState` and its `FromRef` projections ([State composition](#state-composition)) |
| domain | `api_community_content`, `api_identity_and_access` | 6 | their route groups in [Routes by domain](#routes-by-domain) |
| domain | `api_administration`, `api_server_infrastructure` | 7 | 〃 |
| domain | `api_missions`, `api_match_telemetry` | 8 | 〃 |
| domain | `api_operations` | 9 | 〃 |
| domain | `api_command_center` | 10 | 〃 |
| workers | [`api_background_workers`](/crates/api/api_background_workers/README.md) | 10 | the thirteen interval tasks and `spawn_all` |
| server | [`api_server`](/crates/api/api_server/README.md) | 11 | the router, the composition root, the `api-server` and `import-item-registry` binaries and the integration suites |

A domain crate exposes one `routes()` table, which the router merges; it depends on another
domain only along the domain graph `crates/api/api_server/src/tests/architecture_rules.rs` holds
(administration on identity and access; server infrastructure on community content; match
telemetry on server infrastructure; missions on community content and server infrastructure;
operations on identity and access, match telemetry, missions and server infrastructure; command
center on community content, identity and access, missions, operations and server
infrastructure), and no kernel or infrastructure crate depends on a domain. The
`staging-fixtures` host tool in `tools/staging/staging_fixtures/` writes through the domain
crates' services and carries 4 of the API's 154 integration binaries.

### State composition

`api::composition::application_state(pool, config)` is the one place the concrete services are
built: the database session authority of `api_caller_identity`, the Discord OAuth2 client and the
announcement webhook of `api_discord`, and the `EquipmentDatasets` of `api_equipment_datasets`.
It passes them to `api_state::AppState::new`, which builds the token manager, the CORS allow-list,
the rate-limit state, the realtime hub and the metrics registry itself. `AppState` holds the
services concretely and the session authority as an `Arc<dyn SessionAuthority>`, so
`api_http_layer`'s `AuthUser` extractor asks it without naming the caller identity crate. A
handler extracts the whole state; a middleware or extractor of `api_http_layer` takes only its
part through a `FromRef` projection (the pool, the configuration, the token manager, the session
authority, the CORS origins, the rate limits, the hub, the Discord services), which
keeps `api_http_layer` below `api_state`. The `api-server` binary, `import-item-registry`
and every integration suite build their state through `application_state`.

### Failpoints

`fail_point!(<point>)` marks a place on a write path where a test injects a failure or a pause:
before or after a commit, before an external effect. The macro and the catalogue of 18 points
live in `api_failpoints`; the domain crates that place points (identity and access,
administration, server infrastructure, missions, match telemetry, operations) depend on it
normally and turn its `failpoints` feature on only through their dev-dependencies, as the
API server does. A test build carries the registry; the deploy build
(`cargo build --release -p api_server --bin api-server`) compiles every point to nothing. The
`failure_injection_*` integration binaries arm the points, and the
[failpoints README](/crates/api/api_failpoints/README.md) gives the arming rules.

### Ids

Every id at a public boundary of an API crate is a typed id from `api_identifiers`: one type per
concept (`EventId`, `MissionId`, `ServerId`, `DiscordUserId` and the rest), declared with the
`newtype_ids` macros. Each is serde- and sqlx-transparent, so its JSON, its text, its SQL bind and
an axum path parse are those of the bare `Uuid`, `String` or `i64`; the response goldens and stored
digests stay byte-equal. A field or public function parameter named `id` or `*_id` with a bare
type is refused by `cargo xtask verify crate-anatomy`. The
[identifiers README](/crates/api/api_identifiers/README.md) lists every id by domain.

Contract parity follows Law 9 of `CLAUDE.md`: the domain models are the snake_case source of
truth, the contract types are generated from `contracts/definitions/` into the
[contract_schema_types](/crates/contracts/contract_schema_types/README.md) crate by
`cargo xtask ci schema-codegen`, and the frontend DTOs in
`crates/frontend/foundation/frontend_api_dtos/src/` mirror the models under golden tests. The missions
domain's [contract layer](/crates/api/api_missions/src/contract/README.md) validates every
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
DTO in `crates/frontend/foundation/frontend_api_dtos/src/`. Acceptance of the API as a whole is
`cargo xtask verify api-readiness`, which judges the receipts described in the
[verification evidence](/documentation/crates/api/api_server/verification_evidence/README.md).

## Open work

- [T-940 — Website platform: events, telemetry, admin, content](/documentation/tickets/specs/t940_website_platform.md)
  (queued, [plan](/documentation/tickets/plans/t-940_plan.md)): the program whose open children
  follow.
- [T-940.10 — Mortar ballistics crate for API and offline frontend](/documentation/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation/tickets/plans/t-940_10_plan.md)): the scope is built by
  milestone B, [game ballistics](/documentation/crates/api/api_server/verification_evidence/game_ballistics.md):
  `fire_mission_planning`'s `solve_fire_mission` runs in the mortar page, which solves without the API
  and works offline, and in `POST /api/v1/fire-missions`, which re-solves every save; the
  registry closes the ticket with the milestone.
- [T-940.13 — Combat, medical and vehicle telemetry events](/documentation/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation/tickets/plans/t-940_13_plan.md)): the events schema
  (`contracts/definitions/match-telemetry.schema.json`), `POST /api/v1/ingest/match-events` and
  `GET /api/v1/matches/{matchId}/events` exist; the after-action replay that plays them does
  not.
- [T-952 — api set_var mutates a shared test process](/.ai/tickets/T-952.toml) (idea, no
  plan): the pool-setting tests in `crates/api/api_database/src/tests/connection.rs`
  stop mutating the process environment and use `DbPoolConfig::from_lookup`.
- [T-1131 — Decide whether Markdown edits should invalidate API readiness evidence](/.ai/tickets/T-1131.toml)
  (idea, no plan): whether documentation stays in the readiness fingerprint.

## Decisions

The [API decisions log](/documentation/crates/api/api_server/decisions.md) records each with its
context and consequences.

- The API is organised by domain, and a route's tier travels with its handler's extractor:
  a domain can be read, changed and tested whole, and no router position grants access.
- `/map-assets` sits below the rate limiter: terrain streaming is bytes, not requests, and a cold
  Mission Creator boot fetches hundreds of files at once.
- Configuration fails closed: a required variable left empty, or a set value that cannot work,
  stops the boot instead of surfacing later as an outage.
- Game hosts are reached only through work they claim: the game server host agent and the game runtime
  poll outbound with per-server machine credentials, and game servers fetch mission artifacts
  over HTTPS rather than from files staged on disk.
- Background workers own the schedule and nothing else: the work is a domain service, and only
  the binary imports the workers.
- The API server is a thin crate over the other API crates: the crate graph, not a source walk, holds the
  domain boundaries, and each crate builds and tests alone.
- Ids at a crate's public boundary are typed and wire-transparent: a mixed-up id is a compile
  error, and no response, digest or SQL bind changes.
