# Frontend route table

The `frontend_route_table` crate: the contract of every route the single-page app answers, and the
sidebar's navigation menu. It holds each route's path, component name, layout flags and access
tier, the readers the frame and the route guard use over them, and the sections and links the
sidebar draws.

## Contents

```text
crates/frontend/foundation/frontend_route_table/
├── Cargo.toml  the package: `frontend_api_dtos` for the role ladder, layout tier 8, any target
└── src/        the `ROUTES` table and its readers, the sidebar's `NAVIGATION`, their tests
```

## How it works

The route table exists twice. `crates/frontend/shell/frontend_application/src/app_routes.rs` is the render form: the
`<Routes>` that binds each path to its component, with `NotFoundPage` as the fallback.
`src/routes.rs` here is the contract: each `RouteDef` in `ROUTES` names the path, the component,
the `full_bleed` and `chromeless` layout flags and the `auth` tier. A path is matched segment by
segment, a `:param` segment matching any value, and five readers use the match: `breadcrumb` for
the top bar, `full_bleed` and `chromeless` for the frame, and `role_may_enter` and
`auth_denial_redirect` for the session's route guard. A path added to `app_routes.rs` without a
row in `ROUTES` renders padded, with no breadcrumb and no tier. The `component` field is a name,
so the crate depends on no page.

Access tiers are enforced in the browser after mount, since the server answers every app path with
the same page. An open route (`none`) and an unmatched path admit everyone; a declared tier admits
a signed-in viewer whose [role](/documentation/glossary/n_to_z.md#role) clears it and never an
anonymous one; a tier the ladder does not know refuses everyone. A refused `mission_maker` route
under `/missions/:id/` redirects to `/missions/:id?role_notice=mission_maker`, and any other to
`/missions?role_notice=mission_maker`; a refused `admin` route stays put and its page renders the
refusal through `AdminGate`.

`src/navigation_menu.rs` is data only: its sections render top to bottom in the order written,
every item names the lowest role that sees it, and its `path` values match `ROUTES`, since the
sidebar marks the active link by comparing them with the live pathname by prefix.

Everything compiles on every target: the native tests read the same tables the browser build does.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_route_table   # the access tiers, the open routes and the redirects
```

## Configuration

None: no feature, no environment variable.

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

- `routes`: `ROUTES`, `RouteDef` and the readers `breadcrumb`, `full_bleed`, `chromeless`,
  `role_may_enter` and `auth_denial_redirect`, all re-exported at the crate root.
- `navigation_menu`: `NAVIGATION`, `NavSection` and `NavItem`, read by the sidebar in
  `crates/frontend/shell/frontend_application/src/shell/`.
- `prelude`: `RouteDef` and the five readers.

## Boundaries

- Depends on: `frontend_api_dtos` (`Role` and `has_min_role_authed` from its `role` module).
- Used by: the route guard of the session (`crates/frontend/foundation/frontend_session/src/`); the frame, the
  top bar and the sidebar in `crates/frontend/shell/frontend_application/src/shell/`; the headless browser gates of `tools/browser_testing/browser_gate_suites/`, which drive
  the built app by its routes.
- Rules:
  - `app_routes.rs` and `ROUTES` list the same paths, `ROUTES` adding the fallback as its `*` row;
    no unit test compares the two;
  - every route declares a recognised tier (`every_route_declares_a_recognised_tier`), a
    misdeclared tier refuses every viewer (`a_misdeclared_tier_denies_every_viewer`), and each
    `mission_maker` route redirects to its [mission](/documentation/glossary/g_to_m.md#mission)'s
    overview (`denial_redirects_to_overview_with_role_notice` and
    `the_review_workspace_declares_mission_maker_and_redirects_to_its_mission`), all in
    `src/tests/route_authorization.rs`;
  - the crate depends on no frontend crate above `frontend_api_dtos`
    (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [App layout and navigation](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md)
  — the frame, the sidebar, the top bar and the not-found page.
- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) — each route with its code
  folder and feature doc.
