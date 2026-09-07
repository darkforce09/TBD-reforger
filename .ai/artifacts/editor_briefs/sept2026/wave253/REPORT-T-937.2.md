# REPORT T-937.2 — Undo grouping per gesture

## pwd_branch
`/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-937.2` · `slice/T-937.2`

## defect_verified
Three LOCAL `move_entities` inside 50 ms were three undo steps (`capture_timeout_millis = 0` + `ZeroClock`).

**RED VERBATIM (pre-fix, store.rs pin on live `MissionDocCore`):**

```
thread 'doc::store::tests::three_position_ops_within_50ms_are_one_undo_group' (40950) panicked at crates/map-engine-core/src/doc/store.rs:6885:9:
assertion `left == right` failed: T-937.2: three position ops within 50ms are ONE undo group; got 3
  left: 3
 right: 1
test doc::store::tests::three_position_ops_within_50ms_are_one_undo_group ... FAILED

failures:

failures:
    doc::store::tests::three_position_ops_within_50ms_are_one_undo_group

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1034 filtered out; finished in 0.00s
```

Live grouping pin after the fix (same assertion, `ManualClock` 25 ms apart):

```
test doc::undo_groups::tests::three_position_ops_within_50ms_are_one_undo_group ... ok
```

## changes
- New `crates/map-engine-core/src/doc/undo_groups.rs`: `GESTURE_WINDOW_MS = 300`, injectable clock (`ManualClock` / test auto-advance / real `SystemTime` + wasm `Date.now` via `install_wasm_now`), `GroupingClock` freeze for `begin_group`/`end_group`, depth cap 200 (`hidden_prefix_after`).
- `doc/mod.rs` registers `undo_groups`.
- `store.rs`: Yjs-parity comment at the UndoOptions site replaced by the dated T-937.2 decision; `UndoOptions` built from `undo_groups::undo_options`; `begin_group`/`end_group`; undo depth respects the 200-group cap.
- New `operations/batch.rs`: `with_batch(label, f)` plus façade wrappers for paste / delete-selection / align; wasm start installs `Date.now`.
- `operations.rs` registers `batch` and re-exports those three names from `batch` (entity/transform keep the ungrouped bodies).

## perturbation
Set `GESTURE_WINDOW_MS` to `0` in `undo_groups.rs`; grouping test RED; restore `300`, `touch`, green.

**red VERBATIM:**

```
thread 'doc::undo_groups::tests::three_position_ops_within_50ms_are_one_undo_group' (107829) panicked at crates/map-engine-core/src/doc/undo_groups.rs:254:9:
assertion `left == right` failed: T-937.2: three position ops within 50ms are ONE undo group; got 3
  left: 3
 right: 1
test doc::undo_groups::tests::three_position_ops_within_50ms_are_one_undo_group ... FAILED

failures:

failures:
    doc::undo_groups::tests::three_position_ops_within_50ms_are_one_undo_group

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1039 filtered out; finished in 0.00s
```

**restored_green:** same test `ok` after restoring `300` and `touch`.

## gate_verdict_tail
```
  no-python (T-620)        PASS

  gate verdict PASS @ 36dbee162c8f recorded: .ai/artifacts/verdicts/T-937.2.json
SLICE GATE: PASS
```

First gate on code SHA `36dbee162`. Report commit follows; re-gate so land SHA matches.

## files_outside_owns
- `.ai/artifacts/editor_briefs/sept2026/wave253/REPORT-T-937.2.md` (this report, required by the brief)

## found_not_fixed
- Cap forgets oldest groups by hiding a prefix of the yrs stack (undo/can_undo/undo_depth); evicted `StackItem`s are not `keep(false)`'d, so CRDT blocks from forgotten groups can still pin memory until the document is dropped.
- Other `Any::Array` comments in `store.rs` / frontend still mention `capture_timeout_millis = 0` (those files are sibling-owned or not this slice).
- `vehicleIds` / non-gesture mutators are unchanged (T-257 scope rules held).
- Worktree LFS pointer on `packages/map-assets/everon/dem/everon-dem-16bit.png` needed a local `git lfs checkout` for `everon_peaks_max_above_350` (not committed).

## deviations
- Default *unit-test* clock auto-advances past 300 ms so existing `two_local_moves_are_two_undo_steps` pins stay green; grouping tests inject `ManualClock`.
- Facade wrappers live in `operations.rs` / `batch.rs` rather than editing `entity.rs` (T-930 owns that file this pack).

## commits
- `36dbee162` T-937.2: group undo by gesture window and batch
- (report commit follows)

## manual_checklist
Drag a slot for two seconds; one Ctrl+Z restores it. (Not run here — no ship / no editor session.)
