# Account settings page

The `/settings` page: the signed-in viewer's profile and [role](/documentation_v2/glossary.md#role),
the link between their Discord account and their Arma identity, and their attendance figures.

## Contents

```text
apps/website/frontend/src/v2/pages/account/settings/
├── mod.rs   the module tree; re-exports `SettingsPage`
├── page.rs  `SettingsPage`: both fetches, the link and unlink actions, and the three cards
└── tests/   unit tests for the profile avatar sink
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
  requests run in the browser build only; a native build shows the failure line.

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

- Depends on: `crate::v2::core::api` (`api_get`, `api_post`, `api_delete`, and `MeResponse`,
  `LinkStatus` and `LinkCodeResponse` from `apps/website/frontend/src/v2/core/api/dto/auth.rs`),
  `crate::v2::core::ui` (`AuthGate`, `PageHeader`, `MaterialIcon`, the toast queue),
  `crate::v2::core::utils::safe_avatar_url` and the `AuthStore` context.
- Used by: the `/settings` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the top bar's account menu in
  `apps/website/frontend/src/v2/pages/navigation/top_nav.rs`, which links `/settings` and
  `/settings#arma-link`; the DOM oracle's `settings` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the cards wait for both fetches; the avatar source passes through `safe_avatar_url`
  (`profile_avatar_src_only_keeps_http_urls` in `tests/settings.rs`, over the cases in
  `apps/website/shared/is_http_url_cases.rs`); the Arma Identity card keeps the id `arma-link`
  that the top bar links to.

## Related documentation

- [Account pages](/documentation_v2/website/frontend/pages/account/account_pages.md) — the
  behaviour and design of the account pages.
