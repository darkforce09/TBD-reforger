# API HTTP layer

The `api_http_layer` crate: everything a request of the [API](/documentation/glossary/a_to_f.md#api)
passes through before its handler runs, and the shared services those layers keep. It signs and
verifies the access tokens, hashes opaque tokens, runs the middleware chain and the
authentication extractors, rate-limits in memory and in Postgres, counts every request into a
Prometheus registry, answers `/metrics` and `/healthz`, fans messages out to
[SSE](/documentation/glossary/n_to_z.md#sse) streams, and retries outbound calls answered with
`429`.

## Contents

```text
crates/api/api_http_layer/
├── Cargo.toml  the package: `axum`, `jsonwebtoken`, `governor`, `sqlx`, the API foundation crates, layout tier 3
└── src/        access tokens, middleware and extractors, observability, the realtime hub, the outbound retry
```

## How it works

Nothing here names the application state. Each extractor and middleware is generic over the
state `S` and takes the one part it reads through axum's `FromRef`: `AuthUser` the token
`Manager`, the `Config` and the `SessionAuthority`; `cors` the `CorsOrigins`; `rate_limit` the
`RateLimitState`; `authorize_event_stream` the session authority. The API application's
`AppState` holds those parts and implements `FromRef` for each, and its router mounts the layers,
so this crate depends on no domain and no application code.

A bearer token is accepted only HS256-signed, unexpired, with the platform's issuer and
audience, a nonempty subject and a non-nil session; the `SessionAuthority` the application
injects then reads the session and the account, so a revoked session or a changed role takes
effect on the next request. An opaque token is stored only as its SHA-256 hex
(`content_digest::sha256_hex`) and compared in constant time. The strict prefixes (`/api/v1/auth/`)
pass an in-memory tier and a Postgres tier that fails closed; the map asset mounts sit below the
rate-limit layer and never reach it.

## Getting started

Run from the repository root:

```bash
cargo test -p api_http_layer   # tokens, limiter tiers, client resolution, metrics, event streams; no database
cargo xtask db test-it --test durable_rate_limit --test forwarded_for_trust --test observability
```

The integration suites in `apps/api/tests/` prove the Postgres tier, the trusted proxies and the
metrics through the assembled router.

## Configuration

No feature and no variable of its own. The application's `Config` carries the values the layers
read: `JWT_SECRET` and `JWT_ACCESS_TTL_MIN` (the token `Manager`), `ALLOWED_ORIGINS` (CORS),
`TRUSTED_PROXIES` (client resolution) and `OBSERVABILITY_TOKEN` (`/metrics` and the detailed
`/healthz`).

## Public surface

- `authentication_primitives`: `Manager` and `Claims`, `SessionAuthority`, `hash_token`,
  `random_token`, `constant_time_equal`, `numeric_code`.
- `middleware`: `AuthUser`, `LeaderUser`, `MissionMakerUser`, `AdminUser`, `cors`,
  `rate_limit` with `RateLimitState`, `IpLimiter` and `PgRateLimiter`, `request_id`, `logging`,
  `authorize_event_stream`, `json_error`, `role_rank`, the body caps and the exempt mounts.
- `observability`: `Registry`, `observe`, `metrics_scrape`, `healthz`, `ObservabilityAuth`.
- `realtime_hub::Hub` and `http_client::retry_on_429::send_with_retry_on_429`.
- `Error` and `Result`, and `prelude` (`Claims`, `Manager`, the extractors, `json_error`, `Hub`).

## Boundaries

- Depends on: `api_foundation` (`ApiError`), `api_configuration` (`Config`, the proxy networks,
  the shutdown signal), `api_identifiers` (the Discord user and session ids), `content_digest`,
  and `axum`, `jsonwebtoken`, `governor`, `sqlx`, `reqwest`, `tokio`; the `rate_limit_buckets`
  table of migration `0021`.
- Used by: the API application (`apps/api`): its router and composition, and through it
  `api_state`, the kernel and domain crates, `api_background_workers` and the integration suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); nothing here names
  `AppState`, another kernel crate above it or a domain, which the crate boundary itself enforces.

## Related documentation

- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
- [API application source](/apps/api/src/README.md) — the router that mounts these layers.
- [Application state](/crates/api/api_state/README.md) — the state that holds their services.
- [Identity transactions](/documentation/apps/api/verification_evidence/identity_transactions.md)
  — how access tokens, persisted sessions and refresh rotation fit together.
