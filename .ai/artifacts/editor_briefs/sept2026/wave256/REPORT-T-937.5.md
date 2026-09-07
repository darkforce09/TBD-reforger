# REPORT-T-937.5

## pwd_branch
```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-937.5
slice/T-937.5
```

## defect_verified
Verified that prior to adding item schemas, a payload with `editor.slots` carrying arbitrary unknown properties (e.g. `{ "id": "s1", "bogus": 1 }`) validated without error. With `$defs/editorSlot` declaring `additionalProperties: false`, unknown keys are strictly rejected.

## changes
1. `packages/tbd-schema/schema/mission-editor-payload.schema.json`: Added `$defs/editorSlot` and `$defs/editorLayer` with `additionalProperties: false` constraining authored slots and layers, and attached `$ref` item schemas to `editor.slots` and `editor.editorLayers`.
2. `apps/website/frontend/src/editor/state/operations/slot_ids.rs`: Added `duplicate_slot_ids(&MissionDocCore) -> Vec<(String, String)>` detecting duplicate slot IDs under the same callsign or squad, leveraging `doc.slot_exists`. Registered in `operations.rs`.
3. `apps/website/frontend/src/editor/library/mission_library.rs`:
   - Enforced `UPLOAD_MAX_BYTES = 8388608` (8 MiB).
   - In `parse_uploaded_document`, added duplicate slot ID check refusing payloads with duplicates and naming both callsign and duplicated slot id.
   - Surfaced the picked file size against the 8.4 MB ceiling in the upload UI when `up_size > 0`.
   - Updated size gate test `the_size_gate_names_both_numbers_and_is_inclusive_at_the_budget` from 67.1 MB to 8.4 MB.
   - Added unit test `duplicate_slot_id_under_callsign_is_refused`.

## perturbation
Verbatim failure when dropping callsign from the duplicate check error:
```
---- editor::library::mission_library::tests::duplicate_slot_id_under_callsign_is_refused stdout ----

thread 'editor::library::mission_library::tests::duplicate_slot_id_under_callsign_is_refused' (1638286) panicked at apps/website/frontend/src/editor/library/mission_library.rs:3508:9:
refusal must name both callsign and duplicated slot id; got "Duplicate slot id \"s1\"."
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    editor::library::mission_library::tests::duplicate_slot_id_under_callsign_is_refused

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1388 filtered out; finished in 0.00s
```
Restoring callsign and touching `mission_library.rs` restored tests to GREEN.

## files_outside_owns
[]

## found_not_fixed
None

## deviations
None

## commits
`f46d994b93968fcb0495ed1f1145fcc5c4d2c59b`: `T-937.5: Payload item schemas, duplicate slot guard, 8 MB ceiling`

## manual_checklist
- [x] Verified defect first
- [x] Top-level payload schema remains open
- [x] Tested against schema validation and unit tests
- [x] Checked wasm32 compilation
- [x] Formatted with `--edition 2021`
- [x] Explicit git paths used
- [x] Slice gate passed

## gate_verdict_tail
```
  gate verdict PASS @ f46d994b9396 recorded: .ai/artifacts/verdicts/T-937.5.json
SLICE GATE: PASS
```
