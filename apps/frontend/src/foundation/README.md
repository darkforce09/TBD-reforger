# Shared foundations

The bottom layer of the single-page app: the building blocks every feature, page, workspace and
the shell share, with no business logic of their own — the transport to the
[API](/documentation/glossary/a_to_f.md#api) and its wire types, the route table and the navigation
menu, the session and its [role](/documentation/glossary/n_to_z.md#role) checks, the interface
primitives, small utilities, and the helpers the crate's own tests share.

## Contents

```text
apps/frontend/src/foundation/
├── auth/          the session store, its refresh, the sign-out hooks, the route guard and the gates
├── map_view/      the shared map seam: canvas, engine, camera fit, pump, navigation, 2 m heights
├── mod.rs         the module tree
├── offline/       the service worker registration and the offline pack download, with its state
├── route_table/   every route's path, layout flags and access tier, and the sidebar's menu
├── test_support/  the source scrubber, the captured API responses and the source pins, for tests
├── transport/     the HTTP client, typed endpoint calls, wire types, the role ladder, the live streams
├── ui/            the interface primitives: form controls, state classes, overlays, notices, layouts
└── utils/         timestamps, the countdown, the avatar sanitiser and the clipboard write
```

## How it works

The app layout provides the two contexts everything else reads, the `AuthStore` of `auth/` and
the toast queue of `ui/`, and spawns `auth::session_refresh::bootstrap`, which restores a stored
session. From then on a page fetches through `transport/` with the store as its token provider,
gates on `auth/`, and draws with `ui/` and `utils/`.

The modules import in one order, each only from those left of it: `ui/` < `utils/` <
`transport/` < `route_table/` < `auth/` < {`offline/`, `map_view/`}; `test_support/` serves the
test files of all of them.

| Module | Uses within `foundation` |
|---|---|
| `ui/` | none |
| `utils/` | `ui/`: the toast context and the placeholder avatar |
| `transport/` | none; the session reaches it as a `TokenProvider` it declares |
| `route_table/` | `transport/`: `dto::role` (`Role`, `has_min_role_authed`) |
| `auth/` | `transport/`: the wire types, `TokenProvider`, `SingleFlight`, the refresh policy and the request path; `route_table/`: the tiers and the redirect; `ui/`: the notice for a refused route |
| `offline/` | `transport/`: the rate-limit retry the saved-copy fetch sends through |
| `map_view/` | none |

`mod.rs` declares `auth/`, `map_view/`, `offline/`, `route_table/`, `transport/`, `ui/` and
`utils/` without a `cfg` gate, so the native build compiles all seven; browser-only code below them carries its own `#[cfg(target_arch = "wasm32")]`,
on its item or on its `pub mod` line, so `cargo test -p frontend` runs the logic of all of
it without a browser. `test_support/` compiles only in test builds.

Nothing here imports from `features`, `pages`, `workspaces` or `shell`. The one piece of work a
higher layer adds to the session is a sign-out hook: `main.rs` registers the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s `purge_local_documents`
with `auth::logout_hooks`, and `AuthStore::clear_session` runs it. The search box, select and
slider take their hover and disabled classes from `ui/tokens.rs`, which the Mission Creator's
chrome layout re-exports. The route guard in `auth/` reads the tiers of `route_table/`.

## Public surface

- `transport::client`: the request verbs, the refresh policy, `SingleFlight` and the failure
  types; `transport::dto`: every wire type, with `dto::role` (`Role`, `has_min_role`,
  `has_min_role_authed`); `transport::token_provider::TokenProvider`; `transport::endpoints`: the
  typed calls; `transport::sse`: the live status stream.
- `route_table`: `ROUTES`, `breadcrumb`, `full_bleed`, `chromeless`, `role_may_enter`,
  `auth_denial_redirect`, and `navigation_menu::NAVIGATION`; see its README.
- `auth`: `AuthStore`, `AuthGate`, `AdminGate`, the session types and their storage (`Session`,
  `PersistState`, `AUTH_PERSIST_KEY`, `persist`, `load_persisted`), `session_refresh`
  (`bootstrap`, `with_refresh_lock`) and `logout_hooks::register_logout_hook`.
- `ui`: `Dialog`, `Sheet`, `MaterialIcon`, `DEFAULT_AVATAR`, `PageHeader`, `cn`, `SearchBox`,
  `Select`, `Slider`, `badge_class`, `tokens` (`HOVER_FILL`, `DISABLED_GLYPH`), the `split_pane` layout, the
  `toast` context and viewport, and `modal_stack`, which the Mission Creator's overlays register
  with.
- `map_view`: the map seam (`handles::MapViewHandles`, `engine_mount`, `frame_pump`, `resize`,
  `navigation`, `mount::mount_map_view`, `camera_fit`, `terrain_height::TerrainHeights`,
  `terrain_preferences`); see its README.
- `offline`: `offline_status` and the `data-offline-state` / `data-offline-progress` attributes;
  `offline_optional_files` (`OptionalFiles`: whether the pack's one optional part, the
  cross-origin icon font, is cached) and the `data-offline-optional` attribute (`complete` or
  `missing` once a download finishes), which the mortar calculator's offline line reads to add
  its icon font notice; `service_worker_registration::register_at_boot` and
  `offline_pack::OfflinePackRouteWatcher`, which `main.rs` calls and mounts; see its README.
- `utils`: `countdown_label`, `safe_avatar_url`, the `datefmt` and `utc_timestamp` readers, and
  `clipboard::write_clipboard`.
- `test_support`: the scrubber, fixtures and pins, for the crate's tests.

## Boundaries

- Depends on:
  - `map_engine::data`, in the wire types; `map_engine` `frame`, `camera` and `streaming`, and
    `terrain_elevation`, in `map_view/`;
  - `offline_cache_policy` (cache names, request classes, the terrain pack list, the saved-copy
    header), in `offline/`;
  - `http_url_guard` (`is_http_url`), in `utils/`;
  - `leptos`, `leptos_router`, `gloo-net`, `gloo-timers`, `web-sys`, `js-sys`, `wasm-bindgen`,
    `wasm-bindgen-futures`, `futures`, `serde`, `serde_json`, `url` and `base64`.
- Used by: the shared features under `apps/frontend/src/features/`; the app frame under
  `apps/frontend/src/shell/` and `apps/frontend/src/main.rs`; the pages under
  `apps/frontend/src/pages/`; the Mission Creator under
  `apps/frontend/src/workspaces/editor/`. The debug benches under
  `apps/frontend/src/workspaces/debug/` use none of it.
- Rules: nothing here imports from a higher layer, and the modules keep the order above
  (`cargo xtask verify frontend-layering`); `mod.rs` declares every child ungated except
  `test_support`, which stays under `#[cfg(test)]` so no shipped code can reach it; every
  production file opens with a `//!` header, stays within 500 lines, documents every visible item
  and holds no inline test module (`frontend_production_files_meet_the_documentation_standard` in
  `apps/frontend/src/tests/doc_audit/mod.rs`).

## Related documentation

- [Frontend documentation](/documentation/apps/frontend/README.md#shared-foundations) — where these foundations
  sit among the routes, pages and workspaces they serve.
