# Platform pages

Every standard page of the web app, grouped by the sidebar section its route belongs to. A page is
a folder whose route component the route table mounts, with one file per panel it renders; the
frame they render inside lives in `apps/frontend/src/shell/`, the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), its review workspace and the
other full-screen workspaces in `apps/frontend/src/workspaces/`, and the review record the mission
hub shares in `apps/frontend/src/features/`.

## Contents

```text
apps/frontend/src/pages/
├── account/            sign-in, the sign-in callback and the viewer's settings
├── administration/     the seven administrator-only screens under `/admin/*`
├── command_center/     the landing dashboard, the server intel panel and the announcement board
├── doctrine_and_info/  the doctrine wiki, the vehicle database and the modpack manifests
├── field_tools/        the standalone tactical aids: the mortar calculator
├── mission_hub/        the mission library, the mission overview and the create dialog
├── mod.rs              the module tree
└── operations/         the event schedule and hub, ORBAT selection, deployments and leaderboards
```

## How it works

`apps/frontend/src/main.rs` mounts `AppLayout` from `apps/frontend/src/shell/`, which draws the frame
and renders `AppRoutes` from `apps/frontend/src/app_routes.rs` inside it. Each route names
a page component in one of these folders; `apps/frontend/src/foundation/route_table/mod.rs` declares, per
path, the access tier the route guard applies and the full-bleed and chromeless layout flags the
frame reads, and the breadcrumb the top bar shows.

| Sidebar section | Folder | Routes |
|---|---|---|
| "Command Center" | `command_center/` | `/`, `/server-intel`, `/announcements`, `/announcements/:id` |
| "Operations" | `operations/` | `/events`, `/events/:id`, `/events/:id/missions/:emid/orbat`, `/deployments`, `/leaderboards` |
| "Mission Hub" | `mission_hub/` | `/missions`, `/missions/:id` |
| "Field Tools" | `field_tools/` | `/tools/mortar` |
| "Doctrine & Info" | `doctrine_and_info/` | `/wiki`, `/wiki/:slug`, `/vehicles`, `/modpacks` |
| "Administration" | `administration/` | `/admin/events`, `/admin/approvals`, `/admin/server`, `/admin/personnel`, `/admin/content`, `/admin/audit`, `/admin/ballistics-catalogs` |
| none | `account/` | `/login`, `/auth/callback`, `/settings` |

The seven [administration](/documentation/glossary/a_to_f.md#administration) routes declare the `admin`
tier and each of their pages also wraps its body in `AdminGate`; the Mission Creator route
`/missions/:id/edit` and the review workspace declare `mission_maker`; every other route declares
`none`. The signed-in pages put their data behind `AuthGate`, so a signed-out viewer sees a sign-in
prompt in its place. The Mission Creator, the review workspace and the debug benches are
chromeless: they render without the sidebar and the top bar.

## Public surface

- The route components `apps/frontend/src/app_routes.rs` mounts, one set per area; each
  area's README lists its own.

## Boundaries

- Depends on: `crate::foundation` (the [API](/documentation/glossary/a_to_f.md#api) client and DTOs, the
  session and route guard, the UI primitives, the utilities), `crate::features` (the mission review
  record); no workspace.
- Used by: the route table in `apps/frontend/src/app_routes.rs`; nothing under
  `apps/frontend/src/foundation/` or `apps/frontend/src/features/` imports from here.
- Rules: a page may import from `foundation`, `features` and the map engine, never a workspace or
  another page area, and no lower layer imports from a page; `cargo xtask verify
  frontend-layering` holds every such import at zero. A path added to
  `apps/frontend/src/app_routes.rs` needs its row in `apps/frontend/src/foundation/route_table/mod.rs`,
  or it renders with the default layout and no tier; every row must declare a recognised tier
  (`every_route_declares_a_recognised_tier` in
  `apps/frontend/src/foundation/route_table/tests/route_authorization.rs`).

## Related documentation

- [Administration pages](/documentation/apps/frontend/pages/administration/README.md) — the
  administrator screens' feature docs.
- [App layout and navigation](/documentation/apps/frontend/shell/app_layout_and_navigation.md)
  — the frame, the sidebar and the top bar.
- [Account pages](/documentation/apps/frontend/pages/account/account_pages.md) — sign-in, the
  callback and settings.
