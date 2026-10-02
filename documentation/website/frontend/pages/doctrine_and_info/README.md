**Status:** live

# Doctrine and info pages

The documentation of the three reference pages members consult, one folder per page: the doctrine
wiki, the vehicle database and the modpacks. Each folder holds the page's feature doc and, where a
design set exists, its design references. Developers and AI agents read it before changing one of
these pages.

## Contents

```text
documentation/website/frontend/pages/doctrine_and_info/
├── modpacks/  the modpacks page: server modpacks, their addons and their administration
├── vehicles/  the vehicle database page: faction-grouped vehicles, dossiers and their editing
└── wiki/      the doctrine wiki page: manuals, their editing and revisions
```

## How it works

The folders mirror the page folders under `apps/website/frontend/src/v2/pages/doctrine_and_info/`
and keep their spelling. Each holds a README index and the page's feature doc; the vehicle
database and the modpacks also hold a `visual_references/` folder of design-phase sets. A feature
doc follows the [feature doc template](/documentation/standards/templates/feature_doc.md): Where
it lives, Behaviour (ending in the known discrepancies between the page and the
[API](/documentation/glossary/a_to_f.md#api), where there are any), Data (what each call means
server-side), Design, Open work and Decisions.

The three pages share one shape: each renders inside `AuthGate`, fetches its whole list from the
[community content](/documentation/glossary/a_to_f.md#community-content) domain once, and lays it out
in a `GlassSplit`, a searchable list beside the selected item; the wiki and vehicle answers decode
into typed DTOs. Each page gives an administrator its writes: the wiki edits a manual and keeps its
revision history, saving against the revision the edit started from and restoring an older
revision; the vehicle database adds, edits and deletes vehicles; the modpacks page edits the packs.

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Doctrine wiki | `/wiki` and `/wiki/:slug`, `WikiPage` | SOPs & Manuals | [wiki_page.md](/documentation/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md) |
| Vehicle database | `/vehicles`, `VehicleDatabasePage` | Vehicle Database | [vehicle_database_page.md](/documentation/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md) |
| Modpacks | `/modpacks`, `ModpacksPage` | Modpacks | [modpacks_page.md](/documentation/website/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md) |

A new page in this sidebar section gets a folder here named like its code folder, with a README
and its feature doc, a line in Contents and a row in the table.

## Code

- [Doctrine and info pages](/apps/website/frontend/src/v2/pages/doctrine_and_info/) — the three
  route components, which the feature docs describe.
- [Community content domain](/apps/website/api_v2/src/community_content/) — the wiki, vehicle
  database and modpack routes behind them.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation/glossary/README.md); the page code, the API handlers it calls and the
  ticket registry in `.ai/tickets/`, which the feature docs are written from.
- Used by: the in-code READMEs of the page folders and of `doctrine_and_info/`, which link the
  feature docs under Related documentation.
- Rules: one folder per page folder of the code, spelled the same; a feature doc keeps its name,
  since the READMEs link it; a feature doc stays within 500 lines; design references live only in
  `visual_references/`, and no document holds a screenshot of the built UI.

## Related documentation

- [Archived platform design spec](/documentation/archive/go_and_react_era_design/platform_context_handoff.md)
  — the design-phase specification of the wiki and the modpacks, which their feature docs compare
  against.
