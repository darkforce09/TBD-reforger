**Status:** live

# Dashboard page documentation

The feature documentation of the `/` page, the landing screen of the
[command center](/documentation_v2/glossary/a_to_f.md#command-center), where a member sees the next
[event](/documentation_v2/glossary/a_to_f.md#event), a game server's status, their own assignment, the
current modpack and the newest announcements, with the page's design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/command_center/dashboard/
├── dashboard_page.md   the feature doc: the panels, what the API composes, and the design
└── visual_references/  the design-phase blueprint of the banner, three cards and feed
```

## How it works

Read [dashboard_page.md](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what each part of the one
[API](/documentation_v2/glossary/a_to_f.md#api) answer means server-side, lists where the page and the API
disagree, and compares the built page with the blueprint in `visual_references/`. The code
folder's README lists the page's files, its call and its states.

## Code

- [Dashboard page](/apps/website/frontend/src/v2/pages/command_center/dashboard/) — the route
  component `DashboardPage` and its five panels.
- [Command center domain](/apps/website/api_v2/src/command_center/) — `GET /api/v1/dashboard`,
  which composes the page's one payload.

## Boundaries

- Depends on: the feature doc template; the page code, the dashboard handler and the ticket
  registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the page's in-code README and the command center pages README, which link the feature
  doc; the web app README's page table in `documentation_v2/website/frontend/`.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Command center domain](/apps/website/api_v2/src/command_center/README.md) — the API side of the
  dashboard.
