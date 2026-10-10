**Status:** live

# Operations pages

The documentation of the five [operations](/documentation/glossary/n_to_z.md#operations) pages, the
second section of the sidebar and the pages it links into, one folder per page: each holds the
page's feature doc and, where a design set exists, its design references. Developers and AI agents
read it before changing an operations page.

## Contents

```text
documentation/crates/frontend/pages/operations_pages/
├── deployments/      the deployments page: the viewer's service record and leave requests
├── event_detail/     the event hub page: one event's places, mission dossiers and slotting
├── leaderboards/     the leaderboards page: five ranked ladders and the operator dossier
├── orbat_selection/  the ORBAT selection page: one mission's slotting on a page of its own
└── schedule/         the event schedule page: upcoming events beside the selected event's hub
```

## How it works

The folders mirror the page folders under `crates/frontend/pages/operations_pages/src/` and
keep their spelling. Each holds a README index and the page's feature doc; the schedule,
deployments and leaderboards folders add a `visual_references/` folder with one design-phase
blueprint set. A feature doc follows the
[feature doc template](/documentation/standards/templates/feature_doc.md): Where it lives,
Behaviour (ending in the known discrepancies between the page and the
[API](/documentation/glossary/a_to_f.md#api)), Data (what each call means server-side), Design (the
layout as built and each difference from the design target), Open work and Decisions.

The [event](/documentation/glossary/a_to_f.md#event) hub has one renderer and the
[ORBAT](/documentation/glossary/n_to_z.md#orbat) slotting one implementation, each shown in more than
one place, so the event hub feature doc describes them once and the schedule and ORBAT selection
feature docs link to it. Start with the feature doc of the page at hand; for anything about
registering, squads or the waiting list, start with the event hub.

Every page is declared in `crates/frontend/foundation/frontend_route_table/src/routes.rs` with the route tier `none` and
renders its body inside `AuthGate` (`crates/frontend/foundation/frontend_session/src/gates.rs`), so only a
signed-in viewer sees data; the deployments page adds a review queue for the `admin`
[role](/documentation/glossary/n_to_z.md#role), and the slotting adds squad controls for `leader`
and above.

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Event schedule | `/events`, `EventSchedulePage` | Event Schedule; heading "Upcoming Ops" | [event_schedule_page.md](/documentation/crates/frontend/pages/operations_pages/schedule/event_schedule_page.md) |
| Event hub | `/events/:id`, `EventHubPage` | Event Hub; heading "Operation Hub" | [event_hub_page.md](/documentation/crates/frontend/pages/operations_pages/event_detail/event_hub_page.md) |
| ORBAT selection | `/events/:id/missions/:emid/orbat`, `OrbatSelectionPage` | ORBAT Selection; the mission's title | [orbat_selection_page.md](/documentation/crates/frontend/pages/operations_pages/orbat_selection/orbat_selection_page.md) |
| Deployments | `/deployments`, `DeploymentsPage` | My Deployments | [deployments_page.md](/documentation/crates/frontend/pages/operations_pages/deployments/deployments_page.md) |
| Leaderboards | `/leaderboards`, `LeaderboardsPage` | Global Leaderboards | [leaderboards_page.md](/documentation/crates/frontend/pages/operations_pages/leaderboards/leaderboards_page.md) |

A new operations page gets a folder here named like its code folder, with a README, its feature
doc and, when a design set exists, a `visual_references/` folder; it also gets a line in Contents
and a row in the table.

## Code

- [Operations pages](/crates/frontend/pages/operations_pages/src/) — the five route components,
  the hub body and the slotting selector, which the feature docs describe.
- [Operations domain](/crates/api/api_operations/src/) — events, their hubs and ORBATs,
  registration, squad holds, the waiting list, the service record and leave requests.
- [Command center domain](/crates/api/api_command_center/src/) — the leaderboard and the
  per-player statistics.
- [Community content domain](/crates/api/api_community_content/src/) — the modpacks the hub
  names.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the page code, the API handlers it calls and the
  ticket manager (`ttm`), which the feature docs are written from.
- Used by: the in-code READMEs of the page folders, which link their feature docs under Related
  documentation; the operations domain READMEs; the glossary's event and service record entries;
  the feature doc template's worked sample; the event manager feature doc; the web app README's
  page table in `documentation/crates/frontend/shell/frontend_application/`.
- Rules: one folder per page folder of the code, spelled the same; a page's feature doc keeps its
  name (`event_hub_page.md` for `EventHubPage`), since the glossary and the READMEs link it; the
  slotting is described only in the event hub feature doc; a feature doc stays within 500 lines;
  design references live only in `visual_references/`, and no document holds a screenshot of the
  built UI.

## Related documentation

- [Event eligibility and allocation](/documentation/crates/api/api_server/design_notes/event_eligibility_allocation.md)
  — the design behind access, pools, promotion and attendance.
