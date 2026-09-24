# Account pages

The pages that act on the viewer's own session rather than on
[mission](/documentation_v2/glossary.md#mission) or
[operations](/documentation_v2/glossary.md#operations) data: sign-in, the callback the sign-in
redirect lands on, and the account settings.

## Contents

```text
apps/website/frontend/src/v2/pages/account/
├── auth_callback/  the `/auth/callback` page that installs the session the sign-in redirect carries
├── login/          the `/login` page that starts the Discord sign-in
├── mod.rs          the module tree; declares the three page modules
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

## Public surface

- `login::LoginPage`, `auth_callback::AuthCallbackPage` and `settings::SettingsPage`: the route
  components `apps/website/frontend/src/app_routes.rs` binds to `/login`, `/auth/callback` and
  `/settings`.
- The `arma-link` anchor on `/settings`, which the top bar's "Link Arma Identity" menu item
  targets.

## Boundaries

- Depends on: `crate::v2::core::auth` (the `AuthStore` and session persistence),
  `crate::v2::core::api` (the request client and the `dto/auth.rs` shapes) and
  `crate::v2::core::ui` (`AuthGate`, the page header, the toast queue); over HTTP, the
  [identity and access](/documentation_v2/glossary.md#identity-and-access) routes of the
  [API](/documentation_v2/glossary.md#api).
- Used by: the route table in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the frame in
  `apps/website/frontend/src/v2/pages/navigation/`, which renders `/login` and `/auth/callback`
  bare and links `/login` and `/settings` from its top bar.
- Rules: `/login` and `/auth/callback` stay reachable signed out and stay named in the frame's
  `classify_frame` (`classify_frame_kinds` in
  `apps/website/frontend/src/v2/pages/navigation/tests/layout.rs`); the callback scrubs the token
  fragment with a history replace before it installs the session.

## Related documentation

- [Account pages](/documentation_v2/website/frontend/pages/account/account_pages.md) — the
  behaviour and design of sign-in, the callback and settings.
- [Identity and access domain](/apps/website/api_v2/src/identity_and_access/README.md) — the API
  routes behind these pages.
