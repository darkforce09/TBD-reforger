# Frontend source root

The source root of the `frontend_application` crate, the thin app over the frontend crates: the WebAssembly
entry point that mounts the app, the render form of the route table, and the app frame around
every route. Every page, workspace, feature and shared foundation is a crate under
`crates/frontend/` (see [Frontend crates](/crates/frontend/README.md)); this folder holds none.

## Contents

```text
crates/frontend/shell/frontend_application/src/
├── app_routes.rs  `AppRoutes`: each path bound to the component that renders it, and the fallback
├── main.rs        the binary: the `app_routes` and `shell` modules and `start_app`, which mounts the app
└── shell/         the app frame around every route: layout, sidebar, top bar, membership status, not-found page
```

## How it works

`start_app` in `main.rs` is the module's only `#[wasm_bindgen(start)]` function, which
wasm-bindgen runs when the module instantiates; the binary's `main` stays empty. It installs the
panic hook (no linked library installs one), registers the Mission Creator's draft purge as a
sign-out hook, registers the offline service worker and mounts
`<div id="root"><Router><AppLayout/><OfflinePackRouteWatcher/></Router></div>` into the body;
`AppLayout`, in `shell/`, provides the session store and the toast queue, restores a stored
session, and renders `AppRoutes` inside the frame the route asks for. On a native build `main` is
empty and nothing mounts.

`app_routes.rs` binds each path to a route component of a page crate under
`crates/frontend/pages/`, or of a workspace crate under `crates/frontend/workspaces/` for the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), its read-only review
workspace and the debug benches, with `NotFoundPage` as the fallback. The contract form of the
table, each route's layout flags and access tier, is the `frontend_route_table` crate
(`crates/frontend/foundation/frontend_route_table/`), whose README lists every route.

The frontend's layers stack in one order, and a layer depends only on the layers below it:

| Layer | Depends on |
|---|---|
| foundation crates (`crates/frontend/foundation/`) | each other in their crate order, and the library crates outside `crates/frontend` |
| feature crates (`crates/frontend/features/`) | the foundation crates |
| page crates (`crates/frontend/pages/`) | the foundation and feature crates; never a workspace, never another page crate |
| workspace crates (`crates/frontend/workspaces/`) | the foundation and feature crates and the map crates; never a page; the Mission Creator's crates in their crate order |
| shell crates (`crates/frontend/shell/`): this app (`shell/`, `main.rs`, `app_routes.rs`) and the offline service worker, peers that never depend on each other | the app every layer below; the worker `offline_cache_policy` alone |

`cargo xtask verify frontend-layering` judges this order: the in-crate table over the app's own
modules, which are all the shell, and the crate edges between the frontend crates, with no
exception.

The mount chain compiles for `wasm32` only: `app_routes.rs` and the frame's components
(`AppLayout`, the sidebar, the top bar, the membership status strip and `NotFoundPage`) carry
`cfg(target_arch = "wasm32")`, so `cargo test -p frontend_application` builds the frame's native,
pure half (the frame classifier, the active-link rule and the account badge) with its tests.

## Public surface

- `start_app`: the binary's one JavaScript export, its WebAssembly start function.
- `shell`: `layout::AppLayout`, which `start_app` mounts, and `not_found::NotFoundPage`, the
  route table's fallback.
- `app_routes::AppRoutes`, which `AppLayout` renders.

## Boundaries

- Depends on: the page and workspace crates whose route components the route table mounts; the
  foundation crates the frame reads (`frontend_session`, `frontend_route_table`, `frontend_ui`,
  `frontend_api_dtos`, `frontend_transport`, `frontend_offline`); `leptos` and `leptos_router`
  for the router, `wasm-bindgen` for the start function, `console_error_panic_hook`. The
  manifest, `crates/frontend/shell/frontend_application/Cargo.toml`, names each.
- Used by: `crates/frontend/shell/frontend_application/index.html`, whose `rust` link has Trunk
  build this binary; the headless browser gates of `tools/browser_testing/browser_gate_suites/`,
  which drive the built app by its routes.
- Rules:
  - page, workspace, feature and foundation code lives in its crate under `crates/frontend/`,
    never here;
  - the layer order above, checked by `cargo xtask verify frontend-layering`.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) —
  the route table: each route with its code folder and feature doc.
- [Frontend crate documentation](/documentation/crates/frontend/README.md) — the feature docs of
  the page and workspace crates.
- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) — the
  Mission Creator's documents, starting from its roadmap.
- [App layout and navigation](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md)
  — the frame, the sidebar, the top bar and the not-found page.
