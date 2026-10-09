**Status:** live

# Frontend documentation

The documentation hub of the web platform's single-page app: every browser route with the code
folder that renders it and the feature doc that describes it, the page crates, the full-screen
workspaces such as the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), and the
shared foundations they are built on. Developers and AI agents start here before changing a page.

## Contents

```text
documentation/crates/frontend/shell/frontend_application/
└── shell/  the app frame: the layout, the sidebar, the top bar and the not-found page
```

## How it works

The app (`crates/frontend/shell/frontend_application`) is the thin shell over the frontend crates, the top
of their crate order, so this tree is the app crate's documentation mirror and documents only what
the app holds: `shell/` documents `crates/frontend/shell/frontend_application/src/shell/`. Every page and
built workspace is a crate under `crates/frontend/`, and its feature docs sit in the crate's documentation mirror,
[frontend crate documentation](/documentation/crates/frontend/README.md):
`documentation/crates/frontend/<layer>/<crate>/<folder>/` documents the crate's
`src/<folder>/`. A page folder there holds a README index, the page's feature doc, named after its
route component (`event_schedule_page.md` for `EventSchedulePage`) and written from the
[feature doc template](/documentation/standards/templates/feature_doc.md), and, where a design set
exists, a `visual_references/` folder. The in-code README of each page
folder holds what the code declares (routes, calls with their DTOs, states with their exact text);
the feature doc holds the flows, what each call means in the [API](/documentation/glossary/a_to_f.md#api),
the design, the open work and the decisions. Open work lives only in the feature docs, as links to
tickets in `.ai/tickets/`.

The app is a Leptos 0.8 client-side-rendered app compiled to WebAssembly, which Trunk serves on
`127.0.0.1:3000` in development and proxies to the API; the crate README
[Frontend application](/crates/frontend/shell/frontend_application/README.md) has the build, the configuration
and the commands.

### Route table

`crates/frontend/shell/frontend_application/src/app_routes.rs` binds each path to its component, and the `ROUTES` table
in `crates/frontend/foundation/frontend_route_table/src/routes.rs` gives each path its access tier and layout flags; the
[route table README](/crates/frontend/foundation/frontend_route_table/README.md#public-surface) lists the
flags. A tier
is enforced in the browser after mount: `none` admits everyone, a `mission_maker` route sends a
viewer below that [role](/documentation/glossary/n_to_z.md#role) back to the
[mission](/documentation/glossary/g_to_m.md#mission) overview or the library, and an `admin` page
renders its own refusal through `AdminGate`. The rows below follow the sidebar's sections, as
`crates/frontend/foundation/frontend_route_table/src/navigation_menu.rs` orders them.

| Route | Component | Access | Code folder | Feature doc |
|---|---|---|---|---|
| `/login` | `LoginPage` | `none` | [account/login/](/crates/frontend/pages/account_pages/src/login/) | [account_pages.md](/documentation/crates/frontend/pages/account_pages/account_pages.md) |
| `/auth/callback` | `AuthCallbackPage` | `none` | [account/auth_callback/](/crates/frontend/pages/account_pages/src/auth_callback/) | [account_pages.md](/documentation/crates/frontend/pages/account_pages/account_pages.md) |
| `/settings` | `SettingsPage` | `none` | [account/settings/](/crates/frontend/pages/account_pages/src/settings/) | [account_pages.md](/documentation/crates/frontend/pages/account_pages/account_pages.md) |
| `/` | `DashboardPage` | `none` | [command_center/dashboard/](/crates/frontend/pages/command_center_pages/src/dashboard/) | [dashboard_page.md](/documentation/crates/frontend/pages/command_center_pages/dashboard/dashboard_page.md) |
| `/server-intel` | `ServerIntelPage` | `none` | [command_center/server_intel/](/crates/frontend/pages/command_center_pages/src/server_intel/) | [server_intel_page.md](/documentation/crates/frontend/pages/command_center_pages/server_intel/server_intel_page.md) |
| `/announcements` and `/announcements/:id` | `AnnouncementsPage` | `none` | [command_center/announcements/](/crates/frontend/pages/command_center_pages/src/announcements/) | [announcements_page.md](/documentation/crates/frontend/pages/command_center_pages/announcements/announcements_page.md) |
| `/events` | `EventSchedulePage` | `none` | [operations/schedule/](/crates/frontend/pages/operations_pages/src/schedule/) | [event_schedule_page.md](/documentation/crates/frontend/pages/operations_pages/schedule/event_schedule_page.md) |
| `/events/:id` | `EventHubPage` | `none` | [operations/event_detail/](/crates/frontend/pages/operations_pages/src/event_detail/) | [event_hub_page.md](/documentation/crates/frontend/pages/operations_pages/event_detail/event_hub_page.md) |
| `/events/:id/missions/:emid/orbat` | `OrbatSelectionPage` | `none` | [operations/orbat_selection/](/crates/frontend/pages/operations_pages/src/orbat_selection/) | [orbat_selection_page.md](/documentation/crates/frontend/pages/operations_pages/orbat_selection/orbat_selection_page.md) |
| `/deployments` | `DeploymentsPage` | `none` | [operations/deployments/](/crates/frontend/pages/operations_pages/src/deployments/) | [deployments_page.md](/documentation/crates/frontend/pages/operations_pages/deployments/deployments_page.md) |
| `/leaderboards` | `LeaderboardsPage` | `none` | [operations/leaderboards/](/crates/frontend/pages/operations_pages/src/leaderboards/) | [leaderboards_page.md](/documentation/crates/frontend/pages/operations_pages/leaderboards/leaderboards_page.md) |
| `/missions` | `MissionLibraryPage` | `none` | [mission_hub/library/](/crates/frontend/pages/mission_hub_pages/src/library/) | [mission_library_page.md](/documentation/crates/frontend/pages/mission_hub_pages/library/mission_library_page.md) |
| `/missions/:id` | `MissionOverviewPage` | `none` | [mission_hub/overview/](/crates/frontend/pages/mission_hub_pages/src/overview/) | [mission_overview_page.md](/documentation/crates/frontend/pages/mission_hub_pages/overview/mission_overview_page.md) |
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | [workspaces/editor/](/crates/frontend/workspaces/mission_creator_workspace/src/) | [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` | `mission_maker` | [workspaces/editor/review_workspace/](/crates/frontend/workspaces/mission_creator_workspace/src/review_workspace/) | [review_workspace_page.md](/documentation/crates/frontend/workspaces/mission_creator_workspace/review_workspace/review_workspace_page.md) |
| `/tools/mortar` | `MortarCalculatorPage` | `none` | [field_tools/mortar/](/crates/frontend/pages/field_tools_pages/src/mortar/) | [mortar_calculator_page.md](/documentation/crates/frontend/pages/field_tools_pages/mortar/mortar_calculator_page.md) |
| `/wiki` and `/wiki/:slug` | `WikiPage` | `none` | [doctrine_and_info/wiki/](/crates/frontend/pages/doctrine_pages/src/wiki/) | [wiki_page.md](/documentation/crates/frontend/pages/doctrine_pages/wiki/wiki_page.md) |
| `/vehicles` | `VehicleDatabasePage` | `none` | [doctrine_and_info/vehicles/](/crates/frontend/pages/doctrine_pages/src/vehicles/) | [vehicle_database_page.md](/documentation/crates/frontend/pages/doctrine_pages/vehicles/vehicle_database_page.md) |
| `/modpacks` | `ModpacksPage` | `none` | [doctrine_and_info/modpacks/](/crates/frontend/pages/doctrine_pages/src/modpacks/) | [modpacks_page.md](/documentation/crates/frontend/pages/doctrine_pages/modpacks/modpacks_page.md) |
| `/admin/events` | `EventManagerPage` | `admin` | [administration/event_manager/](/crates/frontend/pages/administration_pages/src/event_manager/) | [event_manager_page.md](/documentation/crates/frontend/pages/administration_pages/event_manager/event_manager_page.md) |
| `/admin/approvals` | `MissionApprovalsPage` | `admin` | [administration/approvals/](/crates/frontend/pages/administration_pages/src/approvals/) | [mission_approvals_page.md](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md) |
| `/admin/server` | `ServerControlPage` | `admin` | [administration/server_control/](/crates/frontend/pages/administration_pages/src/server_control/) | [server_control_page.md](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md) |
| `/admin/personnel` | `PersonnelRosterPage` | `admin` | [administration/personnel/](/crates/frontend/pages/administration_pages/src/personnel/) | [personnel_roster_page.md](/documentation/crates/frontend/pages/administration_pages/personnel/personnel_roster_page.md) |
| `/admin/content` | `ContentManagerPage` | `admin` | [administration/content_manager/](/crates/frontend/pages/administration_pages/src/content_manager/) | [content_manager_page.md](/documentation/crates/frontend/pages/administration_pages/content_manager/content_manager_page.md) |
| `/admin/audit` | `AuditLogsPage` | `admin` | [administration/audit_logs/](/crates/frontend/pages/administration_pages/src/audit_logs/) | [audit_logs_page.md](/documentation/crates/frontend/pages/administration_pages/audit_logs/audit_logs_page.md) |
| `/admin/ballistics-catalogs` | `BallisticsCatalogsPage` | `admin` | [administration/ballistics_catalogs/](/crates/frontend/pages/administration_pages/src/ballistics_catalogs/) | [ballistics_catalogs_page.md](/documentation/crates/frontend/pages/administration_pages/ballistics_catalogs/ballistics_catalogs_page.md) |
| `/debug/building-viewer` | `BuildingViewerPage` | `none` | [workspaces/debug/building_viewer/](/crates/frontend/workspaces/debug_benches/src/building_viewer/) | [building_viewer_page.md](/documentation/crates/frontend/workspaces/debug_benches/building_viewer_page.md) |
| `/debug/world-los` | `WorldLosPage` | `none` | [workspaces/debug/world_los/](/crates/frontend/workspaces/debug_benches/src/world_los/) | [world_los_page.md](/documentation/crates/frontend/workspaces/debug_benches/world_los_page.md) |
| `/debug/ballistics-agreement` | `BallisticsAgreementPage` | `none` | [workspaces/debug/ballistics_agreement/](/crates/frontend/workspaces/debug_benches/src/ballistics_agreement/) | [ballistics_agreement_page.md](/documentation/crates/frontend/workspaces/debug_benches/ballistics_agreement_page.md) |
| any other path | `NotFoundPage` | `none` | [shell/](/crates/frontend/shell/frontend_application/src/shell/) | [app_layout_and_navigation.md](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md) |

Page folders sit in the page crates under `crates/frontend/pages/`, workspace folders in the
workspace crates under `crates/frontend/workspaces/`, the frame under
`crates/frontend/shell/frontend_application/src/shell/`.
Notes on the table:

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

### Page crates and workspaces

The pages are seven crates, one per navigation area: the six sidebar sections, in the order
`crates/frontend/foundation/frontend_route_table/src/navigation_menu.rs` declares them, and the
account pages, which sit outside the sidebar. Each crate's documentation mirror indexes its pages
and holds their feature docs:

| Page crate | Sidebar section | Routes | Start at |
|---|---|---|---|
| `command_center_pages` | [Command Center](/documentation/glossary/a_to_f.md#command-center) | `/`, `/server-intel`, `/announcements`, `/announcements/:id` | [Command center pages](/documentation/crates/frontend/pages/command_center_pages/README.md) |
| `operations_pages` | [Operations](/documentation/glossary/n_to_z.md#operations) | `/events`, `/events/:id`, `/events/:id/missions/:emid/orbat`, `/deployments`, `/leaderboards` | [Operations pages](/documentation/crates/frontend/pages/operations_pages/README.md) |
| `mission_hub_pages` | Mission Hub | `/missions`, `/missions/:id` | [Mission hub pages](/documentation/crates/frontend/pages/mission_hub_pages/README.md) |
| `field_tools_pages` | Field Tools | `/tools/mortar` | [Field tools pages](/documentation/crates/frontend/pages/field_tools_pages/README.md) |
| `doctrine_pages` | Doctrine & Info | `/wiki`, `/wiki/:slug`, `/vehicles`, `/modpacks` | [Doctrine and info pages](/documentation/crates/frontend/pages/doctrine_pages/README.md) |
| `administration_pages` | [Administration](/documentation/glossary/a_to_f.md#administration), for the `admin` [role](/documentation/glossary/n_to_z.md#role) | the seven `/admin/*` routes | [Administration pages](/documentation/crates/frontend/pages/administration_pages/README.md) |
| `account_pages` | none; `/settings` is in the top bar's account menu | `/login`, `/auth/callback`, `/settings` | [Account pages](/documentation/crates/frontend/pages/account_pages/README.md) |

One mission hub code folder renders no route of its own, the New Mission dialog
(`create_dialog/`); the mission hub README says which feature doc describes it. The review record
the mission hub, the approvals queue and the review workspace share is the `mission_review_record`
feature crate. The seven administration routes declare the `admin` tier and each of
their pages also wraps its body in `AdminGate`; the signed-in pages put their data behind
`AuthGate`, so a signed-out viewer sees a sign-in prompt in its place.

[shell/](/documentation/crates/frontend/shell/frontend_application/shell/README.md) documents the
app frame. The built workspaces are crates too: the
[Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md),
starting from its [roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md),
and the [debug benches documentation](/documentation/crates/frontend/workspaces/debug_benches/README.md).
The [workspace crate documentation](/documentation/crates/frontend/workspaces/README.md) also
holds the two planned workspaces, the mission planner and the after-action review, which have no
code and serve no route.

### Shared foundations

The foundation crates under `crates/frontend/foundation/` and the feature crates under
`crates/frontend/features/` carry no route and no documentation folder here: their in-code
READMEs describe them exactly, and the deeper documents are linked from them.

| Module | What it holds | README |
|---|---|---|
| `crates/frontend/foundation/` | the foundation crates every layer shares, and their crate order | [Frontend foundation crates](/crates/frontend/foundation/README.md) |
| `frontend_transport` (crate) | the HTTP client, the typed endpoint calls, the token provider the session implements, and the live server status and audit log streams over [SSE](/documentation/glossary/n_to_z.md#sse) | [Frontend transport](/crates/frontend/foundation/frontend_transport/README.md) |
| `frontend_api_dtos` (crate) | the wire types with the typed identifiers and the five-tier role ladder (`guest`, `enlisted`, `leader`, `mission_maker`, `admin`) | [Frontend API DTOs](/crates/frontend/foundation/frontend_api_dtos/README.md) |
| `frontend_session` (crate) | the session store, the cross-tab session refresh, the sign-out hooks, the route guard and the two content gates | [Frontend session](/crates/frontend/foundation/frontend_session/README.md) |
| `frontend_ui` (crate) | the interface primitives (the icon, page header, status pill, search box, select, slider and their hover and disabled classes, split pane, toasts, dialog and sheet) and the timestamps, UTC instants, countdown, avatar sanitiser and clipboard write | [Frontend UI](/crates/frontend/foundation/frontend_ui/README.md) |
| `frontend_route_table` (crate) | every route's path, component name, layout flags and access tier, and the sidebar's menu | [Route table](/crates/frontend/foundation/frontend_route_table/README.md) |
| `frontend_map_view` (crate) | the shared map seam: canvas sizing, engine creation, the camera fit, the frame pump, resize tracking, pointer navigation in map metres and the 2 m ground heights | [Frontend map view](/crates/frontend/foundation/frontend_map_view/README.md) |
| `frontend_offline` (crate) | the offline service worker registration, the offline pack download, its saved copies and the page-wide pack status | [Frontend offline core](/crates/frontend/foundation/frontend_offline/README.md) |
| `frontend_test_support` (crate) | the repository-root finder and the captured API responses, for tests only | [Frontend test support](/crates/frontend/foundation/frontend_test_support/README.md) |
| `mission_review_record` (crate) | a mission's review history, thread, provenance and submit control, shared by the mission hub, the approvals queue and the review workspace | [Mission review record](/crates/frontend/features/mission_review_record/README.md) |

The wire types in `frontend_api_dtos` follow the API's models, and the API wins a disagreement; the
golden round trips in `crates/frontend/foundation/frontend_api_dtos/src/tests/` hold the two together.

### Design

The live design tokens are `crates/frontend/shell/frontend_application/style/aegis.css`, which Tailwind
CSS compiles into the build, and the [design tokens](/documentation/design_system/design_tokens.md)
reference describes them. A page's `visual_references/` sets are design-phase references kept for
colour and layout context, never an implementation source: the built UI is the Leptos code, and
each feature doc's Design section lists how the page differs from its set.

### Adding a page

A new route gets a row in `crates/frontend/shell/frontend_application/src/app_routes.rs` and in `ROUTES`, an in-code
README in its page folder inside its page crate, a folder in that crate's documentation mirror
under `documentation/crates/frontend/pages/<crate>/`, named like the code folder, with a README and
its feature doc, and a row in the route table above.

## Code

- [Frontend application](/crates/frontend/shell/frontend_application/) — the app crate: build files and the Aegis
  stylesheet.
- [Frontend source root](/crates/frontend/shell/frontend_application/src/) — the entry point, the render form of
  the route table and the app frame.
- [App shell](/crates/frontend/shell/frontend_application/src/shell/) — the frame that `shell/` documents.
- [Frontend shell crates](/crates/frontend/shell/README.md) — the shell layer: the app and the
  offline service worker beside it.
- [Frontend page crates](/crates/frontend/pages/README.md) — the routed pages, one crate per
  navigation area.
- [Frontend workspace crates](/crates/frontend/workspaces/README.md) — the Mission Creator and the
  debug benches.
- [Frontend foundation crates](/crates/frontend/foundation/README.md) — the transport, route
  table, session, offline pack, primitives and utilities under Shared foundations.
- [Frontend feature crates](/crates/frontend/features/README.md) — the mission review record.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md)
  and the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the route table in
  `crates/frontend/shell/frontend_application/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`, the sidebar in
  `crates/frontend/foundation/frontend_route_table/src/navigation_menu.rs` and the in-code READMEs,
  which this hub is written from.
- Used by: the documentation root README; the READMEs of `crates/frontend/`, the app crate and
  its `src/` tree; the API overview; the commit checklist and the ticket identifiers standard
  in `documentation/standards/`; tickets in `.ai/tickets/` that name it.
- Rules: every route in `ROUTES` has exactly one row in the route table, and a route change
  updates the table in the same commit; a documentation folder mirrors a code folder and keeps its
  spelling; feature docs, not this hub, hold behaviour, design and open work; the hub and its
  README indexes name no ticket.

## Related documentation

- [Frontend crate documentation](/documentation/crates/frontend/README.md) — the feature docs of
  the page and workspace crates.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — the routes of every API
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
