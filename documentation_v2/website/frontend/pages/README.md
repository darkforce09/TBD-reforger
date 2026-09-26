**Status:** live

# Page areas

The documentation of the app's routed pages, one folder per navigation area: the six sidebar
sections, the account pages and the navigation frame around every page. Developers and AI agents
open the area of the page at hand; the [frontend documentation](/documentation_v2/website/frontend/README.md)
maps every route to its folder and feature doc.

## Contents

```text
documentation_v2/website/frontend/pages/
├── account/            the account pages: sign-in, the Discord auth callback and settings
├── administration/     the six administration pages: events, approvals, servers, people, content, audit
├── command_center/     the command center pages: dashboard, server intel and announcements
├── doctrine_and_info/  the reference pages: doctrine wiki, vehicle database and modpacks
├── field_tools/        the standalone tactical aids: the mortar calculator
├── mission_hub/        the mission hub pages: library, overview and review workspace
├── navigation/         the navigation frame: layout, sidebar, top bar and the not-found page
└── operations/         the operations pages: schedule, event hub, ORBAT, deployments, leaderboards
```

## How it works

The folders mirror the area folders under `apps/website/frontend/src/v2/pages/` and keep their
spelling. Inside an area, one folder per page folder of the code holds a README index, the page's
feature doc, named after its route component and written from the
[feature doc template](/documentation_v2/standards/templates/feature_doc.md), and, where a design
set exists, a `visual_references/` folder of design-phase sets. The account pages and the
navigation frame are one feature doc each, `account_pages.md` and `app_layout_and_navigation.md`,
covering every page folder of their area. Start with the area README, then the feature doc of the
page at hand.

The sidebar, declared in `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`, lists six
sections in this order; the account pages and the not-found page sit outside it.

| Area | Sidebar section | Routes | Start at |
|---|---|---|---|
| `command_center/` | Command Center | `/`, `/server-intel`, `/announcements`, `/announcements/:id` | [Command center pages](/documentation_v2/website/frontend/pages/command_center/README.md) |
| `operations/` | Operations | `/events`, `/events/:id`, `/events/:id/missions/:emid/orbat`, `/deployments`, `/leaderboards` | [Operations pages](/documentation_v2/website/frontend/pages/operations/README.md) |
| `mission_hub/` | Mission Hub | `/missions`, `/missions/:id`, `/missions/:id/artifacts/:artifact_id/workspace` | [Mission hub pages](/documentation_v2/website/frontend/pages/mission_hub/README.md) |
| `field_tools/` | Field Tools | `/tools/mortar` | [Field tools pages](/documentation_v2/website/frontend/pages/field_tools/README.md) |
| `doctrine_and_info/` | Doctrine & Info | `/wiki`, `/wiki/:slug`, `/vehicles`, `/modpacks` | [Doctrine and info pages](/documentation_v2/website/frontend/pages/doctrine_and_info/README.md) |
| `administration/` | Administration, for the `admin` [role](/documentation_v2/glossary/n_to_z.md#role) | `/admin/events`, `/admin/approvals`, `/admin/server`, `/admin/personnel`, `/admin/content`, `/admin/audit` | [Administration pages](/documentation_v2/website/frontend/pages/administration/README.md) |
| `account/` | none; `/settings` is in the top bar's account menu | `/login`, `/auth/callback`, `/settings` | [account_pages.md](/documentation_v2/website/frontend/pages/account/account_pages.md) |
| `navigation/` | none; the frame around every route | any path no route matches | [app_layout_and_navigation.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md) |

The [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) at `/missions/:id/edit` and
the debug benches are workspaces, not pages; their documentation sits under
`documentation_v2/website/frontend/apps/`. Two mission hub code folders render no route of their
own, the New Mission dialog (`create_dialog/`) and the review record (`mission_review/`); the
mission hub README says which feature docs describe them.

A new area gets a folder here named like its code folder, with a README index, a line in Contents
and a row in the table; a new page gets a folder in its area, as the area README says.

## Code

- [Pages](/apps/website/frontend/src/v2/pages/) — the area folders and route components the
  feature docs describe.
- [Sidebar configuration](/apps/website/frontend/src/v2/pages/navigation/nav_config.rs) — the
  sections and links the table follows.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md)
  and the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation_v2/glossary/README.md); the page code under
  `apps/website/frontend/src/v2/pages/`, the route table in `apps/website/frontend/src/router.rs`
  and the ticket registry in `.ai/tickets/`, which the feature docs are written from.
- Used by: the [frontend documentation](/documentation_v2/website/frontend/README.md), which
  indexes the areas; the area READMEs and feature docs below this folder; the in-code page READMEs
  that link their feature docs.
- Rules: one folder per area folder of the code, spelled the same; a page's feature doc is named
  after its route component and keeps its name, since the glossary and the in-code READMEs link
  it; design references live only in `visual_references/`, and no document holds a screenshot of
  the built UI.

## Related documentation

- [Frontend documentation](/documentation_v2/website/frontend/README.md) — the route table and the
  shared foundations.
- [Design tokens](/documentation_v2/design_system/design_tokens.md) — the tokens every page is
  drawn with.
