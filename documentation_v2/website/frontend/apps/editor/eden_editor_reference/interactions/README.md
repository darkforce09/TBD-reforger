**Status:** live

# Eden editor interactions

The interactions catalog of the Arma 3 Eden editor, the reference design the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) is measured against: every Eden
interaction as an ID with its wiki source, one topic per file. Developers and AI agents read it to
learn what Eden does before building or judging the Mission Creator's counterpart.

## Contents

```text
documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/
├── asset_browser.md                        modes and F-keys, side submode, search, crew switch
├── compositions.md                         custom compositions: save, edit, place, Workshop
├── connections.md                          grouping, sync, trigger owner, random start, waypoints
├── entity_placement.md                     click, drag, repeat, area, comment, empty vehicle
├── selection_layers_and_attributes.md      selection, layers, Attributes dialog, formation menu
├── toolbar_keys_actions_and_status_bar.md  toolbar index, shortcuts, engine actions, status bar
├── transformation.md                       move, altitude, rotate, vertical mode, snap, widgets
└── vehicle_crew.md                         crew panel, boarding, unboarding, seat changes
```

## How it works

Each topic file groups the entries of one or two ID domains. An entry is a
`#### {ID} — {Short name}` heading over a field table, in the Eden reference format of the
[feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries):
the UI surface, the wiki anchor it was read from, the shortcut, trigger and procedure, and the
Acceptance checks that show a feature behaves as Eden's does. Short entries carry only the fields
their wiki section supports; the index tables of the last two files give an ID, a summary and the
wiki page. Every fact comes from the Bohemia wiki's Eden pages, cited by URL; the 28 scraped pages
are in `.ai/artifacts/eden-wiki/`, listed by the
[scrape manifest](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_wiki_scrape_manifest.yaml).

Eden edits in a 3D scene and on a 2D map. The Mission Creator's map view is top-down, drawn
through the map engine's orthographic camera (`apps/website/map-engine/src/camera/ortho/`), so an
entry that exists only in 3D is marked `N/A (3D)`. Parity with the Mission Creator is not stated
here: the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
pairs each ID with the Mission Creator's feature by ID, and the
[feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
says what the Mission Creator does.

| Topic file | ID domains | IDs | Mission Creator area in the feature inventory |
|---|---|---:|---|
| [asset_browser.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/asset_browser.md) | `RIGHT` | 13 | [right asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) |
| [entity_placement.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/entity_placement.md) | `PLACE` | 7 | [placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) |
| [transformation.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/transformation.md) | `XFORM`, `WIDGET` | 11 | [transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md) |
| [vehicle_crew.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/vehicle_crew.md) | `CREW` | 4 | [attributes and settings](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) (the vehicle view's crew seats) |
| [compositions.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/compositions.md) | `COMP` | 5 | [right asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) (the Compositions tab) |
| [connections.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/connections.md) | `CONN` | 8 | none yet; the code is the [map context menu](/apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/README.md) |
| [selection_layers_and_attributes.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/selection_layers_and_attributes.md) | `SEL`, `LAYER`, `ATTR`, `CTX` | 12 | [selection](/documentation_v2/website/frontend/apps/editor/feature_inventory/selection.md), [left sidebar](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md), [attributes and settings](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) |
| [toolbar_keys_actions_and_status_bar.md](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/toolbar_keys_actions_and_status_bar.md) | `TOOLBAR`, `KEY`, `ACTION`, `STATUS` | 23 | [top command strip](/documentation_v2/website/frontend/apps/editor/feature_inventory/top_command_strip.md), [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md), [bottom toolbelt](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md) |

The folder defines 83 IDs, each in one file. The gap analysis counts them with:

```bash
cd documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions
grep -ohE '\b[A-Z][A-Z0-9]*(-[A-Z0-9]+)*-[0-9]{3}\b' *.md | sort -u | wc -l
```

A new Eden entry goes into the topic file of its domain, with the next free number in its
pattern and a wiki anchor, or `UNVERIFIED` in its Evidence field when no source confirms it; a new
domain gets a new topic file, a Contents line and a table row.

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the workspace whose features the
  gap analysis pairs with these entries.
- [Map context menu](/apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/) — the
  Mission Creator's connect, formation and comment rows, the counterpart of the connection and
  context menu entries.

## Boundaries

- Depends on: the Eden pages of the Bohemia wiki (`https://community.bistudio.com/wiki/`) and
  their scrape in `.ai/artifacts/eden-wiki/`; the Eden reference format and ID patterns of the
  [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md).
- Used by: the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md),
  which gives each ID a parity row; the feature inventory's README and entry schema; the
  [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md); the
  [UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md)
  and [attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md)
  beside it; the context menu README in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/`; and the ticket registry,
  whose tickets name these IDs.
- Rules: an ID is never renumbered, reused or moved between domains, because the gap analysis,
  the roadmap and the tickets cite it verbatim; each ID is defined in exactly one file, and the
  count above stays equal to the gap analysis's interaction rows; every fact cites its wiki
  anchor; a file stays within 500 lines (`cargo xtask verify markdown-placement`) and has a
  Contents line (`cargo xtask verify readme-coverage`).

## Related documentation

- [Eden editor reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/README.md)
  — the catalogs, the gap analysis and the scrape manifest together.
- [Eden terminology](https://community.bistudio.com/wiki/Eden_Editor:_Terminology) — Eden's own
  terms: entity, asset, Entity List, layer, group, connection and view.
