# Account settings page

The `/settings` page: the signed-in viewer's profile and [role](/documentation/glossary/n_to_z.md#role),
the link between their Discord account and their Arma identity, and their attendance figures.

## Contents

```text
crates/frontend/pages/account_pages/src/settings/
├── mod.rs   the module tree; re-exports `SettingsPage`
└── page.rs  `SettingsPage`: both fetches, the link and unlink actions, and the three cards
```

## How it works

`SettingsPage` renders its body inside `AuthGate`. The body fetches the profile and the link status
at once and renders the cards only when both have settled, so the link line never reads "Unlinked"
because its fetch has not answered yet. The link-code signals live in `ArmaLinkCtx`, created above
the suspense boundary: a code generated in this session survives the refetch that follows it and
outranks the server's notice that a code is already pending. Generating a code refetches the link
status; unlinking clears the code and refetches both. Each action ignores a second click while its
request runs. The avatar goes through `safe_avatar_url`, which keeps an `http` or `https` URL and
puts the shipped placeholder in place of anything else. The Arma Identity card carries the id
`arma-link`, the target of the top bar's "Link Arma Identity" menu item.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/settings` | `SettingsPage` | route tier `none`; the cards render only for a signed-in viewer | padded inside the navigation frame; breadcrumb Account / Settings |

## Data

- `GET /api/v1/me`: read as `MeResponse`; the cards read its `user`: `username`, `discord_handle`,
  `role`, `avatar_url`, `total_deployments` and `attendance_rate`.
- `GET /api/v1/me/link/status`: read as `LinkStatus`: `linked`, `arma_character` (or `arma_id`
  when the character is empty) and `pending_code`.
- `POST /api/v1/me/link` with `{}`: read as `LinkCodeResponse`, whose `code` the card shows.
- `DELETE /api/v1/me/link`: unlinks the Arma identity.
- The page reads the `AuthStore` and the toast queue from context and writes nothing else. The
  requests run in the browser build only; the views that run them exist in that build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| profile failed | "Failed to load data." |
| loaded | the "Settings" header, "Account profile, Arma identity, and service statistics.", over the "Profile", "Arma Identity" and "Service Stats" cards; Service Stats shows "Total Operations" and "Attendance" as a percentage |
| linked | "Status: Linked (<character or Arma id>)", and "Unlink Arma ID" beside "Generate Link Code" |
| unlinked, or the link status failed | "Status: Unlinked" and "Generate Link Code" |
| code generated here | "Link code: <code>" |
| code pending on the server | "A link code is already pending. Generate a new one to display it, then enter it in-game." |
| action results | toasts "Link code generated — enter it in-game", "Failed to generate link code", "Arma identity unlinked" and "Failed to unlink" |

## Boundaries

- Depends on: `frontend_transport` (`api_get`, `api_post`, `api_delete`, and `MeResponse`,
  `LinkStatus` and `LinkCodeResponse` from `crates/frontend/foundation/frontend_api_dtos/src/auth.rs`),
  `frontend_ui` (`PageHeader`, `MaterialIcon`, the toast queue, `safe_avatar_url`),
  `frontend_session` (`AuthGate` and the `AuthStore` context).
- Used by: the `/settings` route in `crates/frontend/shell/frontend_application/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the top bar's account menu in
  `crates/frontend/shell/frontend_application/src/shell/top_nav.rs`, which links `/settings` and
  `/settings#arma-link`; the DOM oracle's `settings` capture in
  `tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs`.
- Rules: the cards wait for both fetches; the avatar source passes through `safe_avatar_url`
  (`topnav_avatar_src_only_keeps_http_urls` in
  `crates/frontend/shell/frontend_application/src/shell/tests/layout.rs`, over the cases in
  `crates/foundation/http_url_guard/src/cases.rs`); the Arma Identity card keeps the id `arma-link`
  that the top bar links to.

## Related documentation

- [Account pages](/documentation/crates/frontend/pages/account_pages/account_pages.md) — the
  behaviour and design of the account pages.
