# Command center pages

The web app's [command center](/documentation_v2/glossary/a_to_f.md#command-center), the first section of
the sidebar: the read-only screens a member lands on after signing in, reporting the unit's current
situation (the next [event](/documentation_v2/glossary/a_to_f.md#event), the game server, the viewer's
assignment and the latest announcements).

## Contents

```text
apps/website/frontend/src/v2/pages/command_center/
├── announcements/  the announcement board: the list and the reading pane for one announcement
├── dashboard/      the landing screen: next event, server uplink, assignment, modpack, latest news
├── mod.rs          the module tree; re-exports the three route components
└── server_intel/   one game server's live panel, kept current by its status stream
```

## How it works

| Page | Route | Component |
|---|---|---|
| Dashboard | `/` | `DashboardPage` in `dashboard/` |
| Server Intel | `/server-intel` | `ServerIntelPage` in `server_intel/` |
| Announcements | `/announcements`, `/announcements/:id` | `AnnouncementsPage` in `announcements/` |

Each page renders its content inside `AuthGate`, owns one fetch, and renders that one payload; none
of them writes platform data. Every fetch runs in the browser build only, so a native build
resolves it to `None` and renders the failure branch. The server intel page alone holds a live
connection, its server's [SSE](/documentation_v2/glossary/n_to_z.md#sse) status stream. The dashboard's
Recent Intelligence rows link into the announcement board, and its banner into the event hub page.

## Public surface

- `AnnouncementsPage`, `DashboardPage` and `ServerIntelPage`: the route components
  `apps/website/frontend/src/app_routes.rs` mounts; each child's README gives its route, tier and
  layout.

## Boundaries

- Depends on: `crate::v2::core` (the [API](/documentation_v2/glossary/a_to_f.md#api) client, its DTOs and
  the SSE subscriber, the `AuthStore` session, the UI primitives and the formatting helpers); over
  HTTP, the API's `command_center` domain (`/api/v1/dashboard`), its `community_content` domain
  (`/api/v1/announcements`) and its `server_infrastructure` domain (`/api/v1/servers` and the status
  stream).
- Used by: the route table in `apps/website/frontend/src/app_routes.rs`, whose tiers and layout
  flags `apps/website/frontend/src/router.rs` declares; the source pins in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: the pages only read (each calls `api_get` and nothing that writes), and every page sits
  behind `AuthGate`; no test pins the read-only rule.

## Related documentation

- [Dashboard page](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md)
  — the landing screen.
- [Server intel page](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md)
  — the live server panel.
- [Announcements page](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md)
  — the announcement board.
- [Command center domain](/apps/website/api_v2/src/command_center/README.md) — the dashboard route.
