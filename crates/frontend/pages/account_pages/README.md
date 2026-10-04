# Account pages

The `account_pages` crate: the pages of the single-page app that act on the viewer's own session
rather than on [mission](/documentation/glossary/g_to_m.md#mission) or
[operations](/documentation/glossary/n_to_z.md#operations) data — the sign-in page, the page the
Discord sign-in redirect lands on, and the account settings with the Arma identity link.

## Contents

```text
crates/frontend/pages/account_pages/
├── Cargo.toml  the package: `frontend_session`, `frontend_transport`, `frontend_api_dtos`, `frontend_ui`, `leptos`, layout tier 10
└── src/        the sign-in, callback and settings pages, with the settings page's avatar guard test
```

## How it works

The app's route table (`apps/frontend/src/app_routes.rs`) mounts the three route components:
`LoginPage` at `/login`, `AuthCallbackPage` at `/auth/callback` and `SettingsPage` at
`/settings`. The sign-in button sends the whole page to the
[API](/documentation/glossary/a_to_f.md#api)'s Discord sign-in, which redirects back to
`/auth/callback` with the session in the URL fragment; the callback page scrubs the fragment from
history, installs the session in the shared `AuthStore` and loads `/`. The settings page renders
inside the frame, behind `AuthGate`, and reads and changes the link between the viewer's Discord
account and their Arma identity. The [source tree README](src/README.md) walks through each page.

The three route components drive the browser's location or call endpoints that exist only in the
browser build, so they are compiled for `wasm32` alone; the settings page's avatar guard test runs
natively.

## Getting started

Run from the repository root:

```bash
cargo test -p account_pages   # the settings page's avatar guard over the URL guard's case table
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `login::LoginPage`, `auth_callback::AuthCallbackPage` and `settings::SettingsPage`: the route
  components (`wasm32`), each also reachable at its page module (`<page>::page::<Component>`), so
  the props type leptos derives for it is public.
- `prelude`: the three route components.

## Boundaries

- Depends on: `frontend_session` (the `AuthStore` session, its persistence, the refresh lock and
  the sign-in gate), `frontend_transport` (the API client), `frontend_api_dtos` (the profile, link
  and session token wire types), `frontend_ui` (the interface primitives and the avatar URL
  guard), `leptos`; on `wasm32`, `web-sys`, `js-sys`, `wasm-bindgen`, `futures` and `serde_json`;
  `http_url_guard` for its tests only.
- Used by: the single-page app (`apps/frontend`), whose route table mounts the three pages and
  whose frame renders `/login` and `/auth/callback` bare.
- Rules: a page crate depends on foundation and feature crates only, never on another page crate,
  a workspace or the app (`cargo xtask ci verify-workspace-laws`); `/login` and `/auth/callback`
  stay reachable signed out.

## Related documentation

- [Source tree](src/README.md) — the pages, their routes and the rules they keep.
- [Account pages documentation](/documentation/crates/frontend/pages/account_pages/README.md) —
  the feature doc of sign-in, the callback and settings.
