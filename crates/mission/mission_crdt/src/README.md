# Mission CRDT source

The pieces the [mission](/documentation/glossary/g_to_m.md#mission) document is built from, below its
rows: the ordered id arrays that let peers merge squad and layer membership, the columnar
[slot](/documentation/glossary/n_to_z.md#slot) projection the map draws and picks from, and the clocks
that group edits into undo steps.

## Contents

```text
crates/mission/mission_crdt/src/
├── id_arrays/    the squad and layer id lists as native `yrs` arrays, and the helpers over them
├── lib.rs        the crate root: module header and the module tree
├── prelude.rs    the slot columns, their codes, the interner and the undo constants for glob import
├── soa.rs        `SlotSoa`, the row-aligned slot columns, its stance codes and the string interner
└── undo_groups/  the gesture window, explicit undo groups, the depth cap and the grouping clock
```

## How it works

All three serve `MissionDocCore`, the mission document of the document rows. Its row
writes keep `slotIds` and `entityIds` through `id_arrays/`, its undo manager takes its window,
clock and cap from `undo_groups/`, and `MissionDocCore::materialize` fills a `SlotSoa`.

A `SlotSoa` holds one row per visible slot, keyed by `ids[row]`. A slot marked `editorHidden`, or
filed in a layer that is hidden or sits under a hidden one, has no row, though the document keeps
it. The columns are the id, `xs`, `ys`, the interleaved `xy` pairs the slot render lane uploads,
`zs`, rotations, a stance code (`STANCE_CROUCH` and `STANCE_PRONE` for those `stance` strings,
`STANCE_STAND` for anything else) and the side key, plus four index columns into dictionaries of
roles, tags, squads and layers. The `Interner` fills each dictionary in first-seen
order during one materialize, and `NONE_IDX` marks a slot with no tag or no layer. Positions are
`f32`, fine for pixels and lossy for the compiler, which reads the exact rows instead.

## Public surface

- `soa::SlotSoa` with `NONE_IDX` and the `STANCE_*` codes: the columns the map engine's editing
  layer picks and selects over, and the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
  render lanes, select tool and canvas mount read; `soa::Interner`, which the document's
  materialise fills the dictionaries with.
- `undo_groups::{GESTURE_WINDOW_MS, MAX_UNDO_GROUPS, GroupingClock, platform_clock,
  undo_options, hidden_prefix_after}`: the undo window, cap, clock seam and options the document
  builds its undo manager from.
- `id_arrays`: the id list operations the document's row writes run inside their transactions.
- `prelude` holds the slot columns, the interner, the grouping clock and the undo constants.

## Boundaries

- Depends on: `yrs`, `time_source` (the host clock trait and the platform clock) and the standard
  library.
- Used by:
  - `MissionDocCore`, which builds its rows, materialised columns and undo manager from all three;
  - the map engine's document operations, selection and editing layer, which read `SlotSoa`
    columns, `NONE_IDX` and the stance codes (`crates/mission_editing/mission_editing_session/src/picking.rs`,
    `crates/mission_editing/mission_editing_session/src/selection_universe.rs`, the selection tools in
    `crates/mission_editing/map_editing_tools/src/selection/`, the slot fingerprint in
    `crates/mission_editing/mission_persistence/src/` and the slot attributes command in
    `crates/mission_editing/mission_editing_commands/src/hosted_commands/`);
  - the Mission Creator in `crates/frontend/workspaces/mission_creator_workspace/src/` through `SlotSoa`: the
    document handle, the undo driver and its render lanes in
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/`, the select-in-view of
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/`,
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/select_tool.rs`, and the canvas mount and
    the document helpers in `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/`.
- Rules:
  - `SlotSoa` columns stay row-aligned by id and a hidden slot has no row while the document
    keeps it (the materialise tests of the mission document);
  - mission tier 1: no dependency on graphics, terrain or applications
    (`cargo xtask verify crate-tiers`).
