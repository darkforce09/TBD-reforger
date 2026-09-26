**Status:** live

# Feature entry schema (FEDS)

The rules every feature entry of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
inventory and of the Eden editor reference follows: how a feature is named and numbered, what an
entry holds, the terms both catalogues share, and the row format of the Eden gap analysis, which
links the two by ID. An entry exists so that a developer or an agent can build, check or compare
a feature without guessing.

## Feature IDs

An ID reads `{DOMAIN}-{SUBDOMAIN}-{NNN}`: the domain from the table below, a short grouping token
(`VIEW`, `LAYER`, `TAB`, …) and a zero-padded number, `001` to `999`, per subdomain. One behaviour
takes one ID: a click select, a marquee and a Shift-add are three entries, never one "selection"
entry. An ID is never reused or renumbered, because the gap analysis and other documents cite it.

| Domain | Scope |
|---|---|
| `SHELL` | route, layout, chrome, load lifecycle |
| `MAP` | map viewport, grid, camera, cursor, basemap, world objects |
| `SEL` | selection: click, marquee, modifiers |
| `XFORM` | move, rotate, snap, align, delete |
| `PLACE` | placing from the palette, spawn defaults |
| `LEFT` | left dock: editor layers, Locations, the ORBAT tree |
| `RIGHT` | asset palette and browser (`RIGHT-MODE`, `RIGHT-SEARCH` subdomains) |
| `TOP` | top command strip: menus, save, export, settings |
| `BOTTOM` | toolbelt, status bar and its read-outs |
| `ATTR` | the Attributes dialog and entity properties |
| `KEY` | keyboard shortcuts |
| `DATA` | persistence, hydrate, compile, collaboration |
| `PERF` | behaviour at scale |
| `FILE` | route loading; in Eden, the new, open and save menus |
| `ENV` | time, weather, view distance, fog |
| `ORBAT` | factions, squads, slots as the game receives them |
| `LAYER` | editor layers, the workflow folders |
| `TBD` | features Eden has no counterpart for |
| `WP`, `TRG`, `MRK`, `VEH` | waypoints, triggers and modules, markers, vehicles |
| `MEAS` | ruler, elevation, distance |
| `CONN` | connections: in Eden, its connection types; in the Mission Creator, the Connect flow under Eden's IDs plus the map lines, the Connections panel and the graph findings (`CONN-LINE-001`, `CONN-PANEL-001`, `CONN-VALID-001`) |
| `COMP`, `TOOLBAR`, `WIDGET`, `CTX`, `MENU` | Eden compositions, toolbar buttons, transformation widget, context menu entries, menu bar items |

The Eden reference adds these patterns: `RIGHT-MODE-00N` (the asset browser's F1 to F6 modes),
`RIGHT-SEARCH-00N` (search syntax), `COMP-00N`, `TOOLBAR-00N`, `WIDGET-00N`, `CTX-00N`,
`CONN-{TYPE}-00N` (a connection type, such as `CONN-GROUP-001`), `MENU-{MENU}-{ITEM}`,
`ACTION-{NAME}` (an engine action from the
[Eden actions](https://community.bistudio.com/wiki/Eden_Editor:_Actions) page) and
`ATTR-FIELD-{TYPE}-{NAME}` (one attribute field, such as `ATTR-FIELD-OBJ-TYPE`). Attribute
fields are rows of a table in the [Eden attributes reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md),
and the interactions reference cites them by ID rather than repeating them.

## Mission Creator inventory entries

Each area file of the [inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
is a [feature doc](/documentation_v2/standards/templates/feature_doc.md): Where it lives,
Behaviour, Data, Design, Open work and Decisions. Its entries sit in Behaviour:

1. A table with one row per ID: the ID, the feature in a few words, and its status from the
   legend below.
2. One `###` heading per ID, or per small group of IDs, reading `{ID} — {short name}`, then
   numbered steps that say what the mission maker does and what the code does in reply, with
   interface text quoted exactly as the code writes it and the limits that apply. The code paths
   go in Where it lives and in the steps; the in-code README of each folder carries the detail and
   is linked, not repeated.
3. A closing `### Known discrepancies`: each place where interface text, a code comment or a
   README says one thing and the code does another, with the file on both sides.

| Status | Meaning |
|---|---|
| shipped | the committed code does what the entry says |
| partial | part of the entry works; the steps say which part does not |
| not built | nothing in the code does it, or a visible control has no action |

A status is read from the committed code, never from a ticket. An entry that the code shows to
have changed keeps its ID and gets new steps; a feature the code has and the inventory lacks gets
a new ID, noted under the area's table.

## Eden reference entries

Each Eden feature in the [interactions reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md)
is a `#### {ID} — {Short name}` heading over a field table:

| Field | Holds |
|---|---|
| Domain | the ID's domain and subdomain, such as `SEL-MAP` |
| UI Surface | `MenuBar`, `Toolbar`, `AssetBrowser`, `EntityList`, `View`, `AttributesDialog`, `ScenarioAttributes`, `ContextMenu`, `StatusBar`, `ConnectionLine` or `—` |
| Feature kind | `interaction`, `ui_chrome`, `attribute_field`, `connection_type`, `browser_mode` or `engine_action` |
| Wiki anchor | the full URL with the `#Section_Heading` it was read from |
| Shortcut | the key combination, or `—` |
| Goal | the outcome for the user |
| Trigger | the exact event, with explicit `Ctrl`, `Shift`, `Alt`, `LMB`, `RMB`, `MMB` |
| Preconditions | tool, selection and view state |
| Procedure | numbered steps, as the wiki describes them |
| Postconditions | the state after success |
| Inputs, Outputs | buttons, keys and modifiers; the visible feedback |
| Edge cases | cancel paths, limits, conflicts |
| Acceptance | at least one `- [ ]` check, three or more for a complex feature |
| Evidence | the wiki URL; `UNVERIFIED` and what to check when no source confirms it |
| Parent ID | optional grouping |

An Eden entry is never guessed: without a cited source its Evidence reads `UNVERIFIED`.

## Terms

Both catalogues keep these terms apart; the document keys are those of the mission document in
`apps/website/map-engine/src/data/store/rows/construction.rs`.

| Term | Meaning in the Mission Creator |
|---|---|
| Entity | any placed mission object: the document's `slots`, `vehicles`, `entities` (world objects), `zones`, `triggers`, `comments`, `connections`, `compositions`, `objectives` and `markers` maps |
| Slot | a placed unit in the `slots` map, filed in one squad's `slotIds` and in at most one editor layer's `entityIds`; the saved payload's `editor.slots[]`, the compiled mission's `slots[]` |
| ORBAT | faction → squad → slot in the document and the saved payload's `editor` block; the export's `orbat` list of squads; the compiled mission's `orbat` object, keyed by faction, of groups and roles |
| Faction | a row of the `factions` map, `faction-{SIDE}` for BLUFOR, OPFOR or INDFOR; the compiled mission's `factions[]` |
| Squad | a row of the `squads` map; placing a unit joins the side's last open squad or creates "Squad N", and the first slot placed becomes its leader; the compiled mission calls it a group |
| Editor layer | a workflow folder in the `editorLayers` map (name, parent, filed entity ids, hidden, locked); saved in the payload's `editor.editorLayers` and never compiled for the game; not an Eden layer |
| Active layer | the folder new placements file into: page state (`active_layer`), not document state |
| Selection | the selected entity ids, held by the map engine's editing host and mirrored to the page; never saved |

The payload schemas are `contracts_v2/definitions/mission-editor-payload.schema.json`, what Save
Version sends and the editor loads, and `contracts_v2/definitions/mission.schema.json`, the
compiled mission the game reads.

| Eden term | Meaning | Source |
|---|---|---|
| Entity | object, group, trigger, waypoint, system or marker in the scenario | [Eden terminology](https://community.bistudio.com/wiki/Eden_Editor:_Terminology) |
| Asset | a browser entry before placement | same |
| Entity List | the left panel listing every scenario entity | same |
| Layer | a folder of entities, hidden and shown through its attributes | [Eden layer](https://community.bistudio.com/wiki/Eden_Editor:_Layer) |
| Group | several units with a leader | same as Entity |
| Sync / Connect | a general link between entities (modules, tasks, triggers) | [Eden connecting](https://community.bistudio.com/wiki/Eden_Editor:_Connecting) |
| View | the 3D or map camera workspace | same as Entity |

## Gap analysis rows

The [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
pairs the two catalogues by ID only, never by a free-text feature name:

```text
| eden_id | tbd_id | parity | ticket | gap_notes |
|---|---|---|---|---|
| SEL-MAP-003 | SEL-MAP-003 | match | — | |
```

Parity is one of `match`, `partial`, `missing`, `deferred`, `na` and `tbd_only`; the ticket column
names the registry ticket that owns the gap, or `—`.

## Documents

| Document | Role |
|---|---|
| [Feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md) | what the Mission Creator has, by area |
| [Eden interactions](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md) | Eden's interactions, wiki-anchored |
| [Eden UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md) | Eden's workspace, panel by panel |
| [Eden attributes](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md) | Eden's attribute fields |
| [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md) | the two catalogues paired by ID |
| [Eden wiki scrape manifest](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_wiki_scrape_manifest.yaml) | the wiki pages the Eden reference was read from |
