# Mission editing

The headless editing layer of the map engine, behind the crate's `editing` feature: the editing
host that holds the live [mission](/documentation/glossary/g_to_m.md#mission) document, the commands
and undo drive that edit it, the overlay lanes and picks drawn from it, the decisions behind local
drafts, and the interactive map tools. The
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) supplies everything a browser owns
(clocks, frame pumps, prompts, storage) as closures and function pointers, so every decision here
is answerable by `cargo test`.

## Contents

```text
legacy/map_engine/src/editing/
├── batch.rs               `with_batch`: several document transactions as one undo step
├── commands/              the pure half of commands: export bytes, report wording, digests
├── history/               undo, redo and the post-change tail the host installs
├── host.rs                `EditingHost`: the live document, the selection and the id minter
├── hosted_commands/       commands that edit the hosted document, then run the tail
├── lanes/                 connection, comment and marker lanes from the document, and picks
├── mod.rs                 the module tree
├── persist/               local draft decisions: keys, blob checks, merge, server adoption
├── picking.rs             slot and vehicle picks and marquees mapped to document ids
├── routing.rs             `route_target`: the surface that owns an id, and if a click reaches it
├── selection_universe.rs  selectable ids, the crew hide, the map-render slots, paste anchor
├── tests/                 unit tests for the picks and marquees and the document's tie policy
└── tools/                 the map tools: select, ruler, line of sight, viewshed scheduler
```

## How it works

```text
Mission Creator (apps/frontend/src/workspaces/editor/)
  │ creates the MissionDocCore handle and the selection, then host::install(doc, selection)
  ▼
host.rs ── with_doc / with_doc_mut: one borrow per call, dropped before returning
  │
  ├─ hosted_commands/ ── mission_operations ── history::after_local_edit (host's tail)
  ├─ history/ ────────── MissionDocCore::undo / redo, then the same tail
  ├─ batch.rs ────────── begin_group / end_group around a multi-transaction edit
  ├─ lanes/, picking.rs, routing.rs, selection_universe.rs ── read the document's JSON and SoA
  └─ persist/ ────────── adopt, merge, classify and snapshot through the same DocHandle
tools/ ── session state the host installs into tool cells; never the document
```

The editing host is one thread-local `EditingHost`: the document handle (`DocHandle`, the same
shared cell a restore or a hydrate swaps into, so a command always sees the live document), the
selected ids and a per-session id counter, `next_id`, that restarts at 0 on every install and is
passed to the document's minting operations. The Mission Creator also calls
`mission_operations` and the document's mutators directly, outside this layer.

Every edit through `hosted_commands` ends in `history::after_local_edit`, which runs the one hook
the host installed (prune the selection, rebind lanes, mark unsaved, schedule a save); undo and
redo end in the same hook. `with_batch` closes its undo group through a drop guard, so an early
return or an unwind never leaves the document grouping. The picks in `picking.rs` turn a frozen
camera and a pixel into a world query, ask `spatial_indexes::point_indexes::picking` for rows, and let
`MissionDocCore` map rows to ids and break ties
(a [slot](/documentation/glossary/n_to_z.md#slot) beats a vehicle at equal distance).
`selection_universe.rs` reads membership from the post-change document's raw maps rather than the
materialized slots, so hiding a slot never deselects it, and `routing.rs` answers the affordance
probe and the click with one resolution, so a row is clickable only when a click reaches something.

## Public surface

- `host`: `install`, `with_doc`, `with_doc_mut`, `doc_handle`, `selection_len` and the selection
  setters, for the Mission Creator's canvas mount, arsenal, docks, inspector and zone geometry.
- `hosted_commands`: every document command and its row types, for the Mission Creator's docks,
  inspector, outliner, modals, bridge and input.
- `history`: `undo`, `redo`, `after_local_edit`, `HistoryHost` and `install_host`, for the
  document host that installs the post-change hook.
- `batch::with_batch`, for grouped gestures (delete, paste, align, outliner drops).
- `commands`: the export, report and clipboard wording for the Mission Creator's document commands.
- `lanes`, `routing`, `selection_universe` and `picking::squad_link_inputs`, for the editor page,
  the render lanes and the pointer hover.
- `persist`: key scoping, blob checks, merge, classification, adoption and snapshots, for the
  Mission Creator's shell.
- `tools`: the ruler, line-of-sight, selection, viewshed-scheduler and placement vocabularies.

## Boundaries

- Depends on: `mission_document` (`MissionDocCore`, `ConnectionKind`), `mission_crdt`
  (`SlotSoa`), `mission_operations`, `formation_geometry` (the arrange vocabulary),
  `mission_payload` and `mission_validation` (the compile and its findings), `crate::camera` (the
  frozen ortho camera and the grid reference), `spatial_indexes` (the point index and picking
  rows), the line of sight crates (`terrain_line_of_sight`, `interior_line_of_sight`,
  `world_line_of_sight`), `terrain_elevation::manifest`, `unit_symbology` (squad link inputs, side
  tints), `crate::frame::EngineHandle` on `wasm32` with `render`; `serde_json`. The `editing`
  feature turns on `store`, `world` and `streaming`.
- Used by:
  - the Mission Creator in `apps/frontend/src/workspaces/editor/` (canvas mount, bridge,
    input, shell, docks, inspector, outliner, modals, arsenal) and the debug building viewer in
    `apps/frontend/src/workspaces/debug/building_viewer/`;
  - the headless editor gates in `tools/developer_tools/src/browser_testing/`, through the
    selection self-checks the select tool publishes;
  - the `engine-layers` and `editor-orbat-coherency` gates of `tools/xtask/`, which scan this
    tree.
- Rules:
  - no `web_sys`, `leptos` or `wasm_bindgen` anywhere under this folder (rule 5 of
    `cargo xtask verify engine-layers`);
  - the hosted commands never call `ensure_default_squad` on the place path
    (`cargo xtask verify editor-orbat-coherency`);
  - picks are square for slots and circular for vehicles, ties go to the slot and marquees list
    slots before vehicles (`square_slots_circular_vehicles_and_equal_distance_policy` in
    `tests/picking_selection.rs`);
  - the ruler and line-of-sight measurements never write the document (the `session_local` tests
    of `legacy/map_engine/src/editing/tools/ruler/` and
    `legacy/map_engine/src/editing/tools/line_of_sight/`).

## Related documentation

- [Mission document](/crates/mission/mission_document/README.md) — the document this
  layer edits and the operations it calls.
- [Architecture gates](/tools/xtask/src/verifications/architecture/README.md) — the
  `engine-layers` and `editor-orbat-coherency` gates that scan this tree.
- [Mission Creator feature inventory](/documentation/apps/frontend/workspaces/editor/feature_inventory/README.md)
  — the Mission Creator features this layer backs.
- [Editing layer](/documentation/legacy/map_engine/editing_layer.md) — the host, hosted
  commands, undo and tools as flows, with open work and decisions.
- [Draft persistence](/documentation/legacy/map_engine/draft_persistence.md) — how the
  Mission Creator opens, reconciles, adopts, snapshots and saves a draft through `persist/`.
