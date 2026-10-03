**Status:** live

# Frontend documentation

The documentation hub of the web platform's single-page app: every browser route with the code
folder that renders it and the feature doc that describes it, the page areas, the full-screen
workspaces such as the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), and the
shared foundations they are built on. Developers and AI agents start here before changing a page.

## Contents

```text
documentation/apps/frontend/
├── pages/       the routed pages, one folder per navigation area, with each page's feature doc
├── shell/       the app frame: the layout, the sidebar, the top bar and the not-found page
└── workspaces/  the full-screen workspaces: the Mission Creator and the debug benches
```

## How it works

The tree mirrors the code under `apps/frontend/src/`, minus the `src/` prefix and with the
code's folder spellings: `pages/<area>/<page>/` documents the page folder of the same name,
`workspaces/<workspace>/` the workspace, and `shell/` the app frame. A page folder holds a
README index, the page's feature doc, named after its route component (`event_schedule_page.md`
for `EventSchedulePage`) and written from the
[feature doc template](/documentation/standards/templates/feature_doc.md), and, where a design set
exists, a `visual_references/` folder. The in-code README of each page
folder holds what the code declares (routes, calls with their DTOs, states with their exact text);
the feature doc holds the flows, what each call means in the [API](/documentation/glossary/a_to_f.md#api),
the design, the open work and the decisions. Open work lives only in the feature docs, as links to
tickets in `.ai/tickets/`.

The app is a Leptos 0.8 client-side-rendered app compiled to WebAssembly, which Trunk serves on
`127.0.0.1:3000` in development and proxies to the API; the crate README
[Website frontend](/apps/frontend/README.md) has the build, the configuration and the
commands.

### Route table

`apps/frontend/src/app_routes.rs` binds each path to its component, and the `ROUTES` table
in `apps/frontend/src/foundation/route_table/mod.rs` gives each path its access tier and layout flags; the
[route table README](/apps/frontend/src/foundation/route_table/README.md#public-surface) lists the
flags. A tier
is enforced in the browser after mount: `none` admits everyone, a `mission_maker` route sends a
viewer below that [role](/documentation/glossary/n_to_z.md#role) back to the
[mission](/documentation/glossary/g_to_m.md#mission) overview or the library, and an `admin` page
renders its own refusal through `AdminGate`. The rows below follow the sidebar's sections, as
`apps/frontend/src/foundation/route_table/navigation_menu.rs` orders them.

| Route | Component | Access | Code folder | Feature doc |
|---|---|---|---|---|
| `/login` | `LoginPage` | `none` | [account/login/](/apps/frontend/src/pages/account/login/) | [account_pages.md](/documentation/apps/frontend/pages/account/account_pages.md) |
| `/auth/callback` | `AuthCallbackPage` | `none` | [account/auth_callback/](/apps/frontend/src/pages/account/auth_callback/) | [account_pages.md](/documentation/apps/frontend/pages/account/account_pages.md) |
| `/settings` | `SettingsPage` | `none` | [account/settings/](/apps/frontend/src/pages/account/settings/) | [account_pages.md](/documentation/apps/frontend/pages/account/account_pages.md) |
| `/` | `DashboardPage` | `none` | [command_center/dashboard/](/apps/frontend/src/pages/command_center/dashboard/) | [dashboard_page.md](/documentation/apps/frontend/pages/command_center/dashboard/dashboard_page.md) |
| `/server-intel` | `ServerIntelPage` | `none` | [command_center/server_intel/](/apps/frontend/src/pages/command_center/server_intel/) | [server_intel_page.md](/documentation/apps/frontend/pages/command_center/server_intel/server_intel_page.md) |
| `/announcements` and `/announcements/:id` | `AnnouncementsPage` | `none` | [command_center/announcements/](/apps/frontend/src/pages/command_center/announcements/) | [announcements_page.md](/documentation/apps/frontend/pages/command_center/announcements/announcements_page.md) |
| `/events` | `EventSchedulePage` | `none` | [operations/schedule/](/apps/frontend/src/pages/operations/schedule/) | [event_schedule_page.md](/documentation/apps/frontend/pages/operations/schedule/event_schedule_page.md) |
| `/events/:id` | `EventHubPage` | `none` | [operations/event_detail/](/apps/frontend/src/pages/operations/event_detail/) | [event_hub_page.md](/documentation/apps/frontend/pages/operations/event_detail/event_hub_page.md) |
| `/events/:id/missions/:emid/orbat` | `OrbatSelectionPage` | `none` | [operations/orbat_selection/](/apps/frontend/src/pages/operations/orbat_selection/) | [orbat_selection_page.md](/documentation/apps/frontend/pages/operations/orbat_selection/orbat_selection_page.md) |
| `/deployments` | `DeploymentsPage` | `none` | [operations/deployments/](/apps/frontend/src/pages/operations/deployments/) | [deployments_page.md](/documentation/apps/frontend/pages/operations/deployments/deployments_page.md) |
| `/leaderboards` | `LeaderboardsPage` | `none` | [operations/leaderboards/](/apps/frontend/src/pages/operations/leaderboards/) | [leaderboards_page.md](/documentation/apps/frontend/pages/operations/leaderboards/leaderboards_page.md) |
| `/missions` | `MissionLibraryPage` | `none` | [mission_hub/library/](/apps/frontend/src/pages/mission_hub/library/) | [mission_library_page.md](/documentation/apps/frontend/pages/mission_hub/library/mission_library_page.md) |
| `/missions/:id` | `MissionOverviewPage` | `none` | [mission_hub/overview/](/apps/frontend/src/pages/mission_hub/overview/) | [mission_overview_page.md](/documentation/apps/frontend/pages/mission_hub/overview/mission_overview_page.md) |
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | [workspaces/editor/](/apps/frontend/src/workspaces/editor/) | [Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md) |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` | `mission_maker` | [workspaces/editor/review_workspace/](/apps/frontend/src/workspaces/editor/review_workspace/) | [review_workspace_page.md](/documentation/apps/frontend/workspaces/editor/review_workspace/review_workspace_page.md) |
| `/tools/mortar` | `MortarCalculatorPage` | `none` | [field_tools/mortar/](/apps/frontend/src/pages/field_tools/mortar/) | [mortar_calculator_page.md](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md) |
| `/wiki` and `/wiki/:slug` | `WikiPage` | `none` | [doctrine_and_info/wiki/](/apps/frontend/src/pages/doctrine_and_info/wiki/) | [wiki_page.md](/documentation/apps/frontend/pages/doctrine_and_info/wiki/wiki_page.md) |
| `/vehicles` | `VehicleDatabasePage` | `none` | [doctrine_and_info/vehicles/](/apps/frontend/src/pages/doctrine_and_info/vehicles/) | [vehicle_database_page.md](/documentation/apps/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md) |
| `/modpacks` | `ModpacksPage` | `none` | [doctrine_and_info/modpacks/](/apps/frontend/src/pages/doctrine_and_info/modpacks/) | [modpacks_page.md](/documentation/apps/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md) |
| `/admin/events` | `EventManagerPage` | `admin` | [administration/event_manager/](/apps/frontend/src/pages/administration/event_manager/) | [event_manager_page.md](/documentation/apps/frontend/pages/administration/event_manager/event_manager_page.md) |
| `/admin/approvals` | `MissionApprovalsPage` | `admin` | [administration/approvals/](/apps/frontend/src/pages/administration/approvals/) | [mission_approvals_page.md](/documentation/apps/frontend/pages/administration/approvals/mission_approvals_page.md) |
| `/admin/server` | `ServerControlPage` | `admin` | [administration/server_control/](/apps/frontend/src/pages/administration/server_control/) | [server_control_page.md](/documentation/apps/frontend/pages/administration/server_control/server_control_page.md) |
| `/admin/personnel` | `PersonnelRosterPage` | `admin` | [administration/personnel/](/apps/frontend/src/pages/administration/personnel/) | [personnel_roster_page.md](/documentation/apps/frontend/pages/administration/personnel/personnel_roster_page.md) |
| `/admin/content` | `ContentManagerPage` | `admin` | [administration/content_manager/](/apps/frontend/src/pages/administration/content_manager/) | [content_manager_page.md](/documentation/apps/frontend/pages/administration/content_manager/content_manager_page.md) |
| `/admin/audit` | `AuditLogsPage` | `admin` | [administration/audit_logs/](/apps/frontend/src/pages/administration/audit_logs/) | [audit_logs_page.md](/documentation/apps/frontend/pages/administration/audit_logs/audit_logs_page.md) |
| `/admin/ballistics-catalogs` | `BallisticsCatalogsPage` | `admin` | [administration/ballistics_catalogs/](/apps/frontend/src/pages/administration/ballistics_catalogs/) | [ballistics_catalogs_page.md](/documentation/apps/frontend/pages/administration/ballistics_catalogs/ballistics_catalogs_page.md) |
| `/debug/building-viewer` | `BuildingViewerPage` | `none` | [workspaces/debug/building_viewer/](/apps/frontend/src/workspaces/debug/building_viewer/) | [building_viewer_page.md](/documentation/apps/frontend/workspaces/debug/building_viewer_page.md) |
| `/debug/world-los` | `WorldLosPage` | `none` | [workspaces/debug/world_los/](/apps/frontend/src/workspaces/debug/world_los/) | [world_los_page.md](/documentation/apps/frontend/workspaces/debug/world_los_page.md) |
| `/debug/ballistics-agreement` | `BallisticsAgreementPage` | `none` | [workspaces/debug/ballistics_agreement/](/apps/frontend/src/workspaces/debug/ballistics_agreement/) | [ballistics_agreement_page.md](/documentation/apps/frontend/workspaces/debug/ballistics_agreement_page.md) |
| any other path | `NotFoundPage` | `none` | [shell/](/apps/frontend/src/shell/) | [app_layout_and_navigation.md](/documentation/apps/frontend/shell/app_layout_and_navigation.md) |

Page folders sit under `apps/frontend/src/pages/`, workspace folders under
`apps/frontend/src/workspaces/`, the frame under `apps/frontend/src/shell/`. Notes on the table:

- `/login` and `/auth/callback` render in the bare frame, and `/settings` sits under the top bar's
  account menu rather than the sidebar; the three share one feature doc.
- `/announcements/:id` and `/wiki/:slug` render the same component as their list route, with the
  item selected, so one feature doc covers each pair.
- `/events` asks `GET /api/v1/events` for its list with no `scope`, so the API's default,
  `upcoming`, applies (`list_events` in
  `crates/api/api_operations/src/handlers/event_listing.rs`). The schedule embeds the
  [event](/documentation/glossary/a_to_f.md#event) hub, whose feature doc describes the hub view and
  the [ORBAT](/documentation/glossary/n_to_z.md#orbat) slotting once for the schedule, `/events/:id`
  and the ORBAT selection page alike.
- `/events/:id/missions/:emid/orbat` has no sidebar entry: the deployments page's "Modify
  Assignment" link and direct links lead to it, and its back link returns to the event hub.
- `/missions/:id/edit` and the review workspace fill the viewport without the sidebar and the top
  bar; the review workspace opens the Mission Creator read-only on the version an
  [artifact](/documentation/glossary/a_to_f.md#artifact) was compiled from.
- `/debug/building-viewer`, `/debug/world-los` and `/debug/ballistics-agreement` are URL-only: no
  navigation entry leads to them.

### Page areas and workspaces

[pages/](/documentation/apps/frontend/pages/README.md) indexes the seven areas: the six
sidebar sections ([command center](/documentation/glossary/a_to_f.md#command-center),
[operations](/documentation/glossary/n_to_z.md#operations), mission hub, field tools, doctrine and
info, [administration](/documentation/glossary/a_to_f.md#administration)) and the account pages.
[shell/](/documentation/apps/frontend/shell/README.md) documents the app frame. `workspaces/`
holds the
[Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md),
starting from its [roadmap](/documentation/apps/frontend/workspaces/editor/mission_creator_roadmap.md),
and the [debug benches documentation](/documentation/apps/frontend/workspaces/debug/README.md).
The code's `aar/` and `planner/` workspace folders hold no code and serve no route, so they have
no documentation folder.

### Shared foundations

The code under `apps/frontend/src/foundation/` and `apps/frontend/src/features/` carries no
route and no documentation folder here: its in-code READMEs describe it exactly, and the deeper
documents are linked from them.

| Module | What it holds | README |
|---|---|---|
| `foundation/` | the foundations every layer shares, and the imports between them | [Shared foundations](/apps/frontend/src/foundation/README.md) |
| `foundation/transport/` | the HTTP client, the typed endpoint calls, the wire types with the five-tier role ladder (`guest`, `enlisted`, `leader`, `mission_maker`, `admin`), the token provider the session implements, and the live server status stream over [SSE](/documentation/glossary/n_to_z.md#sse) | [API layer](/apps/frontend/src/foundation/transport/README.md) |
| `foundation/auth/` | the session store, the cross-tab session refresh, the sign-out hooks, the route guard and the two content gates | [Session and access](/apps/frontend/src/foundation/auth/README.md) |
| `foundation/ui/` | the interface primitives: the icon, page header, status pill, search box, select, slider and their hover and disabled classes, split pane, toasts, dialog and sheet | [Shared interface primitives](/apps/frontend/src/foundation/ui/README.md) |
| `foundation/utils/` | timestamps, UTC instants, the countdown, the avatar sanitiser and the clipboard write | [Utilities](/apps/frontend/src/foundation/utils/README.md) |
| `foundation/route_table/` | every route's path, component name, layout flags and access tier, and the sidebar's menu | [Route table](/apps/frontend/src/foundation/route_table/README.md) |
| `foundation/test_support/` | the source scrubber, the captured API responses and the source pins, for tests only | [Test support](/apps/frontend/src/foundation/test_support/README.md) |
| `features/mission_review_record/` | a mission's review history, thread, provenance and submit control, shared by the mission hub, the approvals queue and the review workspace | [Mission review record](/apps/frontend/src/features/mission_review_record/README.md) |

The wire types in `foundation/transport/dto/` follow the API's models, and the API wins a disagreement; the
golden round trips in `apps/frontend/src/foundation/transport/dto/tests/` hold the two together.

### Design

The live design tokens are `apps/frontend/style/aegis.css`, which Tailwind CSS compiles
into the build, and the [design tokens](/documentation/design_system/design_tokens.md)
reference describes them. A page's `visual_references/` sets are design-phase references kept for
colour and layout context, never an implementation source: the built UI is the Leptos code, and
each feature doc's Design section lists how the page differs from its set.

### Adding a page

A new route gets a row in `app_routes.rs` and in `ROUTES` (the route drift gate diffs `ROUTES`
against `tools/browser_testing/browser_gate_suites/fixtures/dom_oracle/manifests/routes.csv`), an in-code README
in its page folder, a folder here under its area named like the code folder, with a README and
its feature doc, and a row in the route table above.

## Code

- [Website frontend](/apps/frontend/) — the crate: build files, the Aegis stylesheet and
  the captured API responses.
- [Frontend source root](/apps/frontend/src/) — the entry point, the render form of the route
  table and the five layers.
- [Pages](/apps/frontend/src/pages/) — the routed pages that `pages/` documents.
- [App shell](/apps/frontend/src/shell/) — the frame that `shell/` documents.
- [Workspaces](/apps/frontend/src/workspaces/) — the Mission Creator and the debug benches
  that `workspaces/` documents.
- [Shared foundations](/apps/frontend/src/foundation/) — the transport, route table, session,
  primitives and utilities under Shared foundations.
- [Shared features](/apps/frontend/src/features/) — the mission review record.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md)
  and the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the route table in
  `apps/frontend/src/app_routes.rs` and `apps/frontend/src/foundation/route_table/mod.rs`, the sidebar
  in `apps/frontend/src/foundation/route_table/navigation_menu.rs` and the in-code READMEs, which
  this hub is written from.
- Used by: the documentation root README; the READMEs of `apps/`, the frontend crate and
  its `src/` tree; the API overview; the commit checklist and the ticket identifiers standard
  in `documentation/standards/`; tickets in `.ai/tickets/` that name it.
- Rules: every route in `ROUTES` has exactly one row in the route table, and a route change
  updates the table in the same commit; a documentation folder mirrors a code folder and keeps its
  spelling; feature docs, not this hub, hold behaviour, design and open work; the hub and its
  README indexes name no ticket.

## Related documentation

- [Page areas](/documentation/apps/frontend/pages/README.md) — the index of the eight page
  areas.
- [API overview](/documentation/apps/api/api_overview.md) — the routes of every API
  domain the pages call.
- [Documentation standards](/documentation/standards/documentation_standards.md#2-contracts-behind-the-tags)
  — how the API's models, the schemas and the frontend's wire types stay one contract.
- [Where does X go](/documentation/standards/where_does_x_go.md) — where new frontend code and
  documents belong.
- [Local development](/documentation/runbooks/local_development.md) — running the API and the
  app locally, the dev login included.
- [Editor gates](/documentation/runbooks/editor_gates.md) — the headless browser gates that
  drive the built app.
- [Archived frontend roadmap](/documentation/archive/go_and_react_era_design/frontend_roadmap.md)
  — the planning view and shipped log this hub replaces.
