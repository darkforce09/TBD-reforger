# Frontend foundation crates

The lowest layer of the frontend crates: the building blocks every feature, page, workspace and
the app read, with no business logic of their own — the transport to the
[API](/documentation/glossary/a_to_f.md#api) and its wire types, the route table, the session and
its [role](/documentation/glossary/n_to_z.md#role) checks, the shared map mount, the offline pack,
the interface primitives with the small utilities, and the test helpers.

## Contents

```text
crates/frontend/foundation/
├── frontend_api_dtos/      `frontend_api_dtos`: the API wire types and the typed identifiers they hold
├── frontend_map_view/      `frontend_map_view`: the shared map seam: canvas sizing, engine creation, camera fit, pump, navigation, 2 m heights
├── frontend_offline/       `frontend_offline`: the offline worker registration, the offline pack download, its saved copies and the pack status
├── frontend_route_table/   `frontend_route_table`: every route's path, layout flags and access tier, their readers, the sidebar's menu
├── frontend_session/       `frontend_session`: the session store, the cross-tab session refresh, the sign-out hooks, the route guard and the content gates
├── frontend_test_support/  `frontend_test_support`: the repository-root finder and `golden!`, for tests only
├── frontend_transport/     `frontend_transport`: the HTTP client, the typed endpoint calls, the server-sent event streams
└── frontend_ui/            `frontend_ui`: the design-system primitives, the overlay stack, and the formatting, URL policy and clipboard helpers
```

## How it works

The app layout provides the two contexts everything else reads, `frontend_session`'s `AuthStore`
and the toast queue of `frontend_ui`, and spawns `frontend_session::session_refresh::bootstrap`,
which restores a stored session. From then on a page fetches through `frontend_transport` with the
store as its token provider, gates on `frontend_session`, and draws with `frontend_ui`. The app's
entry point registers the offline service worker through `frontend_offline` and mounts its pack
route watcher inside the router.

Every crate compiles natively, so its logic is tested with `cargo test -p <crate>` without a
browser; browser-only code carries its own `#[cfg(target_arch = "wasm32")]`, on its item or on
its `pub mod` line, and the browser crates sit in a wasm32 dependency table.

## Boundaries

- Depends on: external crates and the library crates under `crates/` outside `crates/frontend`
  (for example `offline_cache_policy` under `frontend_offline`, the rendering and streaming
  crates under `frontend_map_view`).
- Used by: the feature, page and workspace crates under `crates/frontend/` and the app
  (`crates/frontend/shell/frontend_application`): its entry point and shell; `frontend_test_support` only through
  `[dev-dependencies]`.
- Rules: a foundation crate depends only on foundation crates before it in the crate order of the
  frontend-layering law (`frontend_ui` < `frontend_api_dtos` < {`frontend_transport`,
  `frontend_route_table`} < `frontend_session` < {`frontend_offline`, `frontend_map_view`}, the
  braced crates peers), never on a feature, page or workspace crate or the app (`cargo xtask ci
  verify-workspace-laws`); `frontend_test_support` is reached only through dev-dependencies.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md#shared-foundations) — where these
  foundations sit among the routes, pages and workspaces they serve.
- [Frontend crates](/crates/frontend/README.md) — the layer order every frontend crate follows.
