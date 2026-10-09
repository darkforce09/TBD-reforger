# API server source

The source of the `api_server` crate: the thin application the API crates under `crates/api/` are
assembled into. It holds the router, the composition root, the binaries' error, the prelude, the
two binaries and the layout and prose rules; every domain, the kernel below them and the
[background workers](/documentation/glossary/a_to_f.md#background-workers) are API crates.

## Contents

```text
crates/api/api_server/src/
├── bin/             the `api-server` server and the `import-item-registry` tool
├── composition.rs   the composition root: the application state with its concrete services
├── error.rs         `Error` and `Result`: why a binary stops, printed with its cause chain
├── lib.rs           the `api_server` library root: the module tree and the source-rule tests
├── prelude.rs       `router` and `application_state` for glob import
├── router.rs        `router`: every route and mount, and the middleware chain
└── tests/           the layout and prose rules, and the router's unit tests
```

## How it works

The `api-server` binary loads `Config`, opens the pool with `api_database::connect`, applies the
migrations with `api_database::migrate`, builds the state with
`composition::application_state(pool, config)`, arms the workers with
`api_background_workers::spawn_all`, and serves `router::router(state)`. On SIGINT or SIGTERM it
begins `api_configuration::process_lifecycle::process_shutdown`, which closes every open
[SSE](/documentation/glossary/n_to_z.md#sse) stream so the graceful drain completes.

`error.rs` holds `Error`, every failure a binary ends on: a refused configuration variable, the
pool, a migration, a socket or file operation, a registry import, and the two command-line
refusals of `import-item-registry`. Each `main` returns `Result`, so the runtime prints
`Error: ` and the `Debug` rendering, which is the message followed by its cause chain under
`Caused by:`, and exits 1.

`composition.rs` constructs the services that need a concrete implementation (the database
session authority, the Discord OAuth2 and webhook clients, the equipment datasets) and injects
them into `api_state::AppState::new`, so the state names no domain. `AppState` is the one
dependency container: handlers extract it whole or take one part through its `FromRef`
implementations, and the middleware of `api_http_layer` reads only its sub-states.

`router.rs` nests the `/api/v1` tree, which merges the eight domain crates' route tables
(`api_<domain>::routes`) and adds no prefix of its own, so a public URL is the path written in a
domain's `routes.rs` with `/api/v1` in front. Beside it the router serves `/healthz`, `/metrics`,
the upload directory at `/uploads` (created when the router is built), the terrain and glyph trees
at `/map-assets` and `/map-assets/glyphs` from the directories the configuration resolved (a
warning is logged at boot when either directory is missing), and, when `SPA_DIST_DIR` is set, the built single-page app with an `index.html`
fallback and the cross-origin isolation headers, its offline service worker loader
`/service_worker.js` with `Cache-Control: no-cache`. The middleware chain wraps all of it,
outermost first: request id, access log, metrics, panic recovery, CORS, body limit, rate limit;
the two asset mounts sit below the rate limit and never reach it, which `tests/router.rs` pins by
the order of the two registrations in `router.rs`.

```text
bin/api_server.rs ─▶ composition ─▶ api_state and the concrete services of the API crates
bin/api_server.rs ─▶ router ─▶ the eight domain route tables ─▶ handlers ─▶ services ─▶ models
bin/api_server.rs ─▶ api_background_workers ─▶ domain services
```

A route's access tier is the extractor its handler takes (`AuthUser`, `LeaderUser`,
`MissionMakerUser` and `AdminUser` from `api_http_layer::middleware`, the caller identity crate's
`MachineCaller` for game hosts, and `ObservabilityAuth` from
`api_http_layer::observability::observability_auth` for the operator's scraper), never its
position in the router. Every refusal a handler answers carries the `{error, details?}` envelope of
`api_foundation::error_handling`, extractor rejections included.

A new endpoint is a handler in `crates/api/api_<domain>/src/handlers/` carrying its
`/// @route <METHOD> <path>` tag, a registration in that domain's `routes.rs`, and, for anything a
second caller needs, a function in that domain's `services/`. An id at a public boundary is a
typed id from `api_identifiers` (`crates/api/api_identifiers/`), whose JSON, text and SQL bind are
those of the bare `Uuid`, `String` or `i64`. Unit tests live in sibling files under a `tests/`
folder, declared with `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

## Public surface

- The library `api_server` (`lib.rs`): `router::router` and `composition::application_state`, also in
  `prelude`, for the binaries and the integration suites in `crates/api/api_server/tests/`, which
  serve the same router; `Error` and `Result`, which the binaries return.
- The binaries `api-server` and `import-item-registry`, in `bin/`.
- The HTTP routes the router owns: `GET /healthz` (public status; the detailed report with the
  `OBSERVABILITY_TOKEN` bearer), `GET /metrics` (`OBSERVABILITY_TOKEN` bearer), `/uploads`,
  `/map-assets`, `/map-assets/glyphs` and the single-page app fallback; every `/api/v1` route
  belongs to a domain crate.

## Boundaries

- Depends on: the API crates in `crates/api/api_server/Cargo.toml`: the eight domain crates, whose route tables
  the router merges; `api_background_workers`; `api_state`, `api_caller_identity`, `api_discord`
  and `api_equipment_datasets`, which the composition root assembles; `api_http_layer`, whose
  middleware, metrics and health surfaces the router mounts; `api_configuration` and
  `api_database`; axum, tower, tower-http and thiserror.
- Used by: the integration suites in `crates/api/api_server/tests/`; through the binaries, the
  `cargo xtask` recipes, the release image and the systemd unit that run them; over HTTP, the
  single-page app in `crates/frontend/shell/frontend_application/`, the game servers through
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`, and the
  [game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) in
  `crates/fleet/game_server_host_agent/`.
- Rules (`tests/architecture_rules.rs`, which reads the source and the Cargo manifests of this
  crate and of every API crate):
  - `src/` holds only `lib.rs`, `router.rs`, `composition.rs`, `error.rs`, `prelude.rs`, the two
    binaries, `tests/` and this README;
  - the kernel crates depend on no domain crate, and the domain crates depend on one another only
    along the one-way domain graph (`DOMAIN_DEPENDENCIES`), read from their manifests; that
    manifest assertion replaced the source-import layer ratchet once no layer was left inside one
    crate;
  - only this crate depends on `api_background_workers`, and only `bin/api_server.rs` names it;
  - no crate imports another domain's handlers, and each domain crate defines one route table in
    its `routes.rs`, which `router.rs` merges;
  - no source file of this crate or of an API crate holds an inline `mod tests` body, a ticket id
    or a comparison with another implementation.
- Prose rules (`tests/prose_rules.rs`): the Rust files of `src/`, of the integration suites and of
  every API crate's `src/`, `.env.example`, and the comment lines of the seeds and migrations in
  `crates/api/api_database/` carry no ticket ids, no comparison with another implementation, no
  delivery-process vocabulary and no path the crates lack.
- Every `@route` tag resolves to a route a table registers, and every registered route to a tag
  (`cargo xtask verify route-tags`).

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — the routes of every domain.
- [API HTTP layer](/crates/api/api_http_layer/README.md) — the middleware, extractors, rate
  limiters and metrics the router mounts.
- [API application state](/crates/api/api_state/README.md) — the state the router carries and
  its `FromRef` projections.
- [API crates](/crates/api/README.md) — the kernel, domain and worker crates the application is
  assembled from.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the
  module headers and the `@route` and `@contract` tags the source carries.
