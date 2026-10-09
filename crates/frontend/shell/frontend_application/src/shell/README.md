# App shell

The top layer of the single-page app, the frame every page renders in: the sidebar and the links
it shows, the top bar with the
breadcrumb and the account menu, the membership status banner, and the page shown when no route
matches. It is mounted once at the application root and chooses, from the path, which of three
frames a route gets.

## Contents

```text
crates/frontend/shell/frontend_application/src/shell/
├── layout.rs             `AppLayout`: the root frame, its three shapes and the active-link rule
├── membership_status.rs  `MembershipStatus`: membership banner, profile poll and access extension
├── mod.rs                the module tree: one `pub mod` per frame file, the wasm-only ones gated
├── not_found.rs          `NotFoundPage`: the router's fallback for a path no route matches
├── sidebar.rs            the permanent sidebar, brand block, link list and drawer toggle
├── tests/                unit tests: the active-link rule, frame kinds, account badge, route mounts
└── top_nav.rs            `TopNav`: the breadcrumb, the identity pill, the account menu and sign-out
```

## How it works

`start_app` in `crates/frontend/shell/frontend_application/src/main.rs`, the browser build's `#[wasm_bindgen(start)]`
function, mounts `AppLayout` once, inside the router. The mount chain is compiled for `wasm32`
only: `crate::app_routes`, `AppLayout`, `TopNav` and the `sidebar`, `membership_status` and
`not_found` modules, each gate commented with the user that needs it. A native build mounts
nothing and keeps the pure half of the frame, `classify_frame`, `FrameKind`, `is_active` and the
account badge of `top_nav.rs`, which the unit tests in `tests/` exercise. `AppLayout` creates
the one `AuthStore` every page reads, whose constructor installs the route guard in the browser
build, and the toast queue. It starts the stored-session restore, `bootstrap` in
`crates/frontend/foundation/frontend_transport/src/client/requests.rs`, on every path but `/auth/callback`,
whose page installs the session it was handed. `classify_frame` picks the frame from the pathname,
and a memo over it remounts the frame only when the kind changes:

```text
/login, /auth/callback      ──▶ bare: the page alone, no wrapper
route_table::chromeless     ──▶ chromeless: one full-viewport container
any other path              ──▶ chrome: sidebar + top bar + <main>
                                <main> overflow-hidden if route_table::full_bleed, else padded
```

The chromeless paths are the four rows `crates/frontend/foundation/frontend_route_table/src/routes.rs` flags: the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the review workspace and the two
debug benches. Moving between two chromed routes swaps only the page inside `<main>`. The
membership banner and the toast viewport sit beside the frame, so a frame swap never unmounts them.

The sidebar renders `NAVIGATION` top to bottom through the browse-mode check `has_min_role`: a
signed-out visitor, whose [role](/documentation/glossary/n_to_z.md#role) is unknown, sees every
section, the "Administration" section included; a signed-in viewer sees the links their role
clears, so "Administration" shows only for `admin`, and `guest` sees no link, since every other one
asks for `enlisted`. `is_active` marks one link: `/` only on `/`, any other link on its own path and
every path below it. On a narrow viewport the sidebar gives way to a toggle that opens the same
brand and links in a drawer, which a backdrop click, a link click or Escape closes. The sections and
their links, as `navigation_menu.rs` in `crates/frontend/foundation/frontend_route_table/src/` labels
them:

| Section | Links |
|---|---|
| "Command Center" | "Dashboard" `/`, "Server Intel" `/server-intel`, "Announcements" `/announcements` |
| "Operations" | "Event Schedule" `/events`, "My Deployments" `/deployments`, "Global Leaderboards" `/leaderboards` |
| "Mission Hub" | "Mission Library" `/missions` |
| "Field Tools" | "Mortar Calculator" `/tools/mortar` |
| "Doctrine & Info" | "SOPs & Manuals" `/wiki`, "Vehicle Database" `/vehicles`, "Modpacks" `/modpacks` |
| "Administration" | "Event Manager" `/admin/events`, "Mission Approvals" `/admin/approvals`, "Server Control" `/admin/server`, "Personnel Roster" `/admin/personnel`, "Comms Broadcaster" `/admin/content`, "Audit Logs" `/admin/audit`, "Ballistics Catalogs" `/admin/ballistics-catalogs` |

The top bar shows the route's breadcrumb from `route_table::breadcrumb`. Its account area renders from a
memo of the name, avatar and linked Arma identity, so a profile poll that changes none of them
leaves an open menu alone. Signing out clears the local session at once, removes the stored session
under the refresh lock when it belongs to this session, then asks the
[API](/documentation/glossary/a_to_f.md#api) to revoke the refresh token. `MembershipStatus` refetches
the profile every 30 seconds while a session is held, which keeps the membership flags current, and
shows its banner while the membership is stale or the profile says the viewer may extend access.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `*` (no route matches) | `NotFoundPage` | route tier `none` | padded inside the navigation frame; no breadcrumb, so the top bar reads "TBD Reforger" |

## Data

- `POST /api/v1/auth/logout` with the session's `refresh_token`: sent by "Sign Out" after the local
  session is cleared.
- `GET /api/v1/me`: read as `MeResponse` every 30 seconds while a session is held, and after an
  access extension; the `AuthStore` adopts it.
- `POST /api/v1/admin/users/{discordId}/membership-grace` with `reason` and `duration_hours: 48`:
  extends cached access for the entered account, or for the viewer when the field is blank.
- The frame provides the `AuthStore` and the toast queue as context, and reads from the store the
  viewer's `user` (name, avatar, `arma_id`, role) and the `membership_stale`,
  `membership_override_active` and `can_manage_membership_override` flags.

## States

| State | What the viewer sees |
|---|---|
| no route matches | "404", "Sector Not Found", "The requested route does not exist in this AO." and a "Return to Dashboard" link to `/` |
| signed out | a "Sign in with Discord" link to `/login` in the top bar; every sidebar section |
| signed in | "Linked: <first 8 characters of the Arma id>..." or "Unlinked", then the avatar and name, which open "Settings", "Link Arma Identity" (to `/settings#arma-link`) and "Sign Out" |
| sign-out trouble | toasts "Signed out locally, but this browser could not clear the stored session", or the API's message or "Signed out locally, but server session revocation failed" |
| narrow viewport | the toggle labelled "Open menu" in place of the sidebar |
| banner, membership stale | "Discord verification is delayed. We are using your last verified permissions during the grace period. Older snapshots retain Guest access." |
| banner, extension active | "An administrative access extension is active." |
| banner, viewer may extend access | "Extend cached access", with "Discord account ID (blank for yourself)", "Reason" and "Extend for 48 hours", disabled until a reason is typed |
| extension result | "Cached access extended for 48 hours. The reason is recorded in the audit log.", "Enter a Discord account ID using digits only.", or the API's message or "Access extension failed" |

## Boundaries

- Depends on:
  - `crate::app_routes::AppRoutes`, the routes the frame renders, and `frontend_route_table`
    (`chromeless`, `full_bleed`, `breadcrumb`, and `navigation_menu::NAVIGATION` for the sidebar);
  - `frontend_session` (`AuthStore`, session persistence, `session_refresh::bootstrap` and
    `session_refresh::with_refresh_lock`), `frontend_transport` (`api_get`,
    `api_post_ok`, `MeResponse`, `User`, and `frontend_api_dtos::role` for `Role` and `has_min_role`),
    `frontend_ui` (the toast queue and viewport, `MaterialIcon`, `cn`) and
    `frontend_ui::safe_avatar_url`.
- Used by: `crates/frontend/shell/frontend_application/src/main.rs`, which mounts `AppLayout`; the fallback of
  `crates/frontend/shell/frontend_application/src/app_routes.rs` and the `*` row of
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`, which name `NotFoundPage`; the DOM oracle's `notfound`
  capture in `tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs`, whose captures of
  the chromed pages hold the frame too.
- Rules: `classify_frame` is the one place a frame is chosen, naming only `/login` and
  `/auth/callback` and reading every other layout from the route table (`classify_frame_kinds` in
  `tests/layout.rs`); `is_active` matches `/` exactly and every other link by path prefix
  (`is_active_dashboard_exact`, `is_active_prefix_and_exact`); each `NAVIGATION` path names a route
  in `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the account badge holds only the fields it displays
  (`a_profile_change_the_badge_does_not_display_yields_an_equal_badge` in `tests/top_nav.rs`);
  avatars pass through `safe_avatar_url` (`topnav_avatar_src_only_keeps_http_urls`); a frame
  component used outside its file is a documented `pub` item in a `pub mod`, since a leptos
  component's generated props builder is otherwise an unreachable `pub` item.

## Related documentation

- [App layout and navigation](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md)
  — the behaviour and design of the layout, the sidebar, the top bar and the not-found page.
- [Top bar blueprint](/documentation/crates/frontend/shell/frontend_application/shell/visual_references/topbar_blueprint/README.md)
  — the design-phase reference of the top bar and how the built bar differs.
- [Account pages](/documentation/crates/frontend/pages/account_pages/account_pages.md) — the
  sign-in pages the frame renders bare and the settings page the account menu opens.
