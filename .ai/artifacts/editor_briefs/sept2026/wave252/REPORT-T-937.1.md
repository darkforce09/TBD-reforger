# REPORT T-937.1 — Native YArray id lists

## pwd_branch
`/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-937.1` · `slice/T-937.1`

## defect_verified_on_main
`append_id` / `retain_ids` clone-rewrote `Any::Array` and `Map::insert`ed the whole list. Two peers appending distinct ids from a shared empty `slotIds` last-write-win: one id is dropped.

Pin still in-tree (the old algorithm, not the live mutators):

```
test doc::id_arrays::tests::concurrent_any_array_clone_rewrite_drops_an_id ... ok
```

That test asserts `ids.len() == 1` after two-peer clone-rewrite + sync (`Any::Array clone-rewrite must drop one concurrent id (the T-937.1 defect); got {ids:?}`).

Live document after the fix:

```
test doc::id_arrays::mission_doc_tests::two_peers_concurrent_slot_id_appends_both_survive ... ok
test doc::id_arrays::mission_doc_tests::undo_removes_only_the_local_append ... ok
```

## changes
- New `crates/map-engine-core/src/doc/id_arrays.rs`: `read` / `append` / `retain` / `move` over a yrs `YArray` for `squad.slotIds` and `layer.entityIds`; both `YArray` and legacy `Any::Array` read as the same ordered ids; `migrate_legacy_id_lists` for hydrate.
- `doc/mod.rs` registers `id_arrays`.
- `store.rs` routes every previous `append_id` / `retain_ids` / `read_id_array` caller through that module; `add_squad` / `add_editor_layer` / default-layer seed insert empty native arrays; hydrate migrates after `load_rows`; paste / place-composition / merge / remove / move-to-layer no longer clone-rewrite those two keys. `vehicleIds` / `squadIds` stay `Any::Array`.

## perturbation
Skipped `migrate_legacy_id_lists` in `hydrate`, restored, `touch`, green.

**red VERBATIM:**

```
thread 'doc::id_arrays::mission_doc_tests::hydrate_legacy_payload_migrates_slot_ids_to_yarray' (3296522) panicked at crates/map-engine-core/src/doc/id_arrays.rs:573:9:
hydrate must migrate squad.slotIds to YArray
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test doc::id_arrays::mission_doc_tests::hydrate_legacy_payload_migrates_slot_ids_to_yarray ... FAILED

failures:

failures:
    doc::id_arrays::mission_doc_tests::hydrate_legacy_payload_migrates_slot_ids_to_yarray

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1022 filtered out; finished in 0.00s
```

**restored_green:** same test `ok` after restoring the migrate call and `touch`.

## gate_verdict_tail
```
  no-python (T-620)        PASS

  gate verdict PASS @ b59b99116a09 recorded: .ai/artifacts/verdicts/T-937.1.json
SLICE GATE: PASS
```

(First gate was on dirty HEAD. Commits follow; re-gate so land SHA matches.)

## mod_compile_verdict
N/A — no `.c` / `.layout` in this slice.

## files_outside_owns
- `.ai/artifacts/editor_briefs/sept2026/wave252/REPORT-T-937.1.md` (this report, required by the brief)

## found_not_fixed
- `vehicleIds` / `squadIds` / other `Any::Array` id lists are unchanged (LOCKED: only `slotIds` and `entityIds` change representation).
- Concurrent first-create of the *array object* on an unmigrated `Any::Array` is still LWW; hydrate + `add_squad` / `add_editor_layer` insert the native array before collab appends.

## deviations
- `move_id` has no production `store.rs` caller yet (no in-array reorder mutator). Helper + unit test are present as the ticket required.
- Worktree LFS pointer on `packages/map-assets/everon/dem/everon-dem-16bit.png` made `everon_peaks_max_above_350` fail with `Invalid PNG signature` until a local `git lfs checkout` of that one cached object (not committed).

## commits
- `47b60e04a` T-937.1: store squad.slotIds and layer.entityIds as YArray
- (report commit follows)

## manual_checklist
None (spec ═══ MANUAL ═══ None).

## twins_confirmed
N/A — no Enfusion scripts in owns.
