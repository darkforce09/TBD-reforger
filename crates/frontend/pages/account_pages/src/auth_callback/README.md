# Sign-in callback page

The `/auth/callback` page that the sign-in redirect lands on: it reads the session the
[API](/documentation/glossary/a_to_f.md#api) put in the URL fragment, stores it, loads the viewer's
profile and sends the browser on to the dashboard. The Discord callback and the
[dev login](/documentation/glossary/a_to_f.md#dev-login) both redirect here.

## Contents

```text
crates/frontend/pages/account_pages/src/auth_callback/
├── mod.rs   the module tree; re-exports `AuthCallbackPage`
└── page.rs  `AuthCallbackPage`: scrubs the token fragment, installs the session, shows the outcome
```

## How it works

`AuthCallbackPage` does its work once, when it mounts, and only in the browser build; a native
build shows the "no sign-in details" failure.

```text
fragment ─┬─ error=<code> ──────────────────────▶ scrub ──▶ failure line for the code
          └─ access_token, refresh_token, expires_at
               ──▶ scrub ──▶ install tokens ──▶ persist them under the refresh lock
               ──▶ GET /api/v1/me ──▶ adopt the profile ──▶ persist it ──▶ full-page load of /
```

The fragment carries credentials, so the page replaces the history entry with the bare path as soon
as it has read it: a back button cannot return to the tokens, and a second parse would find
nothing. The frame in `crates/frontend/shell/frontend_application/src/shell/layout.rs` skips the
stored-session restore on this path, because the page installs the session it was handed. When the
profile fetch fails the stored tokens stay, so a reload can still restore the session.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/auth/callback` | `AuthCallbackPage` | route tier `none`; reachable signed out, since it runs before a session exists | bare: the frame renders no sidebar, top bar or wrapper; no breadcrumb |

## Data

- The URL fragment the API redirects with: `access_token`, `refresh_token`, `expires_at` and
  `arma_linked` on success, `error=<code>` on failure. The page parses `arma_linked` and ignores
  it; the profile says whether an Arma identity is linked.
- `GET /api/v1/me`: read as `MeResponse`, which the `AuthStore` adopts as the session's profile.
- The page writes the `AuthStore` from context and the persisted session (`persist`, then
  `persist_profile_if_current`), and ends with a full-page load of `/`.

## States

| State | What the viewer sees |
|---|---|
| completing | "Completing sign-in…" over "Establishing your session." |
| failed | "Sign-in failed", one of the reason lines below, and a "Back to login" link to `/login` |
| `error=missing_code` | "Discord did not return an authorization code. Please try again." |
| `error=invalid_state` | "The sign-in request expired or was tampered with. Please try again." |
| `error=discord_unreachable` | "Could not reach Discord. Please try again in a moment." |
| `error=banned` | "This account is banned from the platform." |
| `error=oauth_unconfigured` | "Discord sign-in is not configured on this server. Contact an administrator." |
| no tokens, or a profile the store refuses | "No sign-in details were found. Please start from the login page." |
| any other code (`server_error`, `oauth_host_mismatch`), or a failed profile fetch | "Something went wrong completing sign-in. Please try again." |
| tokens not storable | "This browser could not safely store the sign-in session. Reload using a supported browser over HTTPS." |
| profile not storable | "The sign-in session changed or could not be stored. Reload to continue." |
| signed in | nothing of its own: the browser loads `/` |

## Boundaries

- Depends on: `frontend_session` (`AuthStore`, `persist`,
  `session::persist_profile_if_current`, `session_refresh::with_refresh_lock`),
  `frontend_transport` (`api_get`, `MeResponse`, `RefreshResponse`), and the browser's location and history through
  `web_sys` and `js_sys`.
- Used by: the `/auth/callback` route in `crates/frontend/shell/frontend_application/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the frame in
  `crates/frontend/shell/frontend_application/src/shell/layout.rs`, which renders this path bare; the DOM
  oracle's `callback` capture in
  `tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs`; over redirects, the Discord
  callback and the dev login in `crates/api/api_identity_and_access/src/handlers/`.
- Rules: the fragment is scrubbed with a history replace, never a push; the path stays reachable
  signed out and stays named in the frame's `classify_frame` (`classify_frame_kinds` in
  `crates/frontend/shell/frontend_application/src/shell/tests/layout.rs`); an error code the page does not
  know falls back to the generic line.

## Related documentation

- [Account pages](/documentation/crates/frontend/pages/account_pages/account_pages.md) — the
  behaviour and design of the account pages.
- [Identity and access domain](/crates/api/api_identity_and_access/src/README.md) — the
  sign-in routes that redirect here.
- [Local development](/documentation/runbooks/local_development.md) — the dev login, which
  lands here with the same fragment as the Discord sign-in.
