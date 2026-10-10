**Status:** live

# Eden editor reference

The reference catalogs of the Arma 3 Eden editor, the design the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) is measured against, and the gap
analysis that pairs them with the Mission Creator's features. Developers and AI agents read it to
learn what Eden does before building or judging a Mission Creator feature.

## Contents

```text
documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/
├── attributes.md                   Eden's attribute fields, one table per entity type
├── eden_gap_analysis.md            every Eden ID's parity with the Mission Creator's features
├── eden_wiki_scrape_manifest.yaml  the scraped Eden wiki pages and their scrape status
├── interactions/                   Eden's interactions, one topic per file
└── ui_anatomy.md                   Eden's workspace, panel by panel
```

## How it works

Three catalogs describe Eden, an external product, from the Bohemia wiki's Eden pages; every fact
cites its wiki URL. The 28 pages the
[scrape manifest](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_wiki_scrape_manifest.yaml)
lists by wiki category, each with a status of `pending`, `scraped`, `reviewed` or `failed`, all
read `scraped` and sit in `.ai/artifacts/eden-wiki/`.

| Document | What it holds | IDs |
|---|---|---|
| [Interactions](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/interactions/README.md) | every Eden interaction as a `#### {ID} — {Short name}` entry over a field table, one topic per file | 83, in the `RIGHT`, `PLACE`, `XFORM`, `WIDGET`, `CREW`, `COMP`, `CONN`, `SEL`, `LAYER`, `ATTR`, `CTX`, `TOOLBAR`, `KEY`, `ACTION` and `STATUS` domains |
| [Attributes](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/attributes.md) | every attribute field of every entity type, one table per type | 93, `ATTR-FIELD-{TYPE}-{NAME}` |
| [UI anatomy](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/ui_anatomy.md) | the workspace layout and each panel: menu bar, toolbar, Entity List, asset browser, view, dialogs, context menu, status bar | none |
| [Gap analysis](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_gap_analysis.md) | one `\| eden_id \| tbd_id \| parity \| … \|` row per catalog ID, pairing it with the Mission Creator's feature by ID | every ID of the two catalogs above |

The ID patterns and the entry format are the
[feature entry schema](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/feds_schema.md)'s,
shared with the Mission Creator's [feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md).
The catalogs state Eden's behaviour only; where one says how the Mission Creator does something,
it is in a section headed Mission Creator counterpart, read from the code and linked to the
inventory area. Parity itself lives in the gap analysis, one row per ID, whose ticket column is
kept by hand: no tool reads or rewrites the file, so a ticket's status change in the central
ticket manager reaches the column only through an edit here.

Eden calls the document it edits a scenario and works in a 3D scene as well as on a 2D map. The
Mission Creator's document is the [mission](/documentation/glossary/g_to_m.md#mission), and its map
view is top-down, so a catalog entry that exists only in 3D is marked `N/A (3D)`. The catalogs
keep Eden's words where they describe Eden's interface.

A new Eden fact goes into the catalog of its kind with its wiki URL, under a new ID in its
domain's pattern, and the gap analysis gets its row; a new wiki page gets a manifest entry.

## Code

- [Mission Creator](/crates/frontend/workspaces/mission_creator_workspace/src/) — the workspace the catalogs'
  Mission Creator counterpart sections and the gap analysis describe.

## Boundaries

- Depends on: the Eden pages of the Bohemia wiki and their scrape in `.ai/artifacts/eden-wiki/`;
  the [feature entry schema](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/feds_schema.md)
  for IDs and entry format; the Mission Creator code for the counterpart sections; the tickets in
  the central ticket manager (`ttm --project reforger show <id>`), which the gap analysis's
  hand-kept ticket column cites.
- Used by: the in-code READMEs under `crates/frontend/workspaces/mission_creator_workspace/src/ui/` (the ui, docks,
  right dock, context menu, inspector, Attributes dialog and outliner READMEs), which link the
  catalog their folder follows; the feature inventory's README and entry schema; the
  [roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md); and the
  tickets in the central ticket manager, which cite the IDs.
- Rules: an ID is never renumbered or reused, because the gap analysis, the roadmap and the
  tickets cite it verbatim; each `| eden_id | … |` table stays whole in one file; the gap analysis has exactly one row per catalog ID; every Eden fact
  cites its wiki URL; each document stays within 500 lines; every child has a Contents line.

## Related documentation

- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) — the
  entry point to the roadmap, the specifications and the decisions.
- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — what the Mission Creator does, area by area.
- [Eden editor on the Bohemia wiki](https://community.bistudio.com/wiki/Eden_Editor) —
  the source of every Eden fact.
