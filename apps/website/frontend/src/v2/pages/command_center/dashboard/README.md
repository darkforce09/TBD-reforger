# Dashboard page

The `/` page, the members' landing screen in the
[command center](/documentation_v2/glossary/a_to_f.md#command-center): the countdown to the next
[event](/documentation_v2/glossary/a_to_f.md#event), the primary game server's status, the viewer's own
assignment, the current modpack and the latest announcements, all from one fetch.

## Contents

```text
apps/website/frontend/src/v2/pages/command_center/dashboard/
├── deployment.rs     the Deployment card: the viewer's faction, squad and role for the next event
├── helpers.rs        `vstr` and `vbool`: total field reads over the untyped parts of the payload
├── hero_banner.rs    the banner: the countdown to the next event, its name and terrain, a hub link
├── mod.rs            the module tree; re-exports `DashboardPage`
├── modpack.rs        the Modpack card: the current modpack's name, version, size and sync chip
├── page.rs           the route component: the dashboard fetch and the panel layout
├── recent_intel.rs   the Recent Intelligence feed: the latest announcements as linked rows
└── server_uplink.rs  the Server Uplink card: online state, players and fill bar, frame rate, uptime
```

## How it works

`DashboardPage` renders inside `AuthGate`. The signed-in half fetches the dashboard payload once
and hands each panel its own slice, owned, so no panel reads the resource again:

```text
DashboardResponse
├── next_event            ─► hero_banner    (full width)
├── server_status         ─► server_uplink  ┐
├── my_assignment         ─► deployment     ├ the three-column card grid
├── current_modpack       ─► modpack_card   ┘
└── recent_announcements  ─► recent_intel   (grows to fill the remaining height)
```

Every panel keeps its shape when its slice is empty and shows its empty text instead. The banner
prefixes the countdown with "T-MINUS " except when it reads `LIVE NOW`, and draws over a fixed
backdrop image (`HERO_IMAGE`, an `lh3.googleusercontent.com` URL). The uplink card's fill bar is a
whole percentage of the player cap, empty when the cap is zero, and prints the frame rate as the
wire sent it. A feed row links to its announcement, or to `/announcements` when the row has no id,
and previews its `snippet` or else the first paragraph of its `body`. The next event, the
assignment and the announcements are untyped JSON, read through `helpers.rs`, where a missing key,
a null and a wrong type all read as empty.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/` | `DashboardPage` | route tier `none`; the data renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Command Center / Dashboard |

## Data

- `GET /api/v1/dashboard`: read as `DashboardResponse`: `next_event` (`start_time`, `name`,
  `terrain`, `event_id`), `my_assignment` (`faction`, `squad`, `role`), `server_status` as
  `ServerStatusDto`, `current_modpack` as `ModpackDto`, and `recent_announcements` (`id`, `title`,
  `is_pinned`, `published_at`, `snippet`, `body`).
- The page reads the session from the `AuthStore` context and writes nothing. The fetch runs in
  the browser build only; a native build renders the failure branch.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load data." |
| loaded | the banner ("T-MINUS " and the countdown, "OPERATION: <name> — <terrain>", an "Open Operation Hub" link to `/events/{event_id}`), the "Server Uplink", "Deployment" and "Modpack" cards, and "Recent Intelligence" |
| no next event | "NO UPCOMING OPS" and "Check the event schedule for new operations.", with no link |
| no server status | "OFFLINE", 0/0 players, an empty bar, and "—" after "FPS: " and "UPTIME: " |
| no assignment | "No active assignment" under the "Deployment" heading |
| no modpack | "No modpack", "—" in place of the size, and "STATUS: " with a grey "NONE" in place of the green "SYNCED" |
| no announcements | "No announcements yet." |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get` client, `DashboardResponse`,
  `ServerStatusDto`, `ModpackDto`), `crate::v2::core::ui` (`AuthGate`, `MaterialIcon`, `cn`),
  `crate::v2::core::utils` (countdown, short date and uptime formatting), the `AuthStore` context,
  and the banner image on `lh3.googleusercontent.com`.
- Used by: the `/` route in `apps/website/frontend/src/app_routes.rs`.
- Rules: the fetch belongs to `page.rs` and each panel receives its data owned; a panel with nothing
  to show keeps its heading and shape and says so, so the card grid always holds three cells. No
  test pins these rules.

## Related documentation

- [Dashboard page](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md)
  — the page's behaviour and design.
