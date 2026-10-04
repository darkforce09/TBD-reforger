# Frontend page crates

The pages layer of the frontend crates: one crate per sidebar section, each holding the route
components of that section's pages and the panels they are built from.

## Contents

```text
crates/frontend/pages/
├── account_pages/  `account_pages`: sign-in, the sign-in callback and the account settings with the Arma identity link
├── administration_pages/  `administration_pages`: events, approvals, server control, personnel, content, audit trail, ballistics catalogs
├── command_center_pages/  `command_center_pages`: the landing dashboard, the server intel panel and the announcement board
├── doctrine_pages/  `doctrine_pages`: the doctrine wiki, the vehicle database and the modpack manifests
├── field_tools_pages/  `field_tools_pages`: the mortar calculator with its map picker, saved fire missions and offline line
├── mission_hub_pages/  `mission_hub_pages`: the mission library and its dossier, the mission overview, the New Mission dialog
└── operations_pages/  `operations_pages`: the event schedule and hub, ORBAT selection, deployments and leaderboards
```

## How it works

A page crate fetches through the foundation crates, renders the feature crates' shared views, and
exports its route components; the app's route table (`apps/frontend/src/app_routes.rs`) mounts
them. The route components fetch in the browser only, so they exist on `wasm32`; the pure readers
under them compile on every target, so their tests run natively.

## Public surface

- `account_pages`: `LoginPage`, `AuthCallbackPage` and `SettingsPage`; see its README.
- `administration_pages`: `EventManagerPage`, `MissionApprovalsPage`, `ServerControlPage`,
  `PersonnelRosterPage`, `ContentManagerPage`, `AuditLogsPage` and `BallisticsCatalogsPage`; see
  its README.
- `command_center_pages`: `DashboardPage`, `ServerIntelPage` and `AnnouncementsPage`; see its
  README.
- `doctrine_pages`: `WikiPage`, `VehicleDatabasePage` and `ModpacksPage`; see its README.
- `field_tools_pages`: `MortarCalculatorPage`; see its README.
- `mission_hub_pages`: `MissionLibraryPage` and `MissionOverviewPage`; see its README.
- `operations_pages`: `EventSchedulePage`, `EventHubPage`, `OrbatSelectionPage`,
  `DeploymentsPage` and `LeaderboardsPage`; see its README.

## Boundaries

- Depends on: the foundation crates under `crates/frontend/foundation/` and the feature crates
  under `crates/frontend/features/`.
- Used by: the app (`apps/frontend`), whose route table mounts the pages.
- Rules: a page crate never depends on another page crate, a workspace or the app (layer order
  foundation < features < pages, workspaces < shell, `cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Frontend crates](/crates/frontend/README.md) — the layer order every frontend crate follows.
- [Frontend source root](/apps/frontend/src/README.md) — the app's entry point, route table and
  frame, and the layer order of the frontend crates.
