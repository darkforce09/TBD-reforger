# Sign-in page

The `/login` page: the card a signed-out visitor lands on, whose one button hands the browser to
the Discord sign-in the [API](/documentation_v2/glossary.md#api) runs.

## Contents

```text
apps/website/frontend/src/v2/pages/account/login/
├── mod.rs   the module tree; re-exports `LoginPage`
└── page.rs  `LoginPage`: the sign-in card and the button that leaves for the Discord flow
```

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/login` | `LoginPage` | route tier `none`; reachable signed out | bare: the frame renders no sidebar, top bar or wrapper; no breadcrumb |

## Data

- `GET /api/v1/auth/discord/login`: never fetched. The button sets the browser's location to it,
  because the flow leaves the application for Discord and returns through `/auth/callback`.
- The page reads no context or storage and writes nothing.

## States

| State | What the viewer sees |
|---|---|
| any viewer | "TBD Reforger", "Sign in to register, deploy, and manage operations.", a "Sign in with Discord" button, and a "Continue browsing without signing in" link to `/` |

## Boundaries

- Depends on: `leptos`, and `web_sys` for the browser's location.
- Used by: the `/login` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sign-in links of `AuthGate` in
  `apps/website/frontend/src/v2/core/ui/gates.rs`, of the top bar in
  `apps/website/frontend/src/v2/pages/navigation/top_nav.rs` and of the sign-in callback's failure
  view; the DOM oracle's `login` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the flow starts with a full-page navigation, never a request, since it continues off-site;
  the path stays reachable signed out and stays named in the frame's `classify_frame`
  (`classify_frame_kinds` in `apps/website/frontend/src/v2/pages/navigation/tests/layout.rs`).

## Related documentation

- [Account pages](/documentation_v2/website/frontend/pages/account/account_pages.md) — the
  behaviour and design of the account pages.
- [Identity and access domain](/apps/website/api_v2/src/identity_and_access/README.md) — the
  Discord sign-in routes the button starts.
