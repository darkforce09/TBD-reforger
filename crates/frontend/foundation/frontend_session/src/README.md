# Session and access

The source tree of `frontend_session`, the browser side of signing in: the session the app holds as signals, the slice of it that
survives a reload, the refresh that keeps it alive across tabs, the hooks that run when it ends,
the guard that keeps a viewer off a route their [role](/documentation/glossary/n_to_z.md#role)
does not clear, and the two content gates. The guard on every link the app renders is the
`http_url_guard` crate (`crates/foundation/http_url_guard/`).

## Contents

```text
crates/frontend/foundation/frontend_session/src/
├── gates.rs                `AuthGate` and `AdminGate`: render a subtree only to a viewer who may see it
├── logout_hooks.rs         the sign-out hooks a higher layer registers, run with the departing id
├── lib.rs                  the module tree; re-exports the store, the gates, the guard and the session types
├── prelude.rs              `AuthStore`, `AuthGate` and `AdminGate`
├── refresh_transaction.rs  a spent credential leaves storage first and is replaced only on success
├── route_guard.rs          whether the viewer may stay on a route, and the redirecting effect
├── session.rs              the minted session and the `tbd-auth` blob in local storage
├── session_identity.rs     the untrusted session id an access token names, for correlating tabs
├── session_refresh.rs      the per-tab refresh cell, the cross-tab lock, peer rotation, cold start
├── session_restore.rs      the cold-start restore every request waits for
├── store.rs                `AuthStore`: the session as signals, its mutations and role questions
└── tests/                  unit tests: store, gates, guard, persistence, sign-out hooks
```

## How it works

The app layout provides one `AuthStore`, which installs the route guard and starts out
bootstrapping; `session_refresh::bootstrap` restores the `tbd-auth` blob under the cross-tab
refresh lock and fetches the profile. The store is the transport's `TokenProvider`: every
[API](/documentation/glossary/a_to_f.md#api) request reads its access token and generation, and
refreshes through the one per-tab `SingleFlight` cell and the Web Lock in `session_refresh.rs`,
which re-reads the stored refresh token inside the lock, adopts a peer tab's broadcast rotation
instead of spending a second token, and persists the successor before releasing. The auth
callback page adopts a new token pair with `set_tokens`; `clear_session`, which the top bar's
sign-out and a failed refresh call, runs the sign-out hooks with the departing account's id. The
app root registers the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
`purge_local_documents` there at start, so a sign-out deletes that account's local
[mission](/documentation/glossary/g_to_m.md#mission) drafts even in a tab where the editor never
mounted; a blank or absent id runs no hook.

The store's fields are `RwSignal`s, so it is `Copy`. The blob keeps the refresh token, the user,
the expiry and the session id, never the access token, and its key names are fixed: renaming one
signs out every stored session. Each session start or end advances the session generation, and a
request, refresh or profile answer captured under an older one is discarded when it lands;
`SingleFlight::run_keyed` makes one generation's callers share one refresh, since the API rotates
the refresh token on every call and a second spend ends the session.

The role ladder, `User` and the rotated pair are the transport's wire types
(`frontend_api_dtos`). The route guard waits out bootstrapping, then asks
`frontend_route_table::role_may_enter` and navigates to
`frontend_route_table::auth_denial_redirect` with the notice "Mission Maker role required to open the
editor."; an administrator route has no redirect, and its page renders the `AdminGate` refusal.

The gates derive what they show from the store through a memo:

| Viewer | `AuthGate` shows | `AdminGate` shows |
|---|---|---|
| session still restoring | "Loading session…" | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` | the same as `AuthGate` |
| signed in below the admin tier | its children | "Admin access required." |
| signed in as an administrator | its children | its children |

Each gate rebuilds its children only when that answer changes, so a token rotation or a profile
poll never remounts the page under it.

## Boundaries

- Depends on: `frontend_route_table` (`crates/frontend/foundation/frontend_route_table/src/routes.rs`),
  for the tiers and redirects; `frontend_api_dtos`, for the wire types (`role`, `User`,
  `MeResponse`, `RefreshResponse`, the session identifier); `frontend_transport`, for the
  `TokenProvider` the store implements, the `SingleFlight` cell type, the refresh policy and the
  request path the refresh and the profile read go through; the toast context of `frontend_ui`;
  `leptos`, `futures`, `base64`, `serde`, `serde_json`; and, in the wasm32 build only,
  `leptos_router`, `gloo-net`, `gloo-timers`, `web-sys`, `js-sys`, `wasm-bindgen` and
  `wasm-bindgen-futures` for the router hooks, local storage, Web Locks, the broadcast channel
  and the refresh request. Nothing above the foundation: the layers above reach the session's end
  only through the hooks they register.
- Used by: `crates/frontend/shell/frontend_application/src/main.rs`, which registers the sign-out hook; the app layout and the
  top bar under `crates/frontend/shell/frontend_application/src/shell/`; the auth callback page in
  `crates/frontend/pages/account_pages/`, and every feature, page and Mission Creator panel under
  `crates/frontend/` that reads the session, gates on a role or wraps its body in a content
  gate.
- Rules: the access token is never persisted (`persist_blob_shape_matches_tbd_auth` in
  `tests/auth.rs`); the refresh request is reachable only inside the cross-tab lock; a profile answer never crosses an account switch
  (`profile_response_cannot_cross_an_account_switch` in `tests/store.rs`); a sign-out hook
  receives the departing id and a blank id runs none
  (`a_registered_hook_receives_the_departing_account_id` and `a_blank_departing_id_runs_no_hook`
  in `tests/logout_hooks.rs`); a gate waits out the session restore and rebuilds only when its
  admission changes (`the_sign_in_gate_waits_out_the_session_restore` and
  `the_admin_gate_rebuilds_its_children_only_when_the_role_crosses_the_tier` in
  `tests/gates.rs`).

## Related documentation

- [Account pages](/documentation/crates/frontend/pages/account_pages/account_pages.md) — sign-in,
  the auth callback and settings.
- [Identity transactions](/documentation/crates/api/api_server/design_notes/identity_transactions.md)
  — sessions, refresh rotation and replay revocation in the API, and the browser's generations.
- [Frontend session](../README.md) — the crate: its targets, its public surface and how to test
  it.
- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md#route-table) — the route table
  whose access tiers the route guard enforces.
