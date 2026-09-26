**Status:** live

# Mission overview page documentation

The feature documentation of the `/missions/:id` page, one
[mission](/documentation_v2/glossary/g_to_m.md#mission)'s standalone dossier with its Edit Armory dialog
and review record, with the design-phase reference of the dossier it shares with the library.

## Contents

```text
documentation_v2/website/frontend/pages/mission_hub/overview/
├── mission_overview_page.md  the feature doc: the dossier, the armory editor and the API
└── visual_references/        the design-phase blueprint of a slide-over mission dossier
```

## How it works

Read [mission_overview_page.md](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, explains why the
[armory](/documentation_v2/glossary/a_to_f.md#armory) editor picks faction keys from the
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) and saves the whole armory at once, and compares the
built dossier with the blueprint in `visual_references/`. The blueprint is a design-phase
reference drawn as a slide-over: it lists required assets and an order of battle, while the built
dossier shows a faction-tabbed armory and no order of battle. The code folder's README lists the
page's files.

## Code

- [Mission overview page](/apps/website/frontend/src/v2/pages/mission_hub/overview/) — the route
  component `MissionOverviewPage`, the shared dossier body and the Edit Armory dialog.
- [Missions domain](/apps/website/api_v2/src/missions/) — the mission detail, armory and review
  routes the page calls.

## Boundaries

- Depends on: the feature doc template; the page code, the missions handlers and the ticket
  registry the feature doc is written from.
- Used by: the [armory](/documentation_v2/glossary/a_to_f.md#armory) glossary entry, the in-code READMEs
  of the overview, the review record and the mission hub, and the missions domain README, which
  link the feature doc; the mission hub pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md)
  — the library whose slide-over renders the same dossier body.
- [Missions domain](/apps/website/api_v2/src/missions/README.md) — the API side of the dossier and
  the armory.
