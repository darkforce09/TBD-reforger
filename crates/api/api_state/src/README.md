# API application state source

The source of `api_state`: `AppState`, the one dependency container the API's handlers,
middleware and background workers receive, and the `FromRef` projections that hand out one part
of it.

## Contents

```text
crates/api/api_state/src/
├── application_state.rs  `AppState`: the pool, config, token manager, hub, limiters, injected services, metrics registry
├── lib.rs                the crate root: module header, `mod` lines and the re-export of `AppState`
├── prelude.rs            `AppState` for glob import
└── tests/                the pin that the durable strict tier matches the in-memory strict limiter
```

## How it works

`AppState::new` takes the open pool, the loaded configuration and the four services that need a
concrete implementation: the session authority (as `Arc<dyn SessionAuthority>`), the Discord
OAuth2 client, the announcement webhook and the equipment datasets. It builds the rest itself:
the token manager, the CORS allow-list, the rate-limit state (global 20 req/s burst 40, strict
1 req/s burst 10, the durable strict tier on the pool), the realtime hub and the metrics registry.
Every field is an `Arc` or a pool handle, so a clone shares them all.

The `FromRef` implementations project the pool, the configuration, the token manager, the hub,
the Discord and webhook clients, the session authority, the CORS allow-list and the rate-limit
state. The extractors and middleware of `api_http_layer` read only these projections, so they
never name `AppState`.

## Boundaries

- Depends on: `api_http_layer` (`Manager`, `SessionAuthority`, `CorsOrigins`, `RateLimitState`,
  `IpLimiter`, `Registry`, `Hub`), `api_configuration` (`Config`), `api_discord`
  (`DiscordService`, `WebhookService`), `api_equipment_datasets` (`EquipmentDatasets`), axum and
  sqlx.
- Used by: the API application (`crates/api/api_server`): its composition root, which builds the state, the
  router, every domain handler, the background workers and the integration suites.
- Rules: the state names no domain and no concrete session authority; the strict limiter's
  numbers equal the durable strict tier's (`tests/application_state.rs`).
