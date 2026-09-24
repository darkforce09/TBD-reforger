# Platform pages

Every standard page of the web app, grouped by the sidebar section its route belongs to, and the
persistent navigation frame they render inside. A page is a folder whose route component the
route table mounts, with one file per panel it renders; the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) and the other full-screen
workspaces live in `apps/website/frontend/src/v2/apps/` instead.

## Contents

```text
apps/website/frontend/src/v2/pages/
├── account/            sign-in, the Discord sign-in callback and the viewer's settings
├── administration/     the six administrator-only screens under `/admin/*`
├── command_center/     the landing dashboard, the server intel panel and the announcement board
├── doctrine_and_info/  the doctrine wiki, the vehicle database and the modpack manifests
├── field_tools/        the standalone tactical aids: the mortar calculator
├── mission_hub/        the mission library and overview, the review workspace and review record
├── mod.rs              the module tree
├── navigation/         the persistent frame: sidebar, top bar, navigation registry, not-found page
└── operations/         the event schedule and hub, ORBAT selection, deployments and leaderboards
```

## How it works

`apps/website/frontend/src/main.rs` mounts `AppLayout` from `navigation/`, which draws the frame
and renders `AppRoutes` from `apps/website/frontend/src/app_routes.rs` inside it. Each route names
a page component in one of these folders; `apps/website/frontend/src/router.rs` declares, per
path, the access tier the route guard applies and the full-bleed and chromeless layout flags the
frame reads, and the breadcrumb the top bar shows.

| Sidebar section | Folder | Routes |
|---|---|---|
| "Command Center" | `command_center/` | `/`, `/server-intel`, `/announcements`, `/announcements/:id` |
| "Operations" | `operations/` | `/events`, `/events/:id`, `/events/:id/missions/:emid/orbat`, `/deployments`, `/leaderboards` |
| "Mission Hub" | `mission_hub/` | `/missions`, `/missions/:id`, `/missions/:id/artifacts/:artifact_id/workspace` |
| "Field Tools" | `field_tools/` | `/tools/mortar` |
| "Doctrine & Info" | `doctrine_and_info/` | `/wiki`, `/wiki/:slug`, `/vehicles`, `/modpacks` |
| "Administration" | `administration/` | `/admin/events`, `/admin/approvals`, `/admin/server`, `/admin/personnel`, `/admin/content`, `/admin/audit` |
| none | `account/` | `/login`, `/auth/callback`, `/settings` |
| none | `navigation/` | the not-found page for any path no route matches |

The six [administration](/documentation_v2/glossary.md#administration) routes declare the `admin`
tier and each of their pages also wraps its body in `AdminGate`; the Mission Creator route
`/missions/:id/edit` and the review workspace declare `mission_maker`; every other route declares
`none`. The signed-in pages put their data behind `AuthGate`, so a signed-out viewer sees a sign-in
prompt in its place. The Mission Creator, the review workspace and the debug benches are
chromeless: they render without the sidebar and the top bar.

## Public surface

- `navigation::layout::AppLayout`: the frame `apps/website/frontend/src/main.rs` mounts.
- The route components `apps/website/frontend/src/app_routes.rs` mounts, one set per area; each
  area's README lists its own.

## Boundaries

- Depends on: `crate::v2::core` (the [API](/documentation_v2/glossary.md#api) client and DTOs, the
  session and route guard, the UI primitives, the utilities), the map engine
  (`website_map_engine`), and the workspaces under `apps/website/frontend/src/v2/apps/`, which the
  review workspace and the mission library import.
- Used by: `apps/website/frontend/src/main.rs` (`AppLayout`) and the route table in
  `apps/website/frontend/src/app_routes.rs`; nothing under `apps/website/frontend/src/v2/core/`
  imports from here.
- Rules: a page may import from `core`, the map engine and the apps, and nothing under `core` may
  import from a page; no test enforces this. A path added to
  `apps/website/frontend/src/app_routes.rs` needs its row in `apps/website/frontend/src/router.rs`,
  or it renders with the default layout and no tier; every row must declare a recognised tier
  (`every_route_declares_a_recognised_tier` in
  `apps/website/frontend/src/tests/route_authorization.rs`).

## Related documentation

- [Administration pages](/documentation_v2/website/frontend/pages/administration/README.md) — the
  administrator screens' feature docs.
- [App layout and navigation](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md)
  — the frame, the sidebar and the top bar.
- [Account pages](/documentation_v2/website/frontend/pages/account/account_pages.md) — sign-in, the
  callback and settings.
