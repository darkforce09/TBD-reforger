# Session and access

The browser side of signing in: the session the app holds as signals, the slice of it that
survives a reload, the [role](/documentation_v2/glossary/n_to_z.md#role) ladder every gate measures
against, the guard that keeps a viewer off a route their role does not clear, and the guard on
every link the app renders.

## Contents

```text
apps/website/frontend/src/v2/core/auth/
├── mod.rs                  the module tree; re-exports the store, the roles and the session types
├── refresh_transaction.rs  a spent credential leaves storage first and is replaced only on success
├── role.rs                 `Role`: the five-tier ladder, its wire spelling, the two comparisons
├── route_guard.rs          whether the viewer may stay on a route, and the redirecting effect
├── session.rs              the user, the token pair and the `tbd-auth` blob in local storage
├── session_identity.rs     the untrusted session id an access token names, for correlating tabs
├── session_restore.rs      the cold-start restore every request waits for
├── single_flight.rs        one in-flight refresh shared by the callers of a session generation
├── store.rs                `AuthStore`: the session as signals, its mutations and role questions
├── tests/                  unit tests for every module: store, roles, persistence, flights, guards
└── url_guard.rs            `is_http_url`: whether a string may be rendered as a link target
```

## How it works

The app layout provides one `AuthStore`, which installs the route guard and starts out
bootstrapping; the [API](/documentation_v2/glossary/a_to_f.md#api) client's `bootstrap` restores the
`tbd-auth` blob under the cross-tab refresh lock and fetches the profile. The auth callback page
adopts a new token pair with `set_tokens`; `clear_session`, which the top bar's sign-out and a
failed refresh call, purges the departing account's local
[mission](/documentation_v2/glossary/g_to_m.md#mission) drafts through the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s `purge_local_documents`.

The store's fields are `RwSignal`s, so it is `Copy`. The blob keeps the refresh token, the user,
the expiry and the session id, never the access token, and its key names are fixed: renaming one
signs out every stored session. Each session start or end advances the session generation, and a
request, refresh or profile answer captured under an older one is discarded when it lands;
`SingleFlight::run_keyed` makes one generation's callers share one refresh, since the API rotates
the refresh token on every call and a second spend ends the session.

`Role` runs `Guest` < `Enlisted` < `Leader` < `MissionMaker` < `Admin`, in snake_case on the wire
and in the route table's `auth` strings. `has_min_role` lets a viewer with no role clear every
tier, so a signed-out visitor browses the full navigation, and is for chrome only;
`has_min_role_authed` never admits one, for actions and protected routes. The route guard
waits out bootstrapping, then asks `crate::router::role_may_enter` and navigates to
`crate::router::auth_denial_redirect` with the notice "Mission Maker role required to open the
editor."; an administrator route has no redirect, and its page renders the `AdminGate` refusal.
`is_http_url` admits only an absolute `http` or `https` URL with a host and no control characters
or surrounding whitespace, the same rule as the API's
`apps/website/api_v2/src/core/text/http_url_guard.rs`.

## Boundaries

- Depends on: `crate::router` (`apps/website/frontend/src/router.rs`), for the tiers and
  redirects; `crate::v2::core::api`, for `dto::MeResponse` and the client's refresh lock; the
  toast context of `crate::v2::core::ui`; the Mission Creator's
  `crate::v2::apps::editor::shell::hydrate::purge_local_documents`, which `store.rs` calls to
  purge a departing account's drafts; `leptos`, `leptos_router`, `futures`, `url`, `base64`,
  `serde`, `serde_json`, and `web-sys` for local storage.
- Used by: `apps/website/frontend/src/router.rs`; the API layer, the content gates and the avatar
  sanitiser in `apps/website/frontend/src/v2/core/`; the app layout, the top bar and the auth
  callback page under `apps/website/frontend/src/v2/pages/`, and every page and Mission Creator
  panel under `apps/website/frontend/src/v2/` that reads the session, gates on a role or renders
  a stored link.
- Rules: the access token is never persisted (`persist_blob_shape_matches_tbd_auth` in
  `tests/auth.rs`); an older generation never clears a newer flight
  (`older_generation_completion_cannot_clear_newer_pending_flight` in `tests/single_flight.rs`),
  and a profile answer never crosses an account switch
  (`profile_response_cannot_cross_an_account_switch` in `tests/store.rs`); `is_http_url` agrees
  with the API's copy on every case of `apps/website/shared/is_http_url_cases.rs`
  (`matches_the_api_guard_on_every_shared_case` in `tests/url_guard.rs`).

## Related documentation

- [Account pages](/documentation_v2/website/frontend/pages/account/account_pages.md) — sign-in,
  the auth callback and settings.
- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — sessions, refresh rotation and replay revocation in the API, and the browser's generations.
- [Frontend documentation](/documentation_v2/website/frontend/README.md#route-table) — the route table
  whose access tiers the route guard enforces.
