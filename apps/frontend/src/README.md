# Frontend source root

The source root of the `frontend` crate: the WebAssembly entry point that mounts the app, the
render form of the route table, and the five layers that hold every shared foundation, feature,
page, workspace and the app frame.

## Contents

```text
apps/frontend/src/
├── app_routes.rs  `AppRoutes`: each path bound to the component that renders it, and the fallback
├── features/      product capabilities pages and workspaces share: the mission review record
├── foundation/    the transport, the session, the route table, interface primitives, utilities
├── main.rs        the binary: the layer modules and `start_app`, which mounts the app
├── pages/         the routed pages, one folder per navigation area
├── shell/         the app frame around every route: layout, sidebar, top bar, not-found page
├── tests/         the documentation audit of every production file of the crate
└── workspaces/    the full-screen workspaces: the Mission Creator, the debug benches, two placeholders
```

## How it works

`start_app` in `main.rs` is the module's only `#[wasm_bindgen(start)]` function, which
wasm-bindgen runs when the module instantiates; the binary's `main` stays empty. It installs the
panic hook (no linked library installs one), registers the offline service worker and mounts `<div id="root"><Router><AppLayout/></Router></div>` into the body;
`AppLayout`, in `shell/`, provides the session store and the toast queue, restores a stored
session, and renders `AppRoutes` inside the frame the route asks for. On a native build `main` is
empty and nothing mounts.

`app_routes.rs` binds each path to a route component: a page under `pages/`, or a workspace under
`workspaces/` for the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), its
read-only review workspace and the debug benches, with `NotFoundPage` as the fallback. The contract
form of the table, each route's layout flags and access tier, is `foundation/route_table/`, whose
README lists every route.

The layers stack in one order, and a layer imports only the layers below it:

| Layer | Imports |
|---|---|
| `foundation/` | the map crates (`map_renderer`, `map_streaming_host`, `map_streaming_model`), in the map seam |
| `features/` | `foundation/` |
| `pages/` | `foundation/` and `features/`; never a workspace, never another page area |
| `workspaces/` | `foundation/`, `features/` and the map crates; never a page, never a sibling workspace |
| `shell/`, `main.rs`, `app_routes.rs` | every layer below |

`cargo xtask verify frontend-layering` counts the imports that break this order against a ceiling;
the remaining ones are the foundation's four imports of the Mission Creator's `session` module and
the mission library's two imports of the Mission Creator.

Code that touches `web_sys` or a live engine handle compiles for `wasm32` only, gated on its
`pub mod` line (which then carries the same `cfg` as the code it declares) or inside its file, so
`cargo test -p frontend` builds the native half of the whole tree. Test builds also get the
documentation audit, which `main.rs` declares from `tests/doc_audit/` under `cfg(test)` and which
walks every production file below this folder.

## Public surface

- `foundation`: the [API](/documentation/glossary/a_to_f.md#api) transport, the wire types, the
  route table, the session store and [role](/documentation/glossary/n_to_z.md#role) checks, the
  interface primitives and the utilities.
- `features`: the mission review record the mission hub, the approvals queue and the review
  workspace show.
- `pages` and `workspaces`: the route components that `app_routes.rs` mounts.
- `shell`: `layout::AppLayout`, which `main.rs` mounts at startup, and `NotFoundPage`.

## Boundaries

- Depends on: `leptos` and `leptos_router` for the router, `wasm-bindgen` for the start function,
  `console_error_panic_hook`; the map crates; the browser bindings
  `apps/frontend/Cargo.toml` names.
- Used by: `apps/frontend/index.html`, whose `rust` link has Trunk build this binary; the headless
  browser gates of `tools/browser_testing/browser_gate_suites/`, which drive the built app by its
  routes.
- Rules:
  - every production file opens with a `//!` header, stays within 500 lines, documents every
    visible item, holds no inline test module and names no ticket or wave in a comment
    (`frontend_production_files_meet_the_documentation_standard` in `tests/doc_audit/mod.rs`);
  - the layer order above, checked by `cargo xtask verify frontend-layering`.

## Related documentation

- [Frontend documentation](/documentation/apps/frontend/README.md) — the route table: each
  route with its code folder and feature doc.
- [Page areas](/documentation/apps/frontend/pages/README.md) — the documentation of `pages/`,
  one folder per navigation area.
- [Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md) — the
  Mission Creator's documents, starting from its roadmap.
- [App layout and navigation](/documentation/apps/frontend/shell/app_layout_and_navigation.md)
  — the frame, the sidebar, the top bar and the not-found page.
