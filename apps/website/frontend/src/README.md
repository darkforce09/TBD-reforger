# Frontend source root

The source root of the `website-frontend` crate: the WebAssembly entry point that mounts the app,
the route table in its two forms, and the `v2/` tree that holds every page, workspace and shared
foundation.

## Contents

```text
apps/website/frontend/src/
├── app_routes.rs  `AppRoutes`: each path bound to the component that renders it, and the fallback
├── main.rs        the binary: `start_app`, the WebAssembly start function that mounts the app
├── router.rs      `ROUTES`: each route's layout flags and access tier, and their readers
├── tests/         unit tests for the route table's access tiers and redirects
└── v2/            the pages, the workspaces and the shared foundations
```

## How it works

`start_app` in `main.rs` is a `#[wasm_bindgen(start)]` function rather than the binary's `main`,
because the map engine registers a start function of its own and wasm-bindgen runs every
registered start but never a binary's `main`. It installs the panic hook and mounts
`<div id="root"><Router><AppLayout/></Router></div>` into the body; `AppLayout`, in
`v2/pages/navigation/`, provides the session store and the toast queue, restores a stored
session, and renders `AppRoutes` inside the frame the route asks for. On a native build `main` is
empty and nothing mounts.

The route table exists twice. `app_routes.rs` is the render form: the `<Routes>` that binds each
path to its component, with `NotFoundPage` as the fallback. `router.rs` is the contract: each
`RouteDef` in `ROUTES` names the path, the component, the `full_bleed` and `chromeless` layout
flags and the `auth` tier. A path is matched segment by segment, a `:param` segment matching any
value, and five readers use the match: `breadcrumb` for the top bar, `full_bleed` and `chromeless`
for the frame, and `role_may_enter` and `auth_denial_redirect` for the route guard in
`v2/core/auth/`. A path added to `app_routes.rs` without a row in `ROUTES` renders padded, with no
breadcrumb and no tier.

Access tiers are enforced in the browser after mount, since the server answers every app path with
the same page. An open route (`none`) and an unmatched path admit everyone; a declared tier admits
a signed-in viewer whose [role](/documentation_v2/glossary.md#role) clears it and never an
anonymous one; a tier the ladder does not know refuses everyone. A refused `mission_maker` route
under `/missions/:id/` redirects to `/missions/:id?role_notice=mission_maker`, and any other to
`/missions?role_notice=mission_maker`; a refused `admin` route stays put and its page renders the
refusal through `AdminGate`.

## Public surface

The routes the app answers, as `router.rs` declares them. `/login` and `/auth/callback` render in
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
| `/settings` | `SettingsPage` | `none` | padded |
| `/admin/events` | `EventManagerPage` | `admin` | padded |
| `/admin/approvals` | `MissionApprovalsPage` | `admin` | full-bleed |
| `/admin/server` | `ServerControlPage` | `admin` | full-bleed |
| `/admin/personnel` | `PersonnelRosterPage` | `admin` | full-bleed |
| `/admin/content` | `ContentManagerPage` | `admin` | full-bleed |
| `/admin/audit` | `AuditLogsPage` | `admin` | full-bleed |
| any other path | `NotFoundPage` | `none` | padded |

## Boundaries

- Depends on: `leptos` and `leptos_router` for the router, `wasm-bindgen` for the start function,
  and `console_error_panic_hook`; within the folder, the root files take the route components from
  `v2/pages/` and `v2/apps/`, and `Role` and `has_min_role_authed` from `v2/core/auth/`.
- Used by: `apps/website/frontend/index.html`, whose `rust` link has Trunk build this binary;
  the route drift gate in `tools_v2/developer-tools/src/browser_testing/route_drift.rs`, which
  reads `router.rs`; the headless browser gates of `tools_v2/developer-tools/src/browser_testing/`,
  which drive the built app by its routes.
- Rules:
  - `app_routes.rs` and `ROUTES` list the same paths, `ROUTES` adding the fallback as its `*` row;
    no unit test compares the two;
  - every route declares a recognised tier (`every_route_declares_a_recognised_tier`), a
    misdeclared tier refuses every viewer (`a_misdeclared_tier_denies_every_viewer`), and each
    `mission_maker` route redirects to its [mission](/documentation_v2/glossary.md#mission)'s
    overview (`denial_redirects_to_overview_with_role_notice` and
    `the_review_workspace_declares_mission_maker_and_redirects_to_its_mission`), all in
    `tests/route_authorization.rs`;
  - `cargo run -q -p developer-tools --bin gate -- s-routes` diffs `ROUTES` against the manifest
    `tools_v2/developer-tools/fixtures/dom_oracle/manifests/routes.csv`, so a route change updates
    that manifest too.

## Related documentation

- [App layout and navigation](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md)
  — the frame, the sidebar, the top bar and the not-found page.
