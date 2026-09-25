**Status:** live

# Frontend documentation

The documentation hub of the web platform's single-page app: every browser route with the code
folder that renders it and the feature doc that describes it, the page areas, the full-screen
workspaces such as the [Mission Creator](/documentation_v2/glossary.md#mission-creator), and the
shared foundations they are built on. Developers and AI agents start here before changing a page.

## Contents

```text
documentation_v2/website/frontend/
├── apps/   the full-screen workspaces: the Mission Creator and the debug benches
└── pages/  the routed pages, one folder per navigation area, with each page's feature doc
```

## How it works

The tree mirrors the code under `apps/website/frontend/src/v2/`, minus the `src/v2/` prefix and
with the code's folder spellings: `pages/<area>/<page>/` documents the page folder of the same
name, and `apps/<workspace>/` the workspace. A page folder holds a README index, the page's
feature doc, named after its route component (`event_schedule_page.md` for `EventSchedulePage`)
and written from the [feature doc template](/documentation_v2/standards/templates/feature_doc.md),
and, where a design set exists, a `visual_references/` folder. The in-code README of each page
folder holds what the code declares (routes, calls with their DTOs, states with their exact text);
the feature doc holds the flows, what each call means in the [API](/documentation_v2/glossary.md#api),
the design, the open work and the decisions. Open work lives only in the feature docs, as links to
tickets in `.ai/tickets/`.

The app is a Leptos 0.8 client-side-rendered app compiled to WebAssembly, which Trunk serves on
`127.0.0.1:3000` in development and proxies to the API; the crate README
[Website frontend](/apps/website/frontend/README.md) has the build, the configuration and the
commands.

### Route table

`apps/website/frontend/src/app_routes.rs` binds each path to its component, and the `ROUTES` table
in `apps/website/frontend/src/router.rs` gives each path its access tier and layout flags; the
[source root README](/apps/website/frontend/src/README.md#public-surface) lists the flags. A tier
is enforced in the browser after mount: `none` admits everyone, a `mission_maker` route sends a
viewer below that [role](/documentation_v2/glossary.md#role) back to the
[mission](/documentation_v2/glossary.md#mission) overview or the library, and an `admin` page
renders its own refusal through `AdminGate`. The rows below follow the sidebar's sections, as
`apps/website/frontend/src/v2/pages/navigation/nav_config.rs` orders them.

| Route | Component | Access | Code folder | Feature doc |
|---|---|---|---|---|
| `/login` | `LoginPage` | `none` | [account/login/](/apps/website/frontend/src/v2/pages/account/login/) | [account_pages.md](/documentation_v2/website/frontend/pages/account/account_pages.md) |
| `/auth/callback` | `AuthCallbackPage` | `none` | [account/auth_callback/](/apps/website/frontend/src/v2/pages/account/auth_callback/) | [account_pages.md](/documentation_v2/website/frontend/pages/account/account_pages.md) |
| `/settings` | `SettingsPage` | `none` | [account/settings/](/apps/website/frontend/src/v2/pages/account/settings/) | [account_pages.md](/documentation_v2/website/frontend/pages/account/account_pages.md) |
| `/` | `DashboardPage` | `none` | [command_center/dashboard/](/apps/website/frontend/src/v2/pages/command_center/dashboard/) | [dashboard_page.md](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md) |
| `/server-intel` | `ServerIntelPage` | `none` | [command_center/server_intel/](/apps/website/frontend/src/v2/pages/command_center/server_intel/) | [server_intel_page.md](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md) |
| `/announcements` and `/announcements/:id` | `AnnouncementsPage` | `none` | [command_center/announcements/](/apps/website/frontend/src/v2/pages/command_center/announcements/) | [announcements_page.md](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md) |
| `/events` | `EventSchedulePage` | `none` | [operations/schedule/](/apps/website/frontend/src/v2/pages/operations/schedule/) | [event_schedule_page.md](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md) |
| `/events/:id` | `EventHubPage` | `none` | [operations/event_detail/](/apps/website/frontend/src/v2/pages/operations/event_detail/) | [event_hub_page.md](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md) |
| `/events/:id/missions/:emid/orbat` | `OrbatSelectionPage` | `none` | [operations/orbat_selection/](/apps/website/frontend/src/v2/pages/operations/orbat_selection/) | [orbat_selection_page.md](/documentation_v2/website/frontend/pages/operations/orbat_selection/orbat_selection_page.md) |
| `/deployments` | `DeploymentsPage` | `none` | [operations/deployments/](/apps/website/frontend/src/v2/pages/operations/deployments/) | [deployments_page.md](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md) |
| `/leaderboards` | `LeaderboardsPage` | `none` | [operations/leaderboards/](/apps/website/frontend/src/v2/pages/operations/leaderboards/) | [leaderboards_page.md](/documentation_v2/website/frontend/pages/operations/leaderboards/leaderboards_page.md) |
| `/missions` | `MissionLibraryPage` | `none` | [mission_hub/library/](/apps/website/frontend/src/v2/pages/mission_hub/library/) | [mission_library_page.md](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md) |
| `/missions/:id` | `MissionOverviewPage` | `none` | [mission_hub/overview/](/apps/website/frontend/src/v2/pages/mission_hub/overview/) | [mission_overview_page.md](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md) |
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | [apps/editor/](/apps/website/frontend/src/v2/apps/editor/) | [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` | `mission_maker` | [mission_hub/review_workspace/](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/) | [review_workspace_page.md](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md) |
| `/tools/mortar` | `MortarCalculatorPage` | `none` | [field_tools/mortar/](/apps/website/frontend/src/v2/pages/field_tools/mortar/) | [mortar_calculator_page.md](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md) |
| `/wiki` and `/wiki/:slug` | `WikiPage` | `none` | [doctrine_and_info/wiki/](/apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/) | [wiki_page.md](/documentation_v2/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md) |
| `/vehicles` | `VehicleDatabasePage` | `none` | [doctrine_and_info/vehicles/](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/) | [vehicle_database_page.md](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md) |
| `/modpacks` | `ModpacksPage` | `none` | [doctrine_and_info/modpacks/](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/) | [modpacks_page.md](/documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md) |
| `/admin/events` | `EventManagerPage` | `admin` | [administration/event_manager/](/apps/website/frontend/src/v2/pages/administration/event_manager/) | [event_manager_page.md](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md) |
| `/admin/approvals` | `MissionApprovalsPage` | `admin` | [administration/approvals/](/apps/website/frontend/src/v2/pages/administration/approvals/) | [mission_approvals_page.md](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md) |
| `/admin/server` | `ServerControlPage` | `admin` | [administration/server_control/](/apps/website/frontend/src/v2/pages/administration/server_control/) | [server_control_page.md](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md) |
| `/admin/personnel` | `PersonnelRosterPage` | `admin` | [administration/personnel/](/apps/website/frontend/src/v2/pages/administration/personnel/) | [personnel_roster_page.md](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md) |
| `/admin/content` | `ContentManagerPage` | `admin` | [administration/content_manager/](/apps/website/frontend/src/v2/pages/administration/content_manager/) | [content_manager_page.md](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md) |
| `/admin/audit` | `AuditLogsPage` | `admin` | [administration/audit_logs/](/apps/website/frontend/src/v2/pages/administration/audit_logs/) | [audit_logs_page.md](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md) |
| `/debug/building-viewer` | `BuildingViewerPage` | `none` | [apps/debug/building_viewer/](/apps/website/frontend/src/v2/apps/debug/building_viewer/) | [building_viewer_page.md](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) |
| `/debug/world-los` | `WorldLosPage` | `none` | [apps/debug/world_los/](/apps/website/frontend/src/v2/apps/debug/world_los/) | [world_los_page.md](/documentation_v2/website/frontend/apps/debug/world_los_page.md) |
| any other path | `NotFoundPage` | `none` | [navigation/](/apps/website/frontend/src/v2/pages/navigation/) | [app_layout_and_navigation.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md) |

Page folders sit under `apps/website/frontend/src/v2/pages/`, workspace folders under
`apps/website/frontend/src/v2/`. Notes on the table:

- `/login` and `/auth/callback` render in the bare frame, and `/settings` sits under the top bar's
  account menu rather than the sidebar; the three share one feature doc.
- `/announcements/:id` and `/wiki/:slug` render the same component as their list route, with the
  item selected, so one feature doc covers each pair.
- `/events` asks `GET /api/v1/events` for its list with no `scope`, so the API's default,
  `upcoming`, applies (`list_events` in
  `apps/website/api_v2/src/operations/handlers/event_listing.rs`). The schedule embeds the
  [event](/documentation_v2/glossary.md#event) hub, whose feature doc describes the hub view and
  the [ORBAT](/documentation_v2/glossary.md#orbat) slotting once for the schedule, `/events/:id`
  and the ORBAT selection page alike.
- `/events/:id/missions/:emid/orbat` has no sidebar entry: the deployments page's "Modify
  Assignment" link and direct links lead to it, and its back link returns to the event hub.
- `/missions/:id/edit` and the review workspace fill the viewport without the sidebar and the top
  bar; the review workspace opens the Mission Creator read-only on the version an
  [artifact](/documentation_v2/glossary.md#artifact) was compiled from.
- `/debug/building-viewer` and `/debug/world-los` are URL-only: no navigation entry leads to them.

### Page areas and workspaces

[pages/](/documentation_v2/website/frontend/pages/README.md) indexes the eight areas: the six
sidebar sections ([command center](/documentation_v2/glossary.md#command-center),
[operations](/documentation_v2/glossary.md#operations), mission hub, field tools, doctrine and
info, [administration](/documentation_v2/glossary.md#administration)), the account pages and the
navigation frame. `apps/` holds the
[Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md),
starting from its [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md),
and the [debug benches documentation](/documentation_v2/website/frontend/apps/debug/README.md).
The code's `aar/` and `planner/` workspace folders hold no code and serve no route, so they have
no documentation folder.

### Shared foundations

The code under `apps/website/frontend/src/v2/core/` carries no route and no documentation folder
here: its in-code READMEs describe it exactly, and the deeper documents are linked from them.

| Module | What it holds | README |
|---|---|---|
| `core/` | the foundations every page and workspace shares, and the imports between them | [Shared foundations](/apps/website/frontend/src/v2/core/README.md) |
| `core/api/` | the HTTP client, the typed endpoint calls, the wire types and the live server status stream over [SSE](/documentation_v2/glossary.md#sse) | [API layer](/apps/website/frontend/src/v2/core/api/README.md) |
| `core/auth/` | the session store, the five-tier role ladder (`guest`, `enlisted`, `leader`, `mission_maker`, `admin`), the route guard and the link guard | [Session and access](/apps/website/frontend/src/v2/core/auth/README.md) |
| `core/ui/` | the interface primitives: the icon, page header, status pill, the two content gates, search box, select, slider, split pane, toasts, dialog and sheet | [Shared interface primitives](/apps/website/frontend/src/v2/core/ui/README.md) |
| `core/utils/` | timestamps, UTC instants, the countdown, the avatar sanitiser and the clipboard write | [Utilities](/apps/website/frontend/src/v2/core/utils/README.md) |
| `core/test_support/` | the source scrubber, the captured API responses and the source pins, for tests only | [Test support](/apps/website/frontend/src/v2/core/test_support/README.md) |

The wire types in `core/api/dto/` follow the API's models, and the API wins a disagreement; the
golden round trips in `apps/website/frontend/src/v2/core/api/dto/tests/` hold the two together.

### Design

The live design tokens are `apps/website/frontend/style/aegis.css`, which Tailwind CSS compiles
into the build, and the [design tokens](/documentation_v2/design_system/design_tokens.md)
reference describes them. A page's `visual_references/` sets are design-phase references kept for
colour and layout context, never an implementation source: the built UI is the Leptos code, and
each feature doc's Design section lists how the page differs from its set.

### Adding a page

A new route gets a row in `app_routes.rs` and in `ROUTES` (the route drift gate diffs `ROUTES`
against `tools_v2/developer-tools/fixtures/dom_oracle/manifests/routes.csv`), an in-code README
in its page folder, a folder here under its area named like the code folder, with a README and
its feature doc, and a row in the route table above.

## Code

- [Website frontend](/apps/website/frontend/) — the crate: build files, the Aegis stylesheet and
  the captured API responses.
- [Frontend source root](/apps/website/frontend/src/) — the entry point and the two forms of the
  route table the table above is read from.
- [Pages](/apps/website/frontend/src/v2/pages/) — the routed pages and the navigation frame that
  `pages/` documents.
- [Workspaces](/apps/website/frontend/src/v2/apps/) — the Mission Creator and the debug benches
  that `apps/` documents.
- [Shared foundations](/apps/website/frontend/src/v2/core/) — the client, session, primitives and
  utilities under Shared foundations.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md)
  and the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation_v2/glossary.md); the route table in
  `apps/website/frontend/src/app_routes.rs` and `apps/website/frontend/src/router.rs`, the sidebar
  in `apps/website/frontend/src/v2/pages/navigation/nav_config.rs` and the in-code READMEs, which
  this hub is written from.
- Used by: the documentation root README; the READMEs of `apps/website/`, the frontend crate and
  its `src/v2/` tree; the API overview; the commit checklist and the ticket identifiers standard
  in `documentation_v2/standards/`; tickets in `.ai/tickets/` that name it.
- Rules: every route in `ROUTES` has exactly one row in the route table, and a route change
  updates the table in the same commit; a documentation folder mirrors a code folder and keeps its
  spelling; feature docs, not this hub, hold behaviour, design and open work; the hub and its
  README indexes name no ticket.

## Related documentation

- [Page areas](/documentation_v2/website/frontend/pages/README.md) — the index of the eight page
  areas.
- [API overview](/documentation_v2/website/api_v2/api_overview.md) — the routes of every API
  domain the pages call.
- [Documentation standards](/documentation_v2/standards/documentation_standards.md#2-contracts-behind-the-tags)
  — how the API's models, the schemas and the frontend's wire types stay one contract.
- [Where does X go](/documentation_v2/standards/where_does_x_go.md) — where new frontend code and
  documents belong.
- [Local development](/documentation_v2/runbooks/local_development.md) — running the API and the
  app locally, the dev login included.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the headless browser gates that
  drive the built app.
- [Archived frontend roadmap](/documentation_v2/archive/go_and_react_era_design/frontend_roadmap.md)
  — the planning view and shipped log this hub replaces.
