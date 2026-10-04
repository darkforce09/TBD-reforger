# Route table source

The source tree of `frontend_route_table`: the route table with its readers and the sidebar's
navigation menu. The [crate README](../README.md) lists every route with its tier and layout.

## Contents

```text
crates/frontend/foundation/frontend_route_table/src/
├── lib.rs              the module tree and the root re-exports of the route table's items
├── navigation_menu.rs  `NAVIGATION`: the sidebar's sections and links, each with its lowest role
├── prelude.rs          `RouteDef` and the five readers
├── routes.rs           `ROUTES`: each route's layout flags and access tier, and their readers
└── tests/              unit tests for the route table's access tiers and redirects
```

## How it works

`routes.rs` holds the `static ROUTES: &[RouteDef] = &[…];` table in the shape the route drift gate
parses as text, and the private `match_route` every reader goes through. `navigation_menu.rs` is
a second static table that names no route by reference: its paths are strings the sidebar compares
with the live pathname. `lib.rs` re-exports the table and its readers at the crate root, so callers
write `frontend_route_table::breadcrumb`.

## Boundaries

- Depends on: `frontend_api_dtos::role`.
- Used by: the crate's callers through `lib.rs`; the route drift gate reads `routes.rs` directly.
- Rules: the `ROUTES` table stays in `routes.rs`, a top-level `static` whose closing `];` sits in
  column 0, which is what the route drift gate's parse anchors on; no comment above the table
  spells the declaration itself, since the parse takes the first match for the table.
