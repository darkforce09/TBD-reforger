# Architecture

Frontend/API route contracts, editor ORBAT coherency, engine dependency layers, SQL query policy, and source-file limits. Separate test modules cover positive cases and deliberately broken enforcement.

`route_tags.rs` cross-checks the `@route` doc tags in `apps/website/api_v2/src` against the routes the crate actually registers, in both directions and keyed on (method, path, handler). The registrations are read from the domain route tables at `src/<domain>/routes.rs` — one column-0 `pub fn routes` each — and their union is the router side; `core/http_router.rs` is read only for its shape (`fn api_v1_routes`, nested at `/api/v1`) and for the `.merge(crate::<domain>::routes(` lines, whose domain set must equal the set of discovered route files.

Source modules: `editor_orbat_coherency.rs`, `engine_layer_boundaries.rs`, `engine_layer_rules.rs`, `engine_layer_scan.rs`, `mod.rs`, `route_tags.rs`.
