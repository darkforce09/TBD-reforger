# Host signal state

What the browser knows and the map engine must be told while the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) runs: the editor context installed
at load, the in-flight placement, the selected entities and the host half of undo grouping. None of
it is state of the [mission](/documentation_v2/glossary.md#mission) document.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/bridge/host_state/
├── armed_placement/          the in-flight placement and zone draw, and the release that commits it
├── editor_context/           the context installed at load: handles, dock mirrors and side signals
├── entity_selection.rs       the selected entities: replace, folder select, select in view, frame
├── mod.rs                    the module tree; every module is wasm-only
└── undo_grouped_gestures.rs  the gestures that undo as one step, and the bulk confirmation
```

## How it works

The canvas mount creates the selected-id set, a `SelectionHandle`, and hands the same cell to the
map engine's `editing::host::install` and to `editor_context::install`, so the set lives in the
frontend while engine commands read and write it. Every other part of this folder reaches the
installed context through the `EDITOR_CONTEXT` thread-local, because the document, engine and
selection handles are `!Send` `Rc`s, and each entry point opens exactly one borrow of it.

`entity_selection.rs` writes the set, then the renderer's tint, then the dock mirrors, in that order
from one place, so a tint and a dock row never disagree: a click on a
[slot](/documentation_v2/glossary.md#slot) that is already part of a multi-selection keeps the set,
since it grabs the set for a drag, and an empty id list is refused rather than read as clear. It
also selects a folder's direct slots or its whole subtree, selects every slot and vehicle in view
(Ctrl+A), and moves the camera to the selection's centroid at the same zoom.
`undo_grouped_gestures.rs` wraps a delete, a paste and an align in
`website_map_engine::editing::batch::with_batch`, so a gesture the operator made as one act is one
Ctrl+Z, and supplies the engine with `confirm_bulk`, a browser confirmation the engine calls before
a bulk move above its threshold; at wasm start `install_undo_gesture_clock` feeds `Date.now` into
the engine's document store, the clock the undo grouping needs.

Nothing here mints an undo step of its own: an arm is host state, a selection over ids the document
no longer holds is pruned by the undo driver's tail, and an unregistered side signal is silence
rather than an error.

## Public surface

- `armed_placement`: the palette arms (`begin_place`, `begin_place_vehicle`, `begin_place_object`,
  `begin_place_composition`, `begin_place_marker`), `has_pending`, `cancel_pending`, the releases
  `place_at_alt` and `place_at_keep`, and the zone draw (`begin_zone_draw`, `begin_zone_reshape`,
  `close_zone_polygon`, `zone_draw_pop_vertex`, `cancel_zone_draw`, `zone_draft`).
- `editor_context`: `install`, `refresh_docks` and `refresh_selection_mirrors`, the document field
  accessors (`read_env`, `read_title`, `set_title`, `update_environment`), the Attributes dialog's
  `open_attributes`, `open_arsenal` and `close_attributes`, and the side-signal openers; the
  crate-visible `EDITOR_CONTEXT`, `Pending` and `bump_doc_tick`.
- `entity_selection`: `set_selection_ids`, `select_slot`, `select_layer_children`,
  `select_layer_descendants`, `select_all_in_view` and `center_on_selection`.
- `undo_grouped_gestures`: `delete_selection`, `paste_at_cursor`, `align_selection`, `confirm_bulk`,
  `confirm_bulk_n_step` and the re-exported `with_batch`.

## Boundaries

- Depends on: `website_map_engine` (`editing::host`, `editing::batch`, `editing::hosted_commands`,
  `editing::tools` for selection and placement, `data::store::operations` for the placement commit,
  the zone draft, the projections and the selection centroid); the undo driver in
  `apps/website/frontend/src/v2/apps/editor/bridge/document_host/`; the outliner's node builders and
  `ensure_active_layer`; the asset catalog; the right dock's marker vocabulary and recently placed
  list; `web_sys` for the confirmation dialog.
- Used by:
  - the canvas mount, its input listeners and the page effects in
    `apps/website/frontend/src/v2/apps/editor/mission_editor/`, which install the context, wire the
    toolbar's select-all and cancel an arm on pointercancel;
  - the pointer gestures and the window keydown in `apps/website/frontend/src/v2/apps/editor/input/`
    (release, selection, Ctrl+A, delete, paste, frame);
  - the docks, the outliner, the inspectors and the dialogs under
    `apps/website/frontend/src/v2/apps/editor/ui/`, the overlays and the undo driver in
    `apps/website/frontend/src/v2/apps/editor/bridge/`, and the loadout commands in
    `apps/website/frontend/src/v2/apps/editor/arsenal/`;
  - `website_map_engine::editing`, which reads the selection cell and calls the closures it is
    handed (`confirm_bulk`, `ensure_active_layer`);
  - the source pins in `apps/website/frontend/src/v2/core/test_support/editor_operations.rs`.
- Rules: every module is `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod` line; a
  confirmation is the host's, handed to the engine as a closure, so no browser dialog lives inside
  the engine; every file here is on the place path that `cargo xtask verify editor-orbat-coherency`
  scans (`tools_v2/xtask/src/verifications/architecture/editor_orbat_coherency.rs`), which bans
  `ensure_default_squad` there and fails when a listed file is missing.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — selection, placement, delete and the bulk operations.
