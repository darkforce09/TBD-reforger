# Command center pages

The web app's [command center](/documentation/glossary/a_to_f.md#command-center), the first section of
the sidebar: the read-only screens a member lands on after signing in, reporting the unit's current
situation (the next [event](/documentation/glossary/a_to_f.md#event), the game server, the viewer's
assignment and the latest announcements).

## Contents

```text
crates/frontend/pages/command_center_pages/src/
├── announcements/  the announcement board: the list and the reading pane for one announcement
├── dashboard/      the landing screen: next event, server uplink, assignment, modpack, latest news
├── lib.rs          the crate root: the module tree
├── prelude.rs      the three route components the app's route table mounts
└── server_intel/   one game server's live panel, kept current by its status stream
```

## How it works

| Page | Route | Component |
|---|---|---|
| Dashboard | `/` | `DashboardPage` in `dashboard/` |
| Server Intel | `/server-intel` | `ServerIntelPage` in `server_intel/` |
| Announcements | `/announcements`, `/announcements/:id` | `AnnouncementsPage` in `announcements/` |

Each page renders its content inside `AuthGate`, owns one fetch, and renders that one payload; none
of them writes platform data. Every fetch runs in the browser build only; the views that run them exist in that build only. The server intel page alone holds a live
connection, its server's [SSE](/documentation/glossary/n_to_z.md#sse) status stream. The dashboard's
Recent Intelligence rows link into the announcement board, and its banner into the event hub page.

## Public surface

- `AnnouncementsPage`, `DashboardPage` and `ServerIntelPage`: the route components
  `crates/frontend/shell/frontend_application/src/app_routes.rs` mounts (`wasm32`), re-exported by each page folder and by
  `prelude.rs`; each page folder's `page` module is public, as leptos's derived props builders
  need; each child's README gives its route, tier and layout.

## Boundaries

- Depends on: the foundation crates (the [API](/documentation/glossary/a_to_f.md#api) client, its DTOs and
  the SSE subscriber, the `AuthStore` session, the UI primitives and the formatting helpers); over
  HTTP, the API's `command_center` domain (`/api/v1/dashboard`), its `community_content` domain
  (`/api/v1/announcements`) and its `server_infrastructure` domain (`/api/v1/servers` and the status
  stream).
- Used by: the route table in `crates/frontend/shell/frontend_application/src/app_routes.rs`, whose tiers and layout
  flags `crates/frontend/foundation/frontend_route_table/src/routes.rs` declares.
- Rules: the pages only read (each calls `api_get` and nothing that writes), and every page sits
  behind `AuthGate`; no test pins the read-only rule.

## Related documentation

- [Dashboard page](/documentation/crates/frontend/pages/command_center_pages/dashboard/dashboard_page.md)
  — the landing screen.
- [Server intel page](/documentation/crates/frontend/pages/command_center_pages/server_intel/server_intel_page.md)
  — the live server panel.
- [Announcements page](/documentation/crates/frontend/pages/command_center_pages/announcements/announcements_page.md)
  — the announcement board.
- [Command center domain](/crates/api/api_command_center/src/README.md) — the dashboard route.
