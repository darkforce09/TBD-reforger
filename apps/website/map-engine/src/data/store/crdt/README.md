# Document building blocks

The pieces the [mission](/documentation_v2/glossary.md#mission) document is built from, below its
rows: the ordered id arrays that let peers merge squad and layer membership, the columnar
[slot](/documentation_v2/glossary.md#slot) projection the map draws and picks from, and the clocks
that group edits into undo steps.

## Contents

```text
apps/website/map-engine/src/data/store/crdt/
├── id_arrays/    the squad and layer id lists as native `yrs` arrays, and the helpers over them
├── mod.rs        the module tree
├── soa.rs        `SlotSoa`, the row-aligned slot columns, its stance codes and the string interner
└── undo_groups/  the gesture window, explicit undo groups, the depth cap and the host clock hook
```

## How it works

All three serve `MissionDocCore` in `apps/website/map-engine/src/data/store/rows/`. Its row
writes keep `slotIds` and `entityIds` through `id_arrays/`, its undo manager takes its window,
clock and cap from `undo_groups/`, and `MissionDocCore::materialize` fills a `SlotSoa`.

A `SlotSoa` holds one row per visible slot, keyed by `ids[row]`. A slot marked `editorHidden`, or
filed in a layer that is hidden or sits under a hidden one, has no row, though the document keeps
it. The columns are the id, `xs`, `ys`, the interleaved `xy` pairs the slot render lane uploads,
`zs`, rotations, a stance code (`STANCE_CROUCH` and `STANCE_PRONE` for those `stance` strings,
`STANCE_STAND` for anything else) and the side key, plus four index columns into dictionaries of
roles, tags, squads and layers. The crate-private `Interner` fills each dictionary in first-seen
order during one materialize, and `NONE_IDX` marks a slot with no tag or no layer. Positions are
`f32`, fine for pixels and lossy for the compiler, which reads the exact rows instead.

## Public surface

The document store re-exports what leaves this folder:

- `soa::SlotSoa` with `NONE_IDX` and the `STANCE_*` codes: the columns `crate::editing` picks and
  selects over, and the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s render
  lanes, select tool and canvas mount read.
- `undo_groups::install_wasm_now`, `GESTURE_WINDOW_MS`, `MAX_UNDO_GROUPS` and `ManualClock`: the
  host clock hook the Mission Creator calls at wasm start, and the undo constants and test clock.
- `id_arrays` stays inside the document store: only the row module calls it.

## Boundaries

- Depends on: `yrs` and the standard library; nothing else of the crate.
- Used by:
  - `crate::data::store::rows`, which builds `MissionDocCore` from all three;
  - `crate::data::store::operations` and `crate::data::store::selection`, which read `SlotSoa`
    columns, `NONE_IDX` and the stance codes;
  - `crate::editing` through `SlotSoa`: `apps/website/map-engine/src/editing/picking.rs`,
    `apps/website/map-engine/src/editing/selection_universe.rs`, the selection tools in
    `apps/website/map-engine/src/editing/tools/selection/`, the slot fingerprint in
    `apps/website/map-engine/src/editing/persist/` and the slot attributes command in
    `apps/website/map-engine/src/editing/hosted_commands/`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/` through `SlotSoa`: the
    document handle, the undo driver and its render lanes in
    `apps/website/frontend/src/v2/apps/editor/bridge/document_host/`, the select-in-view of
    `apps/website/frontend/src/v2/apps/editor/bridge/host_state/`,
    `apps/website/frontend/src/v2/apps/editor/input/tools/select_tool.rs`, and the canvas mount and
    the document helpers in `apps/website/frontend/src/v2/apps/editor/mission_editor/`; and
    `apps/website/frontend/src/v2/apps/editor/bridge/host_state/undo_grouped_gestures.rs` through
    `install_wasm_now`.
- Rules:
  - `SlotSoa` columns stay row-aligned by id (`add_slot_materializes_soa` in
    `apps/website/map-engine/src/data/store/rows/tests/cases_2.rs`), and a hidden slot has no row
    while the document keeps it (`materialize_drops_hidden_slots_the_document_still_holds` in
    `cases_8.rs`, `hidden_layer_slot_is_filtered_from_materialize_but_kept_in_the_doc` in
    `cases_5.rs`, same folder);
  - nothing here names a module of the crate outside `crate::data`, nor the graphics engine (rule 7
    of `cargo xtask verify engine-layers`).
