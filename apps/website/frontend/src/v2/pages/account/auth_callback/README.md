# Sign-in callback page

The `/auth/callback` page that the sign-in redirect lands on: it reads the session the
[API](/documentation_v2/glossary/a_to_f.md#api) put in the URL fragment, stores it, loads the viewer's
profile and sends the browser on to the dashboard. The Discord callback and the
[dev login](/documentation_v2/glossary/a_to_f.md#dev-login) both redirect here.

## Contents

```text
apps/website/frontend/src/v2/pages/account/auth_callback/
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
nothing. The frame in `apps/website/frontend/src/v2/pages/navigation/layout.rs` skips the
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

- Depends on: `crate::v2::core::auth` (`AuthStore`, `RefreshResponse`, `persist`,
  `session::persist_profile_if_current`), `crate::v2::core::api` (`api_get`,
  `refresh::with_refresh_lock`, `MeResponse`), and the browser's location and history through
  `web_sys` and `js_sys`.
- Used by: the `/auth/callback` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the frame in
  `apps/website/frontend/src/v2/pages/navigation/layout.rs`, which renders this path bare; the DOM
  oracle's `callback` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`; over redirects, the Discord
  callback and the dev login in `apps/website/api_v2/src/identity_and_access/handlers/`.
- Rules: the fragment is scrubbed with a history replace, never a push; the path stays reachable
  signed out and stays named in the frame's `classify_frame` (`classify_frame_kinds` in
  `apps/website/frontend/src/v2/pages/navigation/tests/layout.rs`); an error code the page does not
  know falls back to the generic line.

## Related documentation

- [Account pages](/documentation_v2/website/frontend/pages/account/account_pages.md) — the
  behaviour and design of the account pages.
- [Identity and access domain](/apps/website/api_v2/src/identity_and_access/README.md) — the
  sign-in routes that redirect here.
- [Local development](/documentation_v2/runbooks/local_development.md) — the dev login, which
  lands here with the same fragment as the Discord sign-in.
