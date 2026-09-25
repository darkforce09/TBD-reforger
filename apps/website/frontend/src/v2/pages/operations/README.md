# Operations pages

The [operations](/documentation_v2/glossary.md#operations) pages, the second section of the sidebar:
the schedule of upcoming [events](/documentation_v2/glossary.md#event), each event's hub with its
inline [ORBAT](/documentation_v2/glossary.md#orbat) slotting, one
[mission](/documentation_v2/glossary.md#mission)'s slotting on a page of its own, the viewer's own
[service record](/documentation_v2/glossary.md#service-record), and the global leaderboards.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/
├── deployments/      the viewer's service record, their leave requests and the leave review queue
├── event_detail/     one event's hub: its missions, places and inline slotting
├── leaderboards/     five global ladders and the operator dossier behind each row
├── mod.rs            the module tree
├── orbat_selection/  one mission's slotting on a page of its own, for direct links
└── schedule/         the upcoming events beside the selected event's hub
```

## How it works

| Page title | Route | Component |
|---|---|---|
| "Event Schedule" | `/events` | `EventSchedulePage` in `schedule/` |
| "Event Hub" | `/events/:id` | `EventHubPage` in `event_detail/` |
| "ORBAT Selection" | `/events/:id/missions/:emid/orbat` | `OrbatSelectionPage` in `orbat_selection/` |
| "My Deployments" | `/deployments` | `DeploymentsPage` in `deployments/` |
| "Global Leaderboards" | `/leaderboards` | `LeaderboardsPage` in `leaderboards/` |

Every page renders its content inside `AuthGate` and fetches in the browser build only. The event
hub has one renderer and the slotting one implementation, each shown in two places: the schedule's
detail column renders `event_hub_view` from `event_detail/`, and the ORBAT selection page mounts its
`OrbatSelector` with the `MissionStanding` it takes. The service record page links into both, from
the banner of the viewer's next [deployment](/documentation_v2/glossary.md#deployment).

## Public surface

- `DeploymentsPage`, `EventHubPage`, `EventSchedulePage`, `LeaderboardsPage` and
  `OrbatSelectionPage`: the route components `apps/website/frontend/src/app_routes.rs` mounts; each
  child's README gives its route, tier and layout.

## Boundaries

- Depends on: `crate::v2::core` (the [API](/documentation_v2/glossary.md#api) client, its DTOs and
  the `event_registration` endpoint helpers, the `AuthStore` session and
  [role](/documentation_v2/glossary.md#role) checks, the UI primitives and the date and countdown
  helpers); over HTTP, the `operations` domain of the API (`/api/v1/events`,
  `/api/v1/event-missions/…`, `/api/v1/members`, `/api/v1/me/deployments`,
  `/api/v1/me/leave-requests` and `/api/v1/admin/leave-requests`), its `command_center` domain
  (`/api/v1/leaderboards`, `/api/v1/users/{discordId}/stats`) and its `community_content` domain
  (`/api/v1/modpacks`).
- Used by: the route table in `apps/website/frontend/src/app_routes.rs`, whose tiers and layout
  flags `apps/website/frontend/src/router.rs` declares; the source pins in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: `event_hub_view` is the one renderer of an event and `OrbatSelector` the one slotting
  view, so the schedule, the event hub and the ORBAT selection page cannot drift apart
  (`schedule_briefing_empty_check_stays_trim_aligned` in `schedule/tests/schedule.rs` keeps the
  schedule on the shared hub body).

## Related documentation

- [Event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
  — the schedule's behaviour and design.
- [Event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
  — the event dossier and its slotting.
- [ORBAT selection page](/documentation_v2/website/frontend/pages/operations/orbat_selection/orbat_selection_page.md)
  — one mission's slotting on a page of its own.
- [Deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md)
  — the service record page.
- [Leaderboards page](/documentation_v2/website/frontend/pages/operations/leaderboards/leaderboards_page.md)
  — the global ladders.
- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the API routes these pages
  call.
