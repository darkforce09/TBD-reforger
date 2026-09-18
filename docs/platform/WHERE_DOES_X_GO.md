# Where does X go?

The canonical home for each artifact class.

| X | Home |
|---|---|
| SPA page module | `apps/website/frontend/src/<page>.rs` (one module per page; route in `src/router.rs`) |
| API endpoint | `apps/website/api_v2/src/<domain>/handlers/<surface>.rs`, registered in that domain's `routes.rs` |
| API logic two surfaces share | that domain's `apps/website/api_v2/src/<domain>/services/` |
| API wire/database model | `apps/website/api_v2/src/<domain>/models/` (the snake_case wire contract) |
| API logic naming no domain concept | `apps/website/api_v2/src/core/` (pagination, SQLSTATE predicates, wire formats, text guards, token primitives) |
| API background ticker | `apps/website/api_v2/src/background_workers/` (the work itself stays in the owning domain's `services/`) |
| DB migration | `apps/website/api_v2/migrations/NNNN_*.sql` (sqlx, embedded, runs on boot) |
| Data seed | `apps/website/api_v2/seeds/*.sql` (applied by root `cargo xtask db seed`; mock_data.sql manual-psql only) |
| Editor/gate smoke | `tools_v2/developer-tools` (`gate` bin) wired through `cargo xtask mk leptos-gates` |
| Test fixture | crate-local `tests/fixtures/` beside consumer; NEVER `.ai/artifacts/` |
| Cross-crate contract golden | `packages/tbd-schema/{schema,golden,golden-missions,registry}/` |
| Map asset | `packages/map-assets/<terrain>/` (LFS: dem png + sat .tbd-sat only; staging/tiles rebuildable local) |
| Ticket | `.ai/tickets/registry.json` + `./scripts/ticket sync` (generated TICKET_*.md never hand-edited) |
| Spec / doc | `docs/**` only — never `apps/**/docs` or `packages/**/docs` (verify-doc-layout enforces) |
| Ops script | `scripts/{website,mod,deploy}/` (mod scripts = tooling, distinct from OFF-LIMITS `apps/mod/`) |
| Shared engine code | `crates/map-engine-{core,render,wasm}` |
| Repo tooling | `xtask` (gates/codegen/ticket lib) · `tools_v2/developer-tools` (gate harness + asset pipelines) |

## Backend rule of thumb

A new endpoint goes in `apps/website/api_v2/src/<domain>/handlers/`, is registered in that domain's
`routes.rs`, and anything a second surface would need goes in that domain's `services/`. The eight
domains are `administration`, `command_center`, `community_content`, `identity_and_access`,
`match_telemetry`, `missions`, `operations`, `server_infrastructure`.

`core::http_router::api_v1_routes` merges the eight route tables and nests the result under
`/api/v1`, so the public URL is the literal written in the domain's `routes.rs` with `/api/v1` in
front of it. `core` imports no domain, a domain's handlers never import another domain's handlers,
and `background_workers` is imported only by `src/bin/api.rs` —
`apps/website/api_v2/src/tests/architecture_rules.rs` enforces all three.
