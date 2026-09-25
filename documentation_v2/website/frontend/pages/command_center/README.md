**Status:** live

# Command center pages

The documentation of the three [command center](/documentation_v2/glossary.md#command-center)
pages, the first section of the sidebar, one folder per page: each holds the page's feature doc
and, where a design set exists, its design references. Developers and AI agents read it before
changing a command center page.

## Contents

```text
documentation_v2/website/frontend/pages/command_center/
├── announcements/  the announcements page: the published feed beside a reading pane
├── dashboard/      the dashboard page: next event, server, assignment, modpack and newest news
└── server_intel/   the server intel page: one game server's live panel
```

## How it works

The folders mirror the page folders under `apps/website/frontend/src/v2/pages/command_center/`
and keep their spelling. Each holds a README index and the page's feature doc; the dashboard and
server intel folders add a `visual_references/` folder with one design-phase blueprint set. A
feature doc follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
Where it lives, Behaviour (ending in the known discrepancies between the page and the
[API](/documentation_v2/glossary.md#api)), Data (what each call means server-side), Design (the
layout as built and each difference from the design target), Open work and Decisions. Start with
the feature doc of the page at hand.

Every page is declared in `apps/website/frontend/src/router.rs` with the route tier `none`, full
bleed inside the navigation frame, and renders its body inside `AuthGate`
(`apps/website/frontend/src/v2/core/ui/gates.rs`), so only a signed-in viewer sees data. The pages
only read: none of them writes platform data.

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Dashboard | `/`, `DashboardPage` | Dashboard | [dashboard_page.md](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md) |
| Server intel | `/server-intel`, `ServerIntelPage` | Server Intel | [server_intel_page.md](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md) |
| Announcements | `/announcements` and `/announcements/:id`, `AnnouncementsPage` | Announcements; heading "Comms Link" | [announcements_page.md](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md) |

A new command center page gets a folder here named like its code folder, with a README, its
feature doc and, when a design set exists, a `visual_references/` folder; it also gets a line in
Contents and a row in the table.

## Code

- [Command center pages](/apps/website/frontend/src/v2/pages/command_center/) — the three route
  components and their panels, which the feature docs describe.
- [Command center domain](/apps/website/api_v2/src/command_center/) — the dashboard payload.
- [Community content domain](/apps/website/api_v2/src/community_content/) — the announcement feed
  and the current modpack.
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/) — the server
  list and its [SSE](/documentation_v2/glossary.md#sse) status stream.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation_v2/glossary.md); the page code, the API handlers it calls and the
  ticket registry in `.ai/tickets/`, which the feature docs are written from.
- Used by: the in-code READMEs of the page folders, which link their feature docs under Related
  documentation; the API READMEs that link the feature doc of the page they serve; the web app
  README's page table in `documentation_v2/website/frontend/`.
- Rules: one folder per page folder of the code, spelled the same; a page's feature doc is named
  after its route component (`server_intel_page.md` for `ServerIntelPage`) and keeps its name,
  since the READMEs link it; a feature doc stays within 500 lines; design references live only in
  `visual_references/`, and no document holds a screenshot of the built UI.

## Related documentation

- [Archived platform design spec](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md)
  — the design-phase specification of the server intel and announcements pages, which the feature
  docs compare against.
