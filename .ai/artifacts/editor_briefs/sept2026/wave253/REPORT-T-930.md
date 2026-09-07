# REPORT T-930 — Vehicle first-paint disc until moved

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-930
slice/T-930
```

## defect_verified

Engine-mount first bind (`MissionEditorPage` engine-create task) uploaded `slots_bind_symbology` only. No `vehicles_bind_symbology`. `pack_vehicle_instances` is the yellow disc:

```
test slots_gpu::tests::pack_vehicle_instances_disc_yellow ... ok
```

(`SLOT_GLYPH_DISC` + `SLOT_SELECTED_RGBA` `[250, 204, 21, 255]`). First-bind chunk contained neither `vehicles_bind_symbology` nor `vehicles_bind(`. Catalog place relied on `after_local_edit` without a vehicle-lane invalidate + damage mark. The disc packer is a deliberate atlas-less fallback, not first paint.

## changes

| path | why |
|---|---|
| `crates/map-engine-core/src/slots_gpu.rs` | Document that `pack_vehicle_instances` is not first-paint. Test: default zoom (−2 → 4 m/px) `pack_vehicle_symbology` is a silhouette cell, not `SLOT_GLYPH_DISC`. |
| `apps/website/frontend/src/editor/mission_editor.rs` | First bind also `vehicles_bind_symbology` from `vehicle_lane_fields()`. Class-R pins for first bind + place-time invalidate. |
| `apps/website/frontend/src/editor/state/operations/entity.rs` | Catalog `Pending::Vehicle` sets `placed_vehicle`; after `after_local_edit` calls `rebind_vehicle_lane_after_place` (`vehicles_bind_symbology` + `mark_dirty`). |

T-819 crewed-slot hide unchanged (`t819_crewed` 8 passed). Move path still `after_doc_change` / drag preview.

## perturbation

Skipped the `rebind_vehicle_lane_after_place()` call in `place_at_impl` (empty `if placed_vehicle`).

**red VERBATIM:**

```
thread 'editor::mission_editor::t930_vehicle_first_paint::place_path_invalidates_vehicle_lane' (19230) panicked at apps/website/frontend/src/editor/mission_editor.rs:3509:9:
T-930: place_at_impl must call the place-time vehicle invalidate; body:
...
        if placed_vehicle {
        }
...
test editor::mission_editor::t930_vehicle_first_paint::place_path_invalidates_vehicle_lane ... FAILED

failures:
    editor::mission_editor::t930_vehicle_first_paint::place_path_invalidates_vehicle_lane

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1325 filtered out; finished in 0.00s
```

Restored the call, `touch` entity.rs: **restored_green** (`place_path_invalidates_vehicle_lane ... ok`).

## gate_verdict_tail

First PASS was on dirty HEAD `9f4a8c7d1dff` (wasm32/clippy lifetime fail fixed: bind `RefMut` before `as_mut`). Re-gate after commit so land SHA matches.

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ <post-commit SHA> recorded: .ai/artifacts/verdicts/T-930.json
SLICE GATE: PASS
```

Lock wait ~210s (T-938.4 / T-938.3 / T-937.4).

## files_outside_owns

- `.ai/artifacts/editor_briefs/sept2026/wave253/REPORT-T-930.md` (this report, required by the brief)

## found_not_fixed

- `pack_vehicle_instances` / `vehicles_bind` remain the atlas-less fallback (`vehicles_bind_symbology` when `symbology_base` is `None`; drag preview when column lengths disagree). Zoom-out degrade in `pack_vehicle_symbology` still packs a side-tinted disc (not yellow).
- ORBAT `orbat_add_vehicle` and composition stamps that include vehicles still rely on `after_local_edit` only (catalog place is the T-930 arm).
- `sync_slot_zoom_uniform` rematerializes slots + comments on the symbology zoom crossing, not the vehicle lane (engine.rs, T-938.3 owns).
- `everon_peaks_max_above_350` failed on the worktree LFS pointer until a local `git lfs checkout` of `packages/map-assets/everon/dem/everon-dem-16bit.png` (not committed).

## deviations

- Ticket verify named `cargo xtask mk leptos-gates`; brief forbids ci-local / leptos-gates. Ran `cargo test -p map-engine-core --all-features`, frontend T-930/T-819 pins, `cargo xtask platform wave gate --slice T-930`.
- SIZE-3 allowlisted files: tests live in the owned files (no extra test path).

## commits

(filled after commit)

## twins_confirmed

N/A — no Enfusion scripts in owns.
