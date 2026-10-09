# API application state

The `api_state` crate: the application state of the [API](/documentation/glossary/a_to_f.md#api),
the one dependency container every handler, middleware and background worker receives. It holds
the pool, the configuration, the token manager, the session authority, the CORS allow-list, the
rate limiters, the realtime hub, the metrics registry, the Discord and webhook clients and the
equipment datasets, and names no domain.

## Contents

```text
crates/api/api_state/
├── Cargo.toml  the package: `api_http_layer`, `api_configuration`, `api_discord`, `api_equipment_datasets`, axum, sqlx (`postgres`), layout tier 5
└── src/        `AppState`, its `FromRef` projections, the prelude and the limiter sizing pin
```

## How it works

The API's composition root (`api_server::composition::application_state`) builds the concrete services
and passes them to `AppState::new`: the database session authority of `api_caller_identity` as an
`Arc<dyn SessionAuthority>`, the Discord OAuth2 client and announcement webhook of `api_discord`,
and the `EquipmentDatasets` of `api_equipment_datasets`. The state builds the token manager, the
CORS allow-list, the rate-limit state, the hub and the metrics registry itself. A handler extracts
the whole state; an extractor or a middleware of `api_http_layer` takes only the part it needs
through a `FromRef` projection.

## Getting started

Run from the repository root:

```bash
cargo test -p api_state
cargo clippy -p api_state --all-targets -- -D warnings
```

## Configuration

No feature and no variable of its own; the state carries the `Config` the API loads at boot.

## Public surface

- `AppState` and `AppState::new` (the session authority, the Discord and webhook clients and the
  equipment datasets injected).
- `FromRef<AppState>` for `PgPool`, `Arc<Config>`, `Arc<Manager>`, `Arc<Hub>`,
  `Arc<DiscordService>`, `Arc<WebhookService>`, `Arc<dyn SessionAuthority>`, `CorsOrigins` and
  `RateLimitState`.
- `prelude` (`AppState`).

## Boundaries

- Depends on: `api_http_layer`, `api_configuration`, `api_discord`, `api_equipment_datasets`,
  axum and sqlx.
- Used by: the API application (`crates/api/api_server`): its composition root, router, domains, background
  workers and integration suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); the state names no domain
  and holds the session authority only as the `api_http_layer` trait object.

## Related documentation

- [API application state source](/crates/api/api_state/src/README.md) — the file, its
  projections and the limiter sizing pin.
- [API HTTP layer](/crates/api/api_http_layer/README.md) — the middleware and extractors that read
  the projections.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
