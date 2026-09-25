**Status:** live

# Editing layer

The map engine's headless editing layer: the editing host that holds the live
[mission](/documentation_v2/glossary.md#mission) document the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) edits, the hosted commands that
change it, the undo drive, and the tool state machines. The Mission Creator supplies everything a
browser owns as closures and function pointers, so every decision here runs under `cargo test`.
Local drafts are a separate doc, [draft persistence](/documentation_v2/website/map-engine/draft_persistence.md).

## Where it lives

- Code: [`apps/website/map-engine/src/editing/`](/apps/website/map-engine/src/editing/README.md),
  compiled with the crate's `editing` feature: `host.rs` (the editing host), `batch.rs`,
  [`hosted_commands/`](/apps/website/map-engine/src/editing/hosted_commands/README.md),
  [`history/`](/apps/website/map-engine/src/editing/history/README.md),
  [`commands/`](/apps/website/map-engine/src/editing/commands/README.md),
  [`lanes/`](/apps/website/map-engine/src/editing/lanes/README.md),
  [`tools/`](/apps/website/map-engine/src/editing/tools/README.md), and the picks and routing
  files. The document it edits is `MissionDocCore` in
  [`data/store/`](/apps/website/map-engine/src/data/store/README.md).
- Entry: `editing::host::install(doc, selection)`, which the Mission Creator's canvas mount calls
  once the document exists (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`),
  and `history::install_host`, through which its document host installs the post-change hook
  (`apps/website/frontend/src/v2/apps/editor/bridge/document_host/history.rs`).
- Related features: [draft persistence](/documentation_v2/website/map-engine/draft_persistence.md),
  the [map engine overview](/documentation_v2/website/map-engine/map_engine_overview.md), and the
  Mission Creator's [feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  for the ORBAT, layer, marker, zone, trigger, connection, clipboard and undo features this layer
  backs.

## Behaviour

### The editing host

1. The Mission Creator creates the document handle and the selection set and calls `install`.
   The host is one thread-local `EditingHost`: the document handle (`DocHandle`), the selected
   ids, and an id counter, `next_id`, that restarts at 0 on every install.
2. The document handle is a shared cell, the same one a restore or a server hydrate swaps a new
   document into, so every command sees the live document without being registered again.
3. `with_doc` and `with_doc_mut` open one borrow of the document per call and drop it before
   returning, so no caller holds a read across a write. Before an install, or while the handle
   holds no document, they answer `None`, which callers read as "nothing happened".
4. A second install replaces the first wholesale: a second mount is a second document.

The Mission Creator also calls `crate::data::store::operations` and the document's mutators
directly, outside this layer.

### A hosted command

1. A caller names what to change (ids, values) and passes the closures only the host can answer:
   the folder a new entity is filed under (`ensure_layer`), the confirmation a large move needs
   (`confirm_bulk`), and the closed vocabularies of marker icons and zone types.
2. The command opens one host borrow and calls the matching `data::store::operations` edit, as one
   transaction, or one undo group for a set. New ids come from the host's counter, passed to the
   document's minting operations.
3. It drops the borrow, then, when something changed, calls `history::after_local_edit`, which runs
   the one hook the host installed: prune the selection, rebind the lanes, bump the version, mark
   the mission unsaved, schedule a draft save, refresh the readouts.
4. A command over many rows runs the tail once for the set, so an apply-to-all or a batch
   reassign is one undo step; `squad_reassignment.rs` brackets its moves with `with_batch`.

Interaction state that is not document content stays in the commands' thread-locals and is never
an undo step: the copied rows of the entity clipboard and the armed connection. The command
families and their files are in the
[hosted commands README](/apps/website/map-engine/src/editing/hosted_commands/README.md#contents).

### Undo and redo

1. There is one undo stack: the document's own `yrs` undo manager, which tracks only the local
   origin. Operator edits are undoable; a boot seed, a restore, a merge or a hydrate, written under
   the init origin, is not.
2. Transactions within one 300 ms capture window (`GESTURE_WINDOW_MS`) merge into one undo step,
   so the transactions of one gesture undo together. `with_batch` opens an explicit group, which
   wins over the window however long it stays open, and closes it through a drop guard, so an
   early return or an unwind never leaves the document grouping.
3. `history::undo` and `history::redo` borrow the document mutably, step its stack, drop the
   borrow, and only then run the same post-change hook as an edit. A step that changed nothing
   runs no hook, so a button pressed against an empty stack costs nothing.
4. The history keeps the last 200 groups (`MAX_UNDO_GROUPS`); older groups are hidden whole, never
   split.
5. Toolbar buttons, keyboard shortcuts and the test bridges all reach this one drive. Without an
   installed hook it still undoes and redoes, and tells nobody.

The grouping clock and the cap are in the
[undo groups README](/apps/website/map-engine/src/data/store/crdt/undo_groups/README.md#how-it-works).

### Tools, picks and routing

- The Mission Creator's canvas mount creates each tool's session state (the ruler chain, the
  line-of-sight capture, the viewshed state, the elevation sampler) and installs it into the
  tools' host cells; its pointer gestures commit clicks, and its overlays read the state back.
- `ruler::EditorTool` decides what a left click means: `Select` drives the selection gesture
  model; `Ruler` and `LoS` share one point-capture gesture. A viewshed runs in budgeted,
  cancellable batches through `viewshed_scheduler`.
- Ruler and line-of-sight results are measurements, held for the session and never written to the
  document; the selection is app state, not document content.
- Picks turn a frozen camera and a pixel into a spatial query; a
  [slot](/documentation_v2/glossary.md#slot) beats a vehicle at equal distance, and a marquee lists
  slots before vehicles. `routing.rs` answers the hover affordance and the click with one
  resolution, so a row looks clickable only when a click reaches something.
- Arrange edits (align, space, orient, pattern, rotate to face) commit as hosted commands over
  the same placement vocabulary the preview uses.

### Known discrepancies

- The module doc of the hosted commands says a command that changed nothing runs no tail
  (`apps/website/map-engine/src/editing/hosted_commands/mod.rs:7-8`); `commit_document_edit`
  (`document_edit.rs:14-19`) and the composition edits (`composition_library.rs:74-80`) run it
  whenever a document is hosted, and the Mission Creator's tail marks the mission unsaved and
  schedules a draft save.
- The line-of-sight walks and the frozen camera's clamp are fixed to Everon's 12,800 m square
  (`TERRAIN_W` and `TERRAIN_H` in `apps/website/map-engine/src/editing/tools/selection/gesture.rs:26`).
- `apps/website/map-engine/src/editing/commands/export_text.rs:12` cites a `/compiled` route the
  API does not serve.

## Data

- The document: `MissionDocCore`, one `yrs` document with the local and init origins; its root
  maps and JSON views are in the [document store README](/apps/website/map-engine/src/data/store/README.md).
- The selection: a list of ids the host owns beside the document; it is never document content.
- The id counter: per install, starting at 0; each mint still checks the document for a clash.
- The host's hook (`HistoryHost::after_document_change`) and the closures a command takes; no
  network call or storage access happens in this layer.

## Design

- The layer is the "survives a reload" half of the
  [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md#where-state-lives):
  the document, the undo stack, the selection and the tool definitions live here; the pointer
  state machine, hover, keybinds and dock state stay in the Mission Creator.
- Rule 5 of `cargo xtask verify engine-layers` keeps `web_sys`, `leptos` and `wasm_bindgen` out
  of the tree, prose included
  ([§2B](/documentation_v2/standards/engine_boundary_rules.md#2b-the-headless-editing-layer)).
  Rules 4 and 7 do not yet treat `editing` as a sibling module, so `data/` naming
  `crate::editing` would pass the gate.
- `cargo xtask verify editor-orbat-coherency` scans `hosted_commands/` and bans
  `ensure_default_squad` on the place path.

## Open work

- [T-1077 — Check hosted commands running the post-change tail on no-op edits](/.ai/tickets/T-1077.toml)
  (idea, no plan): `commit_document_edit` and the composition edits run the tail only when the
  document changed, or the module doc says they always run it.
- [T-1051 — Check whether minted vehicle and object ids can collide](/.ai/tickets/T-1051.toml)
  (idea, no plan): `next_id` restarts at 0 on every install, and the single-id mint checks slot
  ids only; a reopened mission's vehicle or object id either cannot be overwritten, and the guard
  is documented, or the mint checks every id map.
- [T-1055 — Fix engine-layers gate omitting the map engine editing module](/.ai/tickets/T-1055.toml)
  (idea, no plan): rules 4 and 7 treat `editing` as a sibling module.
- [T-1062 — Derive map grid, basemap, peaks and forest from terrain size](/.ai/tickets/T-1062.toml)
  (idea, no plan): terrain-size constants follow the loaded terrain.
- [T-1068 — Rewrite stale map engine comments outside mission data](/.ai/tickets/T-1068.toml)
  (idea, no plan): `export_text.rs` stops citing the unserved `/compiled` route and retired
  modules, and the pick and marquee comments name the current code.

## Decisions

- One installed host with a shared document cell: a restore or a hydrate swaps the document
  without re-registering any command, and a command never holds a stale copy.
- One borrow per entry point, dropped before the tail: the tail opens read borrows of the same
  document, and a held write would panic the `RefCell`.
- One post-change hook, not several: the order of what follows an edit (selection, lanes,
  version, dirty flag, save, readouts) is the host's to decide.
- The document's undo manager is the only stack: operator edits, grouped gestures and adoptions
  share one history, and a boot seed or a merge never becomes an undo step.
- Browser needs cross as closures and function pointers: the tree stays testable with no
  browser, which rule 5 of the gate enforces.
