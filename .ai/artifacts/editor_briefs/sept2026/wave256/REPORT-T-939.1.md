# REPORT-T-939.1

## pwd_branch
```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-939.1
slice/T-939.1
```

## defect_verified
```
---- editor::panels::outliner_drag::tests::verify_defect_first stdout ----

thread 'editor::panels::outliner_drag::tests::verify_defect_first' (1159280) panicked at apps/website/frontend/src/editor/panels/outliner_drag.rs:35:9:
assertion `left == right` failed: Dragging with 2 selected rows currently moves only the clicked row
  left: 1
 right: 2
```

## changes
1. Created pure drag-planning module `apps/website/frontend/src/editor/panels/outliner_drag.rs` defining `DragSet { anchor, ids }` and `plan_drop` which preserves visual/relative ordering and rejects self/descendant drops. Registered in `apps/website/frontend/src/editor/panels/mod.rs` alphabetically before `outliner_tree`.
2. Modified `apps/website/frontend/src/editor/panels/outliner_tree.rs` to drag the whole multi-selection when the clicked row is in the selection set, or drag only the clicked row if unselected.
3. Wrapped multi-item drop in a single transaction (`with_batch`) so a single Ctrl+Z reverts the entire move.
4. Added drag ghost count badge when `ids.len() > 1`.
5. Folded in T-946.69 UI button: Added `Phase Line` tactical draw button in the outliner header invoking `begin_tactical_draw("phase_line")` without keyboard shortcuts.

## perturbation
Verbatim failure when truncating `DragSet.ids`:
```
test editor::panels::outliner_drag::tests::perturbation_check ... FAILED

failures:
---- editor::panels::outliner_drag::tests::perturbation_check stdout ----
thread 'editor::panels::outliner_drag::tests::perturbation_check' panicked at apps/website/frontend/src/editor/panels/outliner_drag.rs:50:9:
assertion `left == right` failed: Truncating multi-item drag set dropped items from move plan
  left: 1
 right: 3
```
Restoring it returned all tests to GREEN.

## files_outside_owns
[]

## found_not_fixed
None

## deviations
None

## commits
`d164435caa5afef7f716c2f415b2e62c79ec7d76`: `T-939.1: Outliner multi-select drag between layers`

## manual_checklist
- [x] Verified defect first (saw test fail)
- [x] Native unit tests for pure drag planner
- [x] Alphabetical registration in `panels/mod.rs`
- [x] Single undo step for multi-item drop
- [x] T-946.69 folded in via UI button without keymap modification
- [x] Formatted with `--edition 2021`
- [x] Slice gate passed

## gate_verdict_tail
```
  gate verdict PASS @ d164435caa5a recorded: .ai/artifacts/verdicts/T-939.1.json
SLICE GATE: PASS
```
