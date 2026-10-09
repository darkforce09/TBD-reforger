**Status:** live

# Mission Creator documentation

Everything written about the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the
workspace in which mission makers build a [mission](/documentation/glossary/g_to_m.md#mission) on a
top-down 2D map: what it does, how it should look and answer, where it is going, why it is built
the way it is, and the Arma 3 Eden editor it is measured against. Developers and AI agents start
here before changing the editor.

## Contents

```text
documentation/crates/frontend/workspaces/mission_creator_workspace/
├── decisions.md                 the decisions log: each design decision, its context and its effects
├── eden_editor_reference/       the Arma 3 Eden editor catalogs and the gap analysis against them
├── feature_inventory/           every editor feature by area, with its state in the committed code
├── mission_creator_roadmap.md   what ships by area, the open and deferred work, and open questions
├── review_workspace/            the review workspace page: the Mission Creator read-only on an artifact
├── ux_spec.md                   the layout, pointer gestures, keys, and the load and save flow
└── visual_references/           the design-phase mock-ups of the editor shell, canvas and diff view
```

## How it works

The documents split one editor into layers, each with one job. Read the one that answers the
question at hand:

| Question | Document |
|---|---|
| What does the editor do today, feature by feature, and in which code? | [feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md), one area per file |
| How is it laid out, and what does each gesture, key and save step do? | [UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) |
| What is open, in which order, and what is deferred? | [roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md) |
| Why is it built this way? | [decisions log](/documentation/crates/frontend/workspaces/mission_creator_workspace/decisions.md) |
| What does Eden do, and how close is the editor to it? | [Eden reference](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/README.md) |
| How does one [slot](/documentation/glossary/n_to_z.md#slot)'s loadout get edited? | [arsenal](/documentation/crates/frontend/workspaces/mission_creator_arsenal/README.md) |
| What did the design phase draw? | [visual references](/documentation/crates/frontend/workspaces/mission_creator_workspace/visual_references/README.md) |

The feature inventory and the UX specification describe the committed code and win over every
other document here when they disagree with it; the code wins over both. The editor follows the
layout and interactions of the Eden editor, styled with the Aegis glass tokens; the mock-ups under
`visual_references/` and the Arsenal's
[visual references](/documentation/crates/frontend/workspaces/mission_creator_arsenal/visual_references/README.md) are design-phase references that settle
styling only, never layout or behaviour.

The editor has one route, `/missions/:id/edit`, for the `mission_maker`
[role](/documentation/glossary/n_to_z.md#role) and above, full-bleed and chromeless; the mission hub's
review workspace mounts the same page read-only. The editor README's
[Routes](/crates/frontend/workspaces/mission_creator_workspace/src/README.md#routes) gives both.

A new document about the editor goes in the folder of the layer it belongs to: a feature area file
in `feature_inventory/`, an Eden catalog in `eden_editor_reference/`, a design set in
`visual_references/`; a new folder here gets a line in Contents and a row in the table.

## Code

- [Mission Creator](/crates/frontend/workspaces/mission_creator_workspace/src/) — the editor page, its docks,
  dialogs, input, canvas bridge, browser session and the Arsenal, which every document here
  describes.
- [Mission crates](/crates/mission/) and the
  [mission editing crates](/crates/mission_editing/) — the mission document, its compiler and
  validation, and the editing commands and tools the editor drives.
- [Missions domain](/crates/api/api_missions/src/) — the mission versions the editor loads
  and saves, and the item [registry](/documentation/glossary/n_to_z.md#registry) the palettes and the
  Arsenal read.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md),
  the [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md)
  and the [glossary](/documentation/glossary/README.md); the editor code, the map engine's data and
  editing modules, the missions API and the ticket registry in `.ai/tickets/`, which the documents
  are written from; the Bohemia wiki's Eden pages, which the Eden catalogs cite.
- Used by: the glossary's Mission Creator entry; the documentation root README; the frontend,
  full-screen workspaces, mission hub, review workspace and map-engine documentation; the in-code
  READMEs of `crates/frontend/shell/frontend_application/src/`, `crates/frontend/workspaces/`, the editor
  folder, the Arsenal and the review workspace page, which link here under Related documentation;
  the app README template, whose sample links here.
- Rules: every Mission Creator document lives under this folder; prose says Mission Creator and
  mission, and quotes code identifiers such as `scenario` as spelled; the feature inventory's IDs
  are never reused; open work is listed from `.ai/tickets/` with each ticket's status, never from
  memory.

## Related documentation

- [Workspace crate documentation](/documentation/crates/frontend/workspaces/README.md) — the editor
  beside the debug benches and the planned planner and after-action review.
- [Review workspace page](/documentation/crates/frontend/workspaces/mission_creator_workspace/review_workspace/review_workspace_page.md)
  — the read-only review mode that mounts the editor.
- [Map rendering documentation](/documentation/crates/map_rendering/README.md) — the renderer the
  editor drives.
- [Editor gates runbook](/documentation/runbooks/editor_gates.md) — the headless browser gates
  that drive the editor route.
- [Design tokens](/documentation/design_system/design_tokens.md) — the Aegis tokens the editor
  chrome uses.
