# Website API source

The source of the `website-api` crate: the `website_api` library, split into a shared `core`,
eight domains and the [background workers](/documentation_v2/glossary/a_to_f.md#background-workers),
and the two binaries built on it.

## Contents

```text
apps/website/api_v2/src/
├── administration/         member roster, bans and warnings, the Discord role resync, the audit log
├── background_workers/     the interval tasks the `api` binary arms at boot
├── bin/                    the `api` server and the `import-registry` tool
├── command_center/         the dashboard, leaderboards and per-player statistics
├── community_content/      announcements, the wiki, the vehicle database, modpacks and uploads
├── core/                   configuration, database, state, errors, router, middleware, shared helpers
├── identity_and_access/    Discord sign-in, sessions, the caller's profile, the Arma link handshake
├── lib.rs                  the `website_api` library root: the module tree and the source-rule tests
├── match_telemetry/        game-runtime heartbeats and finished match results
├── missions/               missions, versions, artifacts, reviews, deployments, armory, registries
├── operations/             events, ORBAT slotting, reservations, service records, fire missions
├── server_infrastructure/  servers, live status, machine credentials, fleet commands, runtime sessions
└── tests/                  source-text rules for layout and prose, and the property-test recorder
```

## How it works

The crate is split by domain, not by layer. `core` is the floor every other module stands on.
Each of the eight domains owns one slice of the [API](/documentation_v2/glossary/a_to_f.md#api) with
the same shape: `mod.rs`, a `routes.rs` that exports `pub fn routes`, and `handlers/`,
`services/` and `models/` folders (`command_center` has no models of its own); `missions` adds
`contract/` and `validation/`.
`core::http_router` merges the eight route tables under `/api/v1` without adding a prefix, so a
public URL is the path written in a domain's `routes.rs` with `/api/v1` in front. The binaries in
`bin/` compose the library, and only `bin/api.rs` arms `background_workers`.

```text
bin/api.rs ─▶ core (configuration, database, state, router)
                 └─▶ the eight domain route tables ─▶ handlers ─▶ services ─▶ models
bin/api.rs ─▶ background_workers ─▶ domain services
```

Dependencies point one way. A domain's handlers, services and models may use `core` and another
domain's services and models, never its handlers. Logic that more than one domain needs goes to
`core` when it names no domain concept (pagination, SQLSTATE checks, wire formats, the URL guard);
logic that names one stays in its domain's `services/`, and the other domains call it there: the
user lookup of `identity_and_access`, the [mission](/documentation_v2/glossary/g_to_m.md#mission)
lookups and cargo catalog of `missions`, the audit writer of `administration`, the modpack lookup
of `community_content`, the status broadcast of `server_infrastructure`, and the
[event](/documentation_v2/glossary/a_to_f.md#event) status rules of `operations`. Mortar ballistics live
in the map engine's `apps/website/map-engine/src/data/scenario/ballistics/`.

A new endpoint is a handler in `<domain>/handlers/` carrying its `/// @route <METHOD> <path>` tag,
a registration in that domain's `routes.rs`, and, for anything a second caller needs, a function
in that domain's `services/`. Unit tests live in sibling files under a `tests/` folder, declared
from the production file with `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`. The
`generated/` folders under `missions/contract/` and under the `models/` of `identity_and_access`,
`missions`, `operations` and `server_infrastructure` hold types generated from
`contracts_v2/definitions/`.

## Public surface

- The library `website_api` (`lib.rs`): `core`, `background_workers` and the eight domain
  modules, all public, for the binaries and the integration suites in
  `apps/website/api_v2/tests/`.
- The binaries `api` and `import-registry`, in `bin/`.
- The HTTP surface: each domain's routes under `/api/v1`, and `core`'s `/healthz`, `/metrics`,
  `/uploads`, `/map-assets`, `/map-assets/glyphs` and single-page app fallback.

## Boundaries

- Depends on: the crates of `apps/website/api_v2/Cargo.toml`, `website-map-engine` among them for
  the mission compiler; the schemas in `contracts_v2/definitions/`, embedded at compile time; the
  migrations in `apps/website/api_v2/migrations/`.
- Used by: the integration suites in `apps/website/api_v2/tests/`; through the binaries, the
  `cargo xtask` recipes, the release image and the systemd unit that run them; over HTTP, the
  single-page app in `apps/website/frontend/`, the game servers through
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`, and the
  [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) in `apps/fleet_host_agent/`.
- Rules:
  - `core` imports no domain outside `core/application_state.rs` and `core/http_router.rs`; a
    domain never imports another domain's handlers; only `bin/api.rs` (and `lib.rs`, which
    declares it) names `background_workers`; every domain exports a route table the router
    merges; `src/` holds no top-level `handlers/`, `services/`, `models/`, `contract/` or `auth/`
    folder and no top-level `app.rs`, `state.rs`, `db.rs`, `config.rs` or `realtime.rs`; no
    production file holds an inline `mod tests` body. `tests/architecture_rules.rs` checks each.
  - The Rust files of `src/` and of the integration suites, `.env.example`, and the comment lines
    of the seeds and migrations carry no ticket ids, no comparison with another implementation,
    no delivery-process vocabulary and no path the crate lacks (`tests/prose_rules.rs`).
  - The `generated/` folders are written by `cargo xtask ci schema-codegen` and never edited by
    hand (`cargo xtask ci verify-codegen-fresh`); `missions/contract/loadout_projection.rs` is the
    one contract model maintained by hand, because the generator's output for the loadout
    export's versioned root loses fields.
  - The snake_case models under each domain's `models/` are the API's wire contract; the
    single-page app's DTOs in `apps/website/frontend/src/v2/core/api/dto/` mirror them under
    golden tests, so a model change updates both.
  - Every `@route` tag resolves to a route a table registers, and every registered route to a
    tag (`cargo xtask verify route-tags`).

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — the routes of every domain.
- [Documentation standards](/documentation_v2/standards/documentation_standards.md) — the
  module headers and the `@route` and `@contract` tags the source carries.
