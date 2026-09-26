# Frontend source tree

The source of the single-page app, organised by domain: the shared foundations, the routed pages,
and the full-screen workspaces. Outside this folder, `apps/website/frontend/src/` holds only
`main.rs`, `app_routes.rs`, `router.rs` and their `tests/`.

## Contents

```text
apps/website/frontend/src/v2/
├── apps/    the full-screen workspaces: the Mission Creator, the debug benches, two placeholders
├── core/    the shared foundations: API client, session, interface primitives, utilities
├── mod.rs   the module tree
├── pages/   the routed pages, one folder per navigation area, and the navigation frame around them
└── tests/   unit tests for the documentation standard of every production file here
```

## How it works

`apps/website/frontend/src/app_routes.rs` binds each path to a route component: a page under
`pages/`, or a workspace under `apps/` for the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) and the debug benches. The
navigation frame in `pages/` wraps every route, and pages and workspaces fetch, gate and draw
through `core/`. The imports between the three run mostly one way:

| Child | Imports |
|---|---|
| `pages/` | `core/`, and in a few places the Mission Creator's `editor` workspace and the map engine |
| `apps/` | `core/` and `website-map-engine`; never `pages/`, never a sibling workspace |
| `core/` | `website-map-engine` in the wire types; nothing from `pages/`; the Mission Creator's `shell` module in four places, which the core README lists |

Code that touches `web_sys` or a live engine handle compiles for `wasm32` only, gated on its
`pub mod` line (which then carries the same `cfg` as the code it declares) or inside its file, so
`cargo test -p website-frontend` builds the native half of the whole tree. Test builds also get
the documentation audit, which `mod.rs` declares from `tests/doc_audit/` under `cfg(test)` and
which walks every production file of the tree.

## Public surface

- `core`: the [API](/documentation_v2/glossary.md#api) client, the wire types, the session store
  and [role](/documentation_v2/glossary.md#role) checks, the interface primitives and the
  utilities, read by `apps/website/frontend/src/router.rs` and throughout this tree.
- `pages` and `apps`: the route components that `apps/website/frontend/src/app_routes.rs` mounts,
  and `pages::navigation::layout::AppLayout`, which `apps/website/frontend/src/main.rs` mounts at
  startup.

## Boundaries

- Depends on: `crate::router` and `crate::app_routes`, in `apps/website/frontend/src/router.rs`
  and `apps/website/frontend/src/app_routes.rs`; `website-map-engine`, never the graphics engine;
  `leptos`, `leptos_router` and the browser bindings `apps/website/frontend/Cargo.toml` names.
- Used by: `apps/website/frontend/src/main.rs`, which declares `v2` and mounts the layout;
  `apps/website/frontend/src/app_routes.rs`, which routes to the pages and workspaces;
  `apps/website/frontend/src/router.rs`, which reads the roles of `core`.
- Rules:
  - every production file opens with a `//!` header, stays within 500 lines, documents every
    visible item, holds no inline test module and names no ticket or wave in a comment
    (`v2_production_files_meet_the_documentation_standard` in `tests/doc_audit/mod.rs`);
  - nothing imports `website_graphics_engine` (`cargo xtask verify engine-layers`, rule 6 of the
    [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md); the map engine's
    packet boundary, section 2C there, is the only path to the renderer);
  - a workspace imports from `core/` and `website-map-engine`, never from `pages/` or a sibling
    workspace, and nothing in `core/` imports from `pages/`; `core/` does import the Mission
    Creator's `shell` module, at the four places its README lists, so shared code is not
    independent of `apps/`; no gate checks these directions.

## Related documentation

- [Frontend documentation](/documentation_v2/website/frontend/README.md) — the route table: each
  route with its code folder and feature doc.
- [Page areas](/documentation_v2/website/frontend/pages/README.md) — the documentation of `pages/`,
  one folder per navigation area.
- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  Mission Creator's documents, starting from its roadmap.
