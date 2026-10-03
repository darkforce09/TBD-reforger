# Sign-in page

The `/login` page: the card a signed-out visitor lands on, whose one button hands the browser to
the Discord sign-in the [API](/documentation/glossary/a_to_f.md#api) runs.

## Contents

```text
apps/frontend/src/pages/account/login/
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
- Used by: the `/login` route in `apps/frontend/src/app_routes.rs` and
  `apps/frontend/src/foundation/route_table/mod.rs`; the sign-in links of `AuthGate` in
  `apps/frontend/src/foundation/auth/gates.rs`, of the top bar in
  `apps/frontend/src/shell/top_nav.rs` and of the sign-in callback's failure
  view; the DOM oracle's `login` capture in
  `tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs`.
- Rules: the flow starts with a full-page navigation, never a request, since it continues off-site;
  the path stays reachable signed out and stays named in the frame's `classify_frame`
  (`classify_frame_kinds` in `apps/frontend/src/shell/tests/layout.rs`).

## Related documentation

- [Account pages](/documentation/apps/frontend/pages/account/account_pages.md) — the
  behaviour and design of the account pages.
- [Identity and access domain](/crates/api/api_identity_and_access/src/README.md) — the
  Discord sign-in routes the button starts.
