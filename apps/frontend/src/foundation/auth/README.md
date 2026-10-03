# Session and access

The browser side of signing in: the session the app holds as signals, the slice of it that
survives a reload, the refresh that keeps it alive across tabs, the hooks that run when it ends,
the guard that keeps a viewer off a route their [role](/documentation/glossary/n_to_z.md#role)
does not clear, and the two content gates. The guard on every link the app renders is the
`http_url_guard` crate (`crates/foundation/http_url_guard/`).

## Contents

```text
apps/frontend/src/foundation/auth/
├── gates.rs                `AuthGate` and `AdminGate`: render a subtree only to a viewer who may see it
├── logout_hooks.rs         the sign-out hooks a higher layer registers, run with the departing id
├── mod.rs                  the module tree; re-exports the store, the gates and the session types
├── refresh_transaction.rs  a spent credential leaves storage first and is replaced only on success
├── route_guard.rs          whether the viewer may stay on a route, and the redirecting effect
├── session.rs              the minted session and the `tbd-auth` blob in local storage
├── session_identity.rs     the untrusted session id an access token names, for correlating tabs
├── session_refresh.rs      the per-tab refresh cell, the cross-tab lock, peer rotation, cold start
├── session_restore.rs      the cold-start restore every request waits for
├── store.rs                `AuthStore`: the session as signals, its mutations and role questions
└── tests/                  unit tests: store, gates, guard, persistence, sign-out hooks, source pins
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
(`crate::foundation::transport::dto`). The route guard waits out bootstrapping, then asks
`crate::foundation::route_table::role_may_enter` and navigates to
`crate::foundation::route_table::auth_denial_redirect` with the notice "Mission Maker role required to open the
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

- Depends on: `crate::foundation::route_table` (`apps/frontend/src/foundation/route_table/mod.rs`), for the tiers and
  redirects; `crate::foundation::transport`, for the wire types (`dto::role`, `dto::User`,
  `dto::MeResponse`, `dto::RefreshResponse`), the `TokenProvider` the store implements, the
  `SingleFlight` cell type, the refresh policy and the request path the refresh and the profile
  read go through; the toast context of `crate::foundation::ui`; `leptos`,
  `leptos_router`, `futures`, `base64`, `serde`, `serde_json`, and `web-sys`, `js-sys` and
  `wasm-bindgen` for local storage, Web Locks and the broadcast channel. Nothing above the
  foundation: the layers above reach the session's end only through the hooks they register.
- Used by: `apps/frontend/src/main.rs`, which registers the sign-out hook; no other foundation
  module (the session is the top of the foundation's order below offline and the map view); the
  app layout and the top bar under
  `apps/frontend/src/shell/`; the auth callback page under `apps/frontend/src/pages/`, and every
  feature, page and Mission Creator panel under `apps/frontend/src/` that reads the session,
  gates on a role or wraps its body in a content gate.
- Rules: the access token is never persisted (`persist_blob_shape_matches_tbd_auth` in
  `tests/auth.rs`); the refresh request is reachable only inside the cross-tab lock
  (`the_refresh_post_is_reachable_only_from_inside_the_cross_tab_lock` in
  `tests/session_refresh.rs`); a profile answer never crosses an account switch
  (`profile_response_cannot_cross_an_account_switch` in `tests/store.rs`); a sign-out hook
  receives the departing id and a blank id runs none
  (`a_registered_hook_receives_the_departing_account_id` and `a_blank_departing_id_runs_no_hook`
  in `tests/logout_hooks.rs`); a gate waits out the session restore and rebuilds only when its
  admission changes (`the_sign_in_gate_waits_out_the_session_restore` and
  `the_admin_gate_rebuilds_its_children_only_when_the_role_crosses_the_tier` in
  `tests/gates.rs`).

## Related documentation

- [Account pages](/documentation/apps/frontend/pages/account/account_pages.md) — sign-in,
  the auth callback and settings.
- [Identity transactions](/documentation/apps/api/verification_evidence/identity_transactions.md)
  — sessions, refresh rotation and replay revocation in the API, and the browser's generations.
- [Frontend documentation](/documentation/apps/frontend/README.md#route-table) — the route table
  whose access tiers the route guard enforces.
