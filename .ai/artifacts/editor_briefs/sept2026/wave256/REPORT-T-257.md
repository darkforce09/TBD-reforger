# REPORT-T-257

## pwd_branch
```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-257
slice/T-257
```

## defect_verified
Yes, tested locally first, mutators on loadouts, items, objectives, and markers were NOT reversed by `doc.undo()` and threw assertion failures `undo returned false` meaning the roots were out of undo scope.

## changes
1. Added `loadouts: MapRef`, `items: MapRef`, `objectives: MapRef`, and `markers: MapRef` to `MissionDocCore` struct fields.
2. In `MissionDocCore::from_doc_with_clock`, added `expand_scope` calls for `loadouts`, `items`, `objectives`, and `markers`.
3. Populated the new fields during `MissionDocCore::from_doc_with_clock`.
4. In `MissionDocCore::hydrate`, reused `self.loadouts`, `self.items`, `self.objectives`, and `self.markers` everywhere instead of querying them from the document during execution.
5. Added tests in `doc::store::tests` at EOF for `t257_loadouts_undo_scoped`, `t257_items_undo_scoped`, `t257_objectives_undo_scoped`, `t257_markers_undo_scoped`.

## perturbation
Commenting out `undo_mgr.expand_scope(&doc, &loadouts);` resulted in verbatim failure:
```
test doc::store::tests::t257_loadouts_undo_scoped ... FAILED

failures:
---- doc::store::tests::t257_loadouts_undo_scoped stdout ----
thread 'doc::store::tests::t257_loadouts_undo_scoped' (1134547) panicked at crates/map-engine-core/src/doc/store.rs:15405:9:
undo returned false
```
Restoring it fixed the build to GREEN.

## files_outside_owns
[]

## found_not_fixed
None

## deviations
None

## commits
Committed early and cleanly:
`T-257: expand_scope to cover loadouts, items, objectives, and markers`

## manual_checklist
- [x] Verified defect first (saw tests fail)
- [x] Placed tests at the bottom of the file
- [x] Avoided pinned signatures
- [x] Explicit git paths used, no `git add -A`
- [x] Ran cargo test
- [x] Slice gate passed

## gate_verdict_tail
```
  gate verdict PASS @ 774e91130608 recorded: .ai/artifacts/verdicts/T-257.json
SLICE GATE: PASS
```
