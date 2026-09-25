**Status:** live

# Mission Creator documentation

Everything written about the [Mission Creator](/documentation_v2/glossary.md#mission-creator), the
workspace in which mission makers build a [mission](/documentation_v2/glossary.md#mission) on a
top-down 2D map: what it does, how it should look and answer, where it is going, why it is built
the way it is, and the Arma 3 Eden editor it is measured against. Developers and AI agents start
here before changing the editor.

## Contents

```text
documentation_v2/website/frontend/apps/editor/
├── arsenal/                     the Arsenal tab, the one-slot loadout editor, and its design references
├── decisions.md                 the decisions log: each design decision, its context and its effects
├── eden_editor_reference/       the Arma 3 Eden editor catalogs and the gap analysis against them
├── feature_inventory/           every editor feature by area, with its state in the committed code
├── mission_creator_roadmap.md   what ships by area, the open and deferred work, and open questions
├── ux_spec.md                   the layout, pointer gestures, keys, and the load and save flow
└── visual_references/           the design-phase mock-ups of the editor shell, canvas and diff view
```

## How it works

The documents split one editor into layers, each with one job. Read the one that answers the
question at hand:

| Question | Document |
|---|---|
| What does the editor do today, feature by feature, and in which code? | [feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md), one area per file |
| How is it laid out, and what does each gesture, key and save step do? | [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) |
| What is open, in which order, and what is deferred? | [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md) |
| Why is it built this way? | [decisions log](/documentation_v2/website/frontend/apps/editor/decisions.md) |
| What does Eden do, and how close is the editor to it? | [Eden reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/README.md) |
| How does one [slot](/documentation_v2/glossary.md#slot)'s loadout get edited? | [arsenal](/documentation_v2/website/frontend/apps/editor/arsenal/README.md) |
| What did the design phase draw? | [visual references](/documentation_v2/website/frontend/apps/editor/visual_references/README.md) |

The feature inventory and the UX specification describe the committed code and win over every
other document here when they disagree with it; the code wins over both. The editor follows the
layout and interactions of the Eden editor, styled with the Aegis glass tokens; the mock-ups under
`visual_references/` and `arsenal/visual_references/` are design-phase references that settle
styling only, never layout or behaviour.

The editor has one route, `/missions/:id/edit`, for the `mission_maker`
[role](/documentation_v2/glossary.md#role) and above, full-bleed and chromeless; the mission hub's
review workspace mounts the same page read-only. The editor README's
[Routes](/apps/website/frontend/src/v2/apps/editor/README.md#routes) gives both.

A new document about the editor goes in the folder of the layer it belongs to: a feature area file
in `feature_inventory/`, an Eden catalog in `eden_editor_reference/`, a design set in
`visual_references/`; a new folder here gets a line in Contents and a row in the table.

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the editor page, its docks,
  dialogs, input, canvas bridge, browser session and the Arsenal, which every document here
  describes.
- [Mission data](/apps/website/map-engine/src/data/) and
  [editing](/apps/website/map-engine/src/editing/) in the map engine — the mission document, its
  compiler and validation, and the editing commands and tools the editor drives.
- [Missions domain](/apps/website/api_v2/src/missions/) — the mission versions the editor loads
  and saves, and the item [registry](/documentation_v2/glossary.md#registry) the palettes and the
  Arsenal read.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md),
  the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md)
  and the [glossary](/documentation_v2/glossary.md); the editor code, the map engine's data and
  editing modules, the missions API and the ticket registry in `.ai/tickets/`, which the documents
  are written from; the Bohemia wiki's Eden pages, which the Eden catalogs cite.
- Used by: the glossary's Mission Creator entry; the documentation root README; the frontend,
  full-screen workspaces, mission hub, review workspace and map-engine documentation; the in-code
  READMEs of `apps/website/frontend/src/v2/`, `apps/website/frontend/src/v2/apps/`, the editor
  folder, the Arsenal and the review workspace page, which link here under Related documentation;
  the app README template, whose sample links here.
- Rules: every Mission Creator document lives under this folder; prose says Mission Creator and
  mission, and quotes code identifiers such as `scenario` as spelled; the feature inventory's IDs
  are never reused; open work is listed from `.ai/tickets/` with each ticket's status, never from
  memory.

## Related documentation

- [Full-screen workspaces documentation](/documentation_v2/website/frontend/apps/README.md) — the
  editor beside the debug benches and the planned planner and after-action review.
- [Review workspace page](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md)
  — the read-only review mode that mounts the editor.
- [Map engine documentation](/documentation_v2/website/map-engine/README.md) — the engine the
  editor drives.
- [Editor gates runbook](/documentation_v2/runbooks/editor_gates.md) — the headless browser gates
  that drive the editor route.
- [Design tokens](/documentation_v2/design_system/design_tokens.md) — the Aegis tokens the editor
  chrome uses.
