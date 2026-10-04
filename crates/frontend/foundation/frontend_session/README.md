# Frontend session

The `frontend_session` crate: the browser side of signing in. It holds the session as signals in
`AuthStore`, persists the slice of it that survives a reload, refreshes the single-use token pair
once across every tab, runs the sign-out hooks a higher layer registers, keeps a viewer off a
route their [role](/documentation/glossary/n_to_z.md#role) does not clear, and gates page bodies on
being signed in or being an administrator.

## Contents

```text
crates/frontend/foundation/frontend_session/
├── Cargo.toml  the package: the four lower foundation crates, `leptos`, the router and browser crates for wasm32, layout tier 9
└── src/        the store, the persisted slice, the session refresh, the sign-out hooks, the route guard and the gates
```

## How it works

The app layout provides one `AuthStore` and spawns `session_refresh::bootstrap`, which restores
the stored session. The store implements `frontend_transport`'s `TokenProvider`, so every request
the transport sends reads the store's access token and refreshes it through the one per-tab
single flight and the cross-tab Web Lock; the transport never names the session. A session that
ends runs every registered sign-out hook with the departing account's id: the app registers the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s local draft purge there,
so the session crate depends on nothing above the foundation. The
[source tree README](src/README.md) walks through the refresh, the persisted blob, the route guard
and what each gate shows.

The code that calls the browser (local storage, the Web Lock and broadcast channel of the session
refresh, the router hooks of the route guard) is gated to the wasm32 build. The store, the
persisted slice's serialisation, the session identity, the refresh transaction, the route guard's
decision and the two gates compile on every target, so their tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_session   # the store, the gates, the guard, persistence, the sign-out hooks, the source pins
```

## Configuration

None: no feature, no environment variable. The persisted slice lives under the local storage key
`tbd-auth` (`session::AUTH_PERSIST_KEY`), the cross-tab lock and broadcast channel are both named
`tbd-auth-refresh` (`session_refresh::REFRESH_LOCK_NAME`), and the refresh goes to the same-origin
`/api/v1/auth/refresh`.

## Public surface

- `store::AuthStore`: the session as signals, its mutations (`set_tokens`, `clear_session`, the
  profile adoption) and its role questions; re-exported at the crate root.
- `gates`: `AuthGate` and `AdminGate`, re-exported at the crate root.
- `session`: `PersistState`, `PersistedAuth`, `AUTH_PERSIST_KEY`, the blob's serialisation, and
  in the wasm32 build `persist`, `load_persisted`, `clear_persisted` and
  `persist_profile_if_current`.
- `session_refresh` (wasm32): `bootstrap`, `with_refresh_lock` (every read-modify-write of the
  stored credential runs inside it, including the app shell's sign-out and the sign-in callback's
  install) and `REFRESH_LOCK_NAME`; the `TokenProvider` implementation of `AuthStore`.
- `logout_hooks::register_logout_hook`: a higher layer's hook, run with the departing account's
  id when a session ends.
- `route_guard::route_auth_redirect`, `session_identity`, `session_restore`,
  `refresh_transaction`.
- `prelude`: `AuthStore`, `AuthGate` and `AdminGate`.

## Boundaries

- Depends on: `frontend_api_dtos`, `frontend_route_table`, `frontend_transport`, `frontend_ui`,
  `leptos`, `base64`, `futures`, `serde`, `serde_json`; `leptos_router`, `gloo-net`,
  `gloo-timers`, `web-sys`, `js-sys`, `wasm-bindgen` and `wasm-bindgen-futures` in the wasm32 build
  only.
- Used by: the single-page app (`apps/frontend`): its entry point, which registers the sign-out
  hook; the app shell; the shared features; the pages; the Mission Creator.
- Rules: the access token is never persisted; the refresh request is reachable only inside the
  cross-tab lock; the crate depends on no frontend crate above the foundation crates it names
  (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Account pages](/documentation/crates/frontend/pages/account_pages/account_pages.md) — sign-in, the auth
  callback and settings.
- [Identity transactions](/documentation/apps/api/verification_evidence/identity_transactions.md)
  — sessions, refresh rotation and replay revocation in the API.
- [Frontend documentation](/documentation/apps/frontend/README.md#shared-foundations) — the shared
  foundations among the routes, pages and workspaces of the app.
