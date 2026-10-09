# Account pages source

The source tree of `account_pages`: the pages that act on the viewer's own session rather than on
[mission](/documentation/glossary/g_to_m.md#mission) or
[operations](/documentation/glossary/n_to_z.md#operations) data: sign-in, the callback the sign-in
redirect lands on, and the account settings.

## Contents

```text
crates/frontend/pages/account_pages/src/
├── auth_callback/  the `/auth/callback` page that installs the session the sign-in redirect carries
├── lib.rs          the crate root: the module tree
├── login/          the `/login` page that starts the Discord sign-in
├── prelude.rs      the three route components the app's route table mounts
└── settings/       the `/settings` page: profile, Arma identity link and attendance figures
```

## How it works

A sign-in passes through the first two pages:

```text
/login ──(full-page load)──▶ GET /api/v1/auth/discord/login ──▶ Discord
       ──▶ GET /api/v1/auth/discord/callback ──(302, tokens in the fragment)──▶ /auth/callback
       ──▶ tokens and profile stored ──▶ full-page load of /
```

The sign-in and callback pages render bare, outside the navigation frame, and stay reachable
signed out; the callback runs before any session exists. The settings page renders inside the
frame, behind `AuthGate`, and reads and changes the link between the viewer's Discord account and
their Arma identity. All three read or write the one `AuthStore` the frame provides, and none
imports another.

All three route components drive the browser's location or call endpoints that exist only in the
browser build, so they are compiled for `wasm32` only; the settings page's avatar guard test runs
natively.

## Public surface

- `login::LoginPage`, `auth_callback::AuthCallbackPage` and `settings::SettingsPage` (`wasm32`):
  the route components `crates/frontend/shell/frontend_application/src/app_routes.rs` binds to `/login`, `/auth/callback` and
  `/settings`, also in `prelude`; each is reachable at its page module too
  (`<page>::page::<Component>`), so the props type leptos derives for it is public.
- The `arma-link` anchor on `/settings`, which the top bar's "Link Arma Identity" menu item
  targets.

## Boundaries

- Depends on: `frontend_session` (the `AuthStore`, session persistence, the refresh lock and
  `AuthGate`), `frontend_transport` (the request client), `frontend_api_dtos` (the shapes in
  `crates/frontend/foundation/frontend_api_dtos/src/auth.rs`) and `frontend_ui` (the page header,
  the icons, the toast queue and the avatar URL guard); over HTTP, the
  [identity and access](/documentation/glossary/g_to_m.md#identity-and-access) routes of the
  [API](/documentation/glossary/a_to_f.md#api).
- Used by: the route table in `crates/frontend/shell/frontend_application/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the frame in
  `crates/frontend/shell/frontend_application/src/shell/`, which renders `/login` and `/auth/callback`
  bare and links `/login` and `/settings` from its top bar.
- Rules: `/login` and `/auth/callback` stay reachable signed out and stay named in the frame's
  `classify_frame` (`classify_frame_kinds` in
  `crates/frontend/shell/frontend_application/src/shell/tests/layout.rs`); the callback scrubs the token
  fragment with a history replace before it installs the session.

## Related documentation

- [Account pages](/documentation/crates/frontend/pages/account_pages/account_pages.md) — the
  behaviour and design of sign-in, the callback and settings.
- [Identity and access domain](/crates/api/api_identity_and_access/src/README.md) — the API
  routes behind these pages.
- [App layout and navigation](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md)
  — the frame that renders sign-in bare and the account menu that links settings.
- [Local development](/documentation/runbooks/local_development.md) — the dev login and the
  Discord sign-in on a workstation.
