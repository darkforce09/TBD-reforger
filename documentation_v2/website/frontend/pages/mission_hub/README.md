**Status:** live

# Mission hub pages

The documentation of the three [mission](/documentation_v2/glossary.md#mission) hub pages, one
folder per page: each holds the page's feature doc and, where a design set exists, its design
references. Developers and AI agents read it before changing the library, a mission's overview or
the review workspace.

## Contents

```text
documentation_v2/website/frontend/pages/mission_hub/
├── library/           the mission library page: browsing, the dossier sheet, creation and upload
├── overview/          the mission overview page: one mission's dossier and its armory editor
└── review_workspace/  the review workspace page: the Mission Creator read-only on an artifact
```

## How it works

The folders mirror the page folders under `apps/website/frontend/src/v2/pages/mission_hub/` and
keep their spelling. Each holds a README index and the page's feature doc; the library and the
overview also hold a `visual_references/` folder with one design-phase blueprint set. A feature doc
follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md): Where it
lives, Behaviour (ending in the known discrepancies between the page and the
[API](/documentation_v2/glossary.md#api)), Data (what each call means server-side), Design, Open
work and Decisions. Start with the feature doc of the page at hand.

Every page renders its content inside `AuthGate`, so its data shows only to a signed-in viewer.
Two code folders have no page of their own and no folder here: the New Mission dialog
(`create_dialog/`), which the library opens and whose behaviour the library's feature doc
describes, and the review record (`mission_review/`), which the library's dossier, the overview and
the [approvals](/documentation_v2/glossary.md#approvals) page render and whose in-code README
describes it.

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Mission library | `/missions`, `MissionLibraryPage` | Mission Library | [mission_library_page.md](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md) |
| Mission overview | `/missions/:id`, `MissionOverviewPage` | the mission's title; breadcrumb Mission Overview | [mission_overview_page.md](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md) |
| Review workspace | `/missions/:id/artifacts/:artifact_id/workspace`, `ReviewWorkspacePage` | "Review workspace of artifact …" banner | [review_workspace_page.md](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md) |

The Mission Creator itself, at `/missions/:id/edit`, is an app, documented under
[the editor documentation](/documentation_v2/website/frontend/apps/editor/README.md). A new mission
hub page gets a folder here named like its code folder, with a README and its feature doc, a line
in Contents and a row in the table.

## Code

- [Mission hub pages](/apps/website/frontend/src/v2/pages/mission_hub/) — the three route
  components, the create dialog and the review record, which the feature docs describe.
- [Missions domain](/apps/website/api_v2/src/missions/) — the missions, versions, armory, reviews
  and artifacts behind the three pages.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation_v2/glossary.md); the page code, the API handlers it calls and the
  ticket registry in `.ai/tickets/`, which the feature docs are written from.
- Used by: the in-code READMEs of the page folders, which link their feature docs under Related
  documentation; the missions domain README; the glossary's armory entry.
- Rules: one folder per routed page folder of the code, spelled the same; a page's feature doc is
  named after its route component and keeps its name, since the glossary and the READMEs link it;
  a feature doc stays within 500 lines; design references live only in `visual_references/`, and
  no document holds a screenshot of the built UI.

## Related documentation

- [Archived platform design spec](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md)
  — the design-phase specification of the mission library, which its feature doc compares against.
