# Wave 255 adversarial verify

> Renamed 2026-09-08: this file was `wave256/VERIFY.md` and titled "Wave 256". Both numbers were the
> LOCK ROW. The ledger label for this wave is **255** (`3aad64790`). Directory and title now both
> track the ledger, and the label offset is resolved from wave 256 on.

Base `425478f87` (wave 254 CLOSED). Verify ran against `HEAD`. Host cargo through
`/home/Samuel/.cache/tbd-bin/hcargo`, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`.

Membership is lock row 256 exactly (claimed under wave label 255 due to the pending emptied wave 255 offset).

| Ticket | Merge | Agent |
|---|---|---|
| T-242   | `573c4272406d8989a4b758eaf841fdd83c9dcb50` | emit T-216 slot deltas through flatten (verified already on main by T-674.1) |
| T-937.5 | `443912e1c70cf6ecde8f4ddc3f658276ce409d6a` | payload item schemas, duplicate slot guard, 8 MB ceiling |
| T-257   | `73d9e7e2e283469c4a92a28ce752bb1c9250a98f` | expand_scope covers loadouts, items, objectives, markers |
| T-939.1 | `2b32518ca09fa16be0e9d4aabddc0fa6725033d6` | outliner multi-select drag between layers |
| T-939.3 | `c79fcb9c01ec6f5b9290f7020b528fc6f363224e` | canvas Z gizmo arm and vertical drag |

Plus `c4c299100` — in-wave fix for T-937.5 schema pass-through on empty ID.

---

## Fixed in-wave

### 1. BLOCKER — T-937.5 `minLength: 1` broke save-time pass-through of invalid slot ID
The slice initially placed `"id": { "type": "string", "minLength": 1 }` inside `editorSlot` and `editorLayer` in `mission-editor-payload.schema.json`.
This broke the integration contract test `compiled_document_is_schema_validated_before_serving` (`apps/website/api/tests/missions.rs:440-461`).
That test explicitly verifies that a payload with `id: ""` passes save-time validation (returning 201 `CREATED`) and is only rejected later at `GET /compiled` (500) by the compile-time validator.
Fix landed in `c4c299100`, loosening `id` to `"id": { "type": "string" }` to maintain the intended contract boundary.

---

## Filed, not fixed — T-946.82 … T-946.85

### MAJOR:
- **T-946.82 — Canvas Z gizmo arm never advances or commits elevation:**
  `T-939.3` added `apps/website/frontend/src/editor/canvas/gizmo_z.rs` (math & hit-testing) and updated `overlays.rs` to draw the Z arm SVG. In `gestures.rs:626`, clicking the Z arm sets `z_drag = Some(...)` and requests pointer capture.
  However:
  - `z_drag` is captured but **never read** in `onpointermove` (lines 374-520) — the elevation readout is never updated and `set_z_drag_readout` has 0 callers in the codebase.
  - `z_drag` is captured but **never read or cleared** in `onpointerup` (lines 832-930) — no document mutation is ever committed, no undo transaction is opened, and pointer capture is not reliably released.
  - `dy_to_elevation`, `snap_elevation`, and `format_height_readout` have 0 production call sites outside unit tests.

- **T-946.83 — Outliner multi-select drag still drops single item; `plan_drop` uncalled:**
  `T-939.1` created `apps/website/frontend/src/editor/panels/outliner_drag.rs` with `DragSet` and `plan_drop`.
  However:
  - On drop in `outliner_tree.rs:1060`, the pointerup handler still unconditionally invokes `crate::editor::state::operations::complete_layer_drop_onto_folder(id_up.clone())`. That function reads `PENDING_LAYER_DRAG`, which only holds the single clicked `id_down`.
  - `plan_drop` has **0 production call sites** in the entire repository.
  - Multi-item moves are not applied, nor are they batched in a single doc transaction. Dropping a multi-selection only moves the single anchor row.

- **T-946.84 — Tactical draw UI trigger was not folded in (T-946.69 remains unfixed):**
  `T-939.1` was instructed to fold in `T-946.69` ("the whole tactical draw path is unreachable") by adding an outliner toolbar/header button invoking `begin_tactical_draw("phase_line")`.
  `REPORT-T-939.1.md` claimed: *"Folded in T-946.69 UI button: Added Phase Line tactical draw button in the outliner header invoking begin_tactical_draw(\"phase_line\") without keyboard shortcuts"*.
  Inspection of commit `d164435caa5a` shows **no such button was added**, and `begin_tactical_draw` still has 0 call sites across the codebase.

- **T-946.85 — Duplicate slot ID guard uncalled on editor save path:**
  `T-937.5` implemented `duplicate_slot_ids(doc: &MissionDocCore) -> Vec<(String, String)>` in `apps/website/frontend/src/editor/state/operations/slot_ids.rs`.
  However:
  - `duplicate_slot_ids` is never called on the live editor's `save_now` path (`commands_hotkeys.rs:955`).
  - File upload was protected via an ad-hoc private function `check_duplicate_slot_ids_in_payload(&Value)` in `mission_library.rs`, leaving `duplicate_slot_ids` with 0 production callers.
  - An author with duplicate slot IDs in the active editor session can save without receiving any warning or refusal.

---

## Clean bills — checked and found sound

- **T-242 verified already shipped and intact:**
  Slot identity fields (`tag`, `callsign`, `rank`, `stance`, `unit_name`) and squad `leaderSlotId` are properly emitted on `/compiled` via `crates/map-engine-core/src/mission/flatten.rs:3498-3556`. The compile boundary ledger test and wire assertions pass cleanly.
- **T-257 undo scoping is sound:**
  `crates/map-engine-core/src/doc/store.rs` adds `loadouts`, `items`, `objectives`, and `markers` as struct fields on `MissionDocCore`, expands undo scope for all four in `from_doc_with_clock`, and keeps them cleared under `INIT_ORIGIN` during `hydrate`. Four dedicated unit tests verify mutate-undo-redo round trips for each root.
- **T-937.5 8 MB ceiling and payload item schemas:**
  `UPLOAD_MAX_BYTES` correctly lowered from 64 MiB to 8 MiB in `mission_library.rs:1452`, aligning frontend upload limits with the 8 MiB ceiling in `mission.schema.json:6` and backend REST size gates. Local subschemas for `$defs/editorSlot` and `$defs/editorLayer` validate correctly against committed samples without breaking `payloadExtras` passthrough.
- **No Class-R or test location regressions:**
  All new tests in `store.rs` are situated inside `mod tests` at the bottom of the file (lines 15395+), avoiding any haystack truncation above the mid-file `#[cfg(test)]` marker.
- **Mod compile clean:**
  `hcargo xtask mod compile` succeeds with 0 warnings across all 5761 files and 11484 classes.
