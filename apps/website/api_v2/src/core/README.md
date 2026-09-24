# API core

The foundations every domain of the [API](/documentation_v2/glossary.md#api) rests on:
configuration, the database pool and migrations, the shared application state, the handler error
type, the router with its middleware chain, authentication primitives, observability, and the
HTTP, text and wire-format helpers the domains reuse instead of writing their own.

## Contents

```text
apps/website/api_v2/src/core/
├── application_state.rs        `AppState`: the pool, config, token manager, hub, limiters and clients
├── authentication_primitives/  access tokens, opaque-token hashing and the session-authority trait
├── configuration/              `Config`, read from the environment at boot, and proxy parsing
├── database/                   the Postgres pool, the embedded migrations and SQLSTATE predicates
├── error_handling/             `ApiError`, the failure handlers return, and its JSON envelope
├── http/                       request-shape primitives: offset pagination
├── http_client/                the bounded retry for outbound calls answered with 429
├── http_router.rs              `router`: every route and mount, and the middleware chain
├── middleware/                 the request middleware chain and the authentication extractors
├── mod.rs                      the module tree
├── observability/              Prometheus metrics, `GET /metrics` and `GET /healthz`
├── realtime_hub/               the in-process publish-subscribe hub behind the SSE streams
├── tests/                      unit tests of the assembled router: metrics, health, route merging
├── text/                       the URL write guard, HTML sanitation and text previews
└── wire_format/                the RFC 3339 timestamp serializers and the `jsonb` passthrough type
```

## How it works

The `api` binary loads `Config`, opens the pool with `database::connect`, applies the migrations
with `database::migrate`, builds `AppState::new(pool, config)`, arms the
[background workers](/documentation_v2/glossary.md#background-workers), and
serves `http_router::router(state)`. `AppState` is the one dependency container: handlers and
middleware extract it whole or take one part (the pool, the config, the token manager, the hub,
the Discord and webhook clients, the session authority) through its `FromRef` implementations.

`router` nests the `/api/v1` tree, which merges the eight domains' route tables and adds no
prefix of its own, so a public URL is the path written in a domain's `routes.rs` with `/api/v1`
in front. Beside it the router serves `/healthz`, `/metrics`, the upload directory at `/uploads`
(created when the router is built), the terrain and glyph trees at `/map-assets` and
`/map-assets/glyphs` (a warning is logged at boot when either directory is missing), and, when
`SPA_DIST_DIR` is set, the built single-page app with an `index.html` fallback and the
cross-origin isolation headers. The middleware chain wraps all of it, outermost first: request
id, access log, metrics, panic recovery, CORS, body limit, rate limit; the two asset mounts sit
below the rate limit and never reach it.

A route's access tier is the extractor its handler takes (`AuthUser`, `LeaderUser`,
`MissionMakerUser`, `AdminUser`, `ServiceAuth`), never its position in the router. Logic that
more than one domain needs and that names no domain concept lives here: pagination, SQLSTATE
predicates, wire formats, the URL guard, the 429 retry, the token primitives.

## Public surface

- `http_router::router`: the whole application, for `apps/website/api_v2/src/bin/api.rs` and the
  integration suites, which exercise the same router.
- `application_state::AppState` and `AppState::new`: the state every handler and background
  worker receives.
- `configuration::Config` (`load`, `for_tests`, `require_discord_bot_token`, `is_development`)
  and `ConfigError`.
- `database`: `connect`, `migrate` and `connect_lazy` for the binaries and suites;
  `postgres_errors` for handlers that answer a constraint violation with a 4xx.
- `error_handling::api_error::ApiError`: the error every domain returns.
- `middleware`: the extractors, `json_error`, `role_rank`, `MAX_MULTIPART_BODY`,
  `authorized_event_stream::authorize_event_stream` for [SSE](/documentation_v2/glossary.md#sse)
  handlers, and `PgRateLimiter` for the
  bucket-pruning worker.
- `authentication_primitives`: `Manager` and `Claims`, `hash_token`, `random_token`,
  `constant_time_equal`, `numeric_code`, and the `SessionAuthority` trait that
  `identity_and_access` implements.
- `realtime_hub::Hub`, `http::pagination::PageParams`,
  `http_client::retry_on_429::send_with_retry_on_429`, the `text` guard and preview helpers, and
  the `wire_format` serializers.
- The HTTP routes `core` owns: `GET /healthz`, `GET /metrics` (service token), `/uploads`,
  `/map-assets`, `/map-assets/glyphs` and the single-page app fallback.

## Boundaries

- Depends on: `axum`, `tower-http`, `sqlx`, `tokio`, `jsonwebtoken`, `governor`, `reqwest` and
  the other crates in `apps/website/api_v2/Cargo.toml`; the migrations in
  `apps/website/api_v2/migrations/`, embedded at compile time; and, in the two composition-root
  files only, the domains: `application_state.rs` builds `identity_and_access`'s Discord client
  and session authority and `community_content`'s webhook client, and `http_router.rs` merges all
  eight route tables.
- Used by: `apps/website/api_v2/src/bin/api.rs` and
  `apps/website/api_v2/src/bin/import_registry.rs`; every domain module and
  `apps/website/api_v2/src/background_workers/`; the integration suites under
  `apps/website/api_v2/tests/`.
- Rules: `core` imports no domain outside `application_state.rs` and `http_router.rs`, and nothing
  from `background_workers`; the router merges every domain's `routes` table. The checks are
  `core_imports_no_domain_except_composition_root`, `background_workers_used_only_by_the_binary`
  and `every_domain_exports_a_route_table` in `apps/website/api_v2/src/tests/architecture_rules.rs`,
  which read code lines only, so a doc comment that names a domain creates no dependency.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — the routes of every domain.
