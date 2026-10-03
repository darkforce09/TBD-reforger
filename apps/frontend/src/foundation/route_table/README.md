# Route table

The contract of every route the single-page app answers, and the sidebar's navigation menu: each
route's path, component name, layout flags and access tier, the readers the frame and the route
guard use over them, and the sections and links the sidebar draws.

## Contents

```text
apps/frontend/src/foundation/route_table/
├── mod.rs              `ROUTES`: each route's layout flags and access tier, and their readers
├── navigation_menu.rs  `NAVIGATION`: the sidebar's sections and links, each with its lowest role
└── tests/              unit tests for the route table's access tiers and redirects
```

## How it works

The route table exists twice. `apps/frontend/src/app_routes.rs` is the render form: the
`<Routes>` that binds each path to its component, with `NotFoundPage` as the fallback. `mod.rs`
here is the contract: each `RouteDef` in `ROUTES` names the path, the component, the `full_bleed`
and `chromeless` layout flags and the `auth` tier. A path is matched segment by segment, a
`:param` segment matching any value, and five readers use the match: `breadcrumb` for the top bar,
`full_bleed` and `chromeless` for the frame, and `role_may_enter` and `auth_denial_redirect` for
the route guard in `foundation/auth/`. A path added to `app_routes.rs` without a row in `ROUTES`
renders padded, with no breadcrumb and no tier. The `component` field is a name, so the table
depends on no page.

Access tiers are enforced in the browser after mount, since the server answers every app path with
the same page. An open route (`none`) and an unmatched path admit everyone; a declared tier admits
a signed-in viewer whose [role](/documentation/glossary/n_to_z.md#role) clears it and never an
anonymous one; a tier the ladder does not know refuses everyone. A refused `mission_maker` route
under `/missions/:id/` redirects to `/missions/:id?role_notice=mission_maker`, and any other to
`/missions?role_notice=mission_maker`; a refused `admin` route stays put and its page renders the
refusal through `AdminGate`.

`navigation_menu.rs` is data only: its sections render top to bottom in the order written, every
item names the lowest role that sees it, and its `path` values match `ROUTES`, since the sidebar
marks the active link by comparing them with the live pathname by prefix.

## Public surface

The routes the app answers, as `ROUTES` declares them. `/login` and `/auth/callback` render in
the bare frame, which the app layout names by path; a chromeless route fills the viewport without
the sidebar and top bar; a full-bleed route's content area does not scroll, and every other route
renders padded in a scrolling one.

| Path | Component | Access | Layout |
|---|---|---|---|
| `/login` | `LoginPage` | `none` | bare |
| `/auth/callback` | `AuthCallbackPage` | `none` | bare |
| `/` | `DashboardPage` | `none` | full-bleed |
| `/server-intel` | `ServerIntelPage` | `none` | full-bleed |
| `/announcements` and `/announcements/:id` | `AnnouncementsPage` | `none` | full-bleed |
| `/deployments` | `DeploymentsPage` | `none` | full-bleed |
| `/leaderboards` | `LeaderboardsPage` | `none` | full-bleed |
| `/missions` | `MissionLibraryPage` | `none` | full-bleed |
| `/missions/:id` | `MissionOverviewPage` | `none` | padded |
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | full-bleed, chromeless |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` | `mission_maker` | full-bleed, chromeless |
| `/events` | `EventSchedulePage` | `none` | full-bleed |
| `/events/:id` | `EventHubPage` | `none` | full-bleed |
| `/events/:id/missions/:emid/orbat` | `OrbatSelectionPage` | `none` | padded |
| `/wiki` and `/wiki/:slug` | `WikiPage` | `none` | full-bleed |
| `/vehicles` | `VehicleDatabasePage` | `none` | full-bleed |
| `/modpacks` | `ModpacksPage` | `none` | full-bleed |
| `/tools/mortar` | `MortarCalculatorPage` | `none` | full-bleed |
| `/debug/building-viewer` | `BuildingViewerPage` | `none` | full-bleed, chromeless |
| `/debug/world-los` | `WorldLosPage` | `none` | full-bleed, chromeless |
| `/debug/ballistics-agreement` | `BallisticsAgreementPage` | `none` | full-bleed, chromeless |
| `/settings` | `SettingsPage` | `none` | padded |
| `/admin/events` | `EventManagerPage` | `admin` | padded |
| `/admin/approvals` | `MissionApprovalsPage` | `admin` | full-bleed |
| `/admin/server` | `ServerControlPage` | `admin` | full-bleed |
| `/admin/personnel` | `PersonnelRosterPage` | `admin` | full-bleed |
| `/admin/content` | `ContentManagerPage` | `admin` | full-bleed |
| `/admin/audit` | `AuditLogsPage` | `admin` | full-bleed |
| `/admin/ballistics-catalogs` | `BallisticsCatalogsPage` | `admin` | padded |
| any other path | `NotFoundPage` | `none` | padded |

`NAVIGATION`, `NavSection` and `NavItem` are read by the sidebar in `apps/frontend/src/shell/`.

## Boundaries

- Depends on: `Role` and `has_min_role_authed` from `foundation/transport/dto/role.rs`.
- Used by: the route guard in `foundation/auth/`; the frame, the top bar and the sidebar in
  `apps/frontend/src/shell/`; the route drift gate in
  `tools/browser_testing/browser_gate_suites/src/route_drift.rs`, which reads `mod.rs`; the headless
  browser gates of `tools/developer_tools/src/browser_testing/`, which drive the built app by its
  routes.
- Rules:
  - `app_routes.rs` and `ROUTES` list the same paths, `ROUTES` adding the fallback as its `*` row;
    no unit test compares the two;
  - every route declares a recognised tier (`every_route_declares_a_recognised_tier`), a
    misdeclared tier refuses every viewer (`a_misdeclared_tier_denies_every_viewer`), and each
    `mission_maker` route redirects to its [mission](/documentation/glossary/g_to_m.md#mission)'s
    overview (`denial_redirects_to_overview_with_role_notice` and
    `the_review_workspace_declares_mission_maker_and_redirects_to_its_mission`), all in
    `tests/route_authorization.rs`;
  - `cargo run -q -p developer_tools --bin gate -- s-routes` diffs `ROUTES` against the manifest
    `tools/browser_testing/browser_gate_suites/fixtures/dom_oracle/manifests/routes.csv`, so a route change updates
    that manifest too.

## Related documentation

- [App layout and navigation](/documentation/apps/frontend/shell/app_layout_and_navigation.md)
  — the frame, the sidebar, the top bar and the not-found page.
- [Frontend documentation](/documentation/apps/frontend/README.md) — each route with its code
  folder and feature doc.
