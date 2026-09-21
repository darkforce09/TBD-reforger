# `core/`

The cross-cutting foundations every domain rests on: runtime configuration, the Postgres
connection lifecycle, the shared application state, the single handler error type, the router and
its global middleware chain, observability, credential primitives, and the HTTP / text /
wire-format primitives the domains reuse instead of re-deriving.

## Public surface

- **`http_router::router(state)`** — builds the whole application: `/healthz`, `/metrics`, the
  `/api/v1` tree, the static `/uploads` and `/map-assets` mounts, the SPA fallback, and the global
  middleware chain. `api_v1_routes` merges the eight domain route tables and nests the result under
  `/api/v1`; it adds no prefix of its own, so a public URL is the literal written in the domain's
  `routes.rs`.
- **`application_state::AppState`** — the dependency container handlers and middleware extract from
  (pool, config, JWT manager, realtime hub, rate-limit state, Discord and webhook clients).
- **`error_handling::api_error::ApiError`** — the failure type every handler returns.
- **`http::pagination`**, **`text/`**, **`wire_format/`**, **`database::postgres_errors`**,
  **`http_client::retry_on_429`**, **`authentication_primitives/`**, **`realtime_hub`**,
  **`middleware/`**, **`observability/`** — the shared floor. Logic that more than one domain needs,
  and that names no domain concept, belongs here rather than in whichever domain reached for it
  first.

## Dependency rules

- `core` imports no domain. The two exceptions are `application_state.rs` (it holds the Discord and
  webhook service types, which live in their domains) and `http_router.rs` (it merges the eight
  domain route tables). `src/tests/architecture_rules.rs` enforces this.
- Every domain may import `core`. Nothing in `core` may import `background_workers`.
- Doc links (`///`, `//!`) naming a domain path are pointers for the reader and create no
  dependency; the rules read code lines only.

## Files

```text
mod.rs                                 Module tree of the shared foundations.
application_state.rs                   Shared application state injected into handlers and middleware.
http_router.rs                         Router assembly, the `/api/v1` merge, and the global middleware chain.
authentication_primitives/
  mod.rs                               Credential primitives shared by every authenticated surface.
  jwt_manager.rs                       HS256 access-token issuance and verification.
  token_hashing.rs                     Opaque tokens: generation, SHA-256 storage hashing, constant-time compare.
configuration/
  mod.rs                               Runtime configuration read from the environment at boot.
  proxy_network.rs                     `TRUSTED_PROXIES` address/CIDR parsing and peer matching.
database/
  mod.rs                               Postgres connection lifecycle: tuned pool, startup retry budget, migrations.
  connection_pool.rs                   Pool tuning read from the four `TBD_DB_POOL_*` environment variables.
  postgres_errors.rs                   Classifying a `sqlx::Error` by the SQLSTATE it carries.
error_handling/
  mod.rs                               The single failure type every handler returns and its JSON envelope.
  api_error.rs                         `ApiError` and the `{"error": msg}` rendering.
http/
  mod.rs                               Request-shape primitives shared by every feature surface.
  pagination.rs                        Offset pagination parameters and the clamping rule list endpoints share.
http_client/
  mod.rs                               Outbound HTTP behaviour shared by every client this service speaks through.
  retry_on_429.rs                      Bounded `429 Too Many Requests` retry honoring `Retry-After`.
middleware/
  mod.rs                               The global request middleware chain and the primitives its layers share.
  authentication.rs                    Authentication and role authorization, expressed as axum extractors.
  client_identity.rs                   Resolving which client a request came from, for the rate limiters.
  cross_origin.rs                      CORS: reflects an allow-listed `Origin`, never `*`.
  durable_ratelimit.rs                 Cross-process rate limiting on Postgres — the L2 tier.
  rate_limiting.rs                     Per-client rate limiting, in-memory L1 over durable L2.
  tracing_correlation.rs               Request correlation id and the structured access log line.
observability/
  mod.rs                               Prometheus metrics and the health probe.
  health_probe.rs                      `GET /healthz`: database and migration-state probe.
  metrics_exposition.rs                Prometheus text exposition 0.0.4 and `GET /metrics`.
  metrics_registry.rs                  Metric families, their label keys, and the cardinality cap.
  request_observer.rs                  The middleware that feeds the metrics registry.
realtime_hub/
  mod.rs                               In-process publish/subscribe hub fanning messages out to SSE clients.
text/
  mod.rs                               Text handling shared across the crate.
  html_sanitizer.rs                    HTML sanitation and the plain-text preview helpers.
  http_url_guard.rs                    The crate's URL write-boundary guard for absolute `http`/`https` strings.
wire_format/
  mod.rs                               The JSON wire contract's shared serialization primitives.
  rfc3339_timestamps.rs                The RFC 3339 UTC timestamp and midnight-UTC date `#[serde(with = …)]` modules the models share.
  raw_json.rs                          The `jsonb` passthrough column type.
tests/
  http_router.rs                       Sibling unit tests for `http_router.rs`.
authentication_primitives/tests/
  jwt_manager.rs                       Sibling unit tests for `jwt_manager.rs`.
  token_hashing.rs                     Sibling unit tests for `token_hashing.rs`.
configuration/tests/
  configuration.rs                     Sibling unit tests for `configuration/mod.rs`.
  proxy_network.rs                     Sibling unit tests for `proxy_network.rs`.
database/tests/
  connection.rs                        Sibling unit tests for `database/mod.rs`.
  connection_pool.rs                   Sibling unit tests for `connection_pool.rs`.
http_client/tests/
  retry_on_429.rs                      Sibling unit tests for `retry_on_429.rs`.
middleware/tests/
  client_identity.rs                   Sibling unit tests for `client_identity.rs`.
  rate_limiting.rs                     Sibling unit tests for `rate_limiting.rs`.
observability/tests/
  metrics_exposition.rs                Sibling unit tests for `metrics_exposition.rs`.
  metrics_registry.rs                  Sibling unit tests for `metrics_registry.rs`.
realtime_hub/tests/
  hub.rs                               Sibling unit tests for `realtime_hub/mod.rs`.
text/tests/
  html_sanitizer.rs                    Sibling unit tests for `html_sanitizer.rs`.
  http_url_guard.rs                    Sibling unit tests for `http_url_guard.rs`.
```

Unit tests live in these sibling files, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`. Inline `mod tests` blocks are rejected by
`src/tests/architecture_rules.rs`.
