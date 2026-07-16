# T-159.19 — Marquee select + entity drag-move — verify log

**Slice:** T-159.19 — the two React `useSelectTool` over-threshold LMB gestures on the Leptos
Mission Creator: **marquee** (empty drag → `PointIndex::pick_rect` → replace selection) and
**drag-move** (icon drag → world delta → `MissionDocCore::move_entities` on release → re-bind
glyphs), plus the first **edit-driven persist** (re-arm the debounced IDB writer after the move).
Deferred: cluster drill, Attributes dblclick, undo UI, save/export (.20), Eden chrome, GpuTimer.
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · branch `t-159-leptos-ui` · base `eb30ebea` (T-159.18 tip).
**Executor:** claude-code.
**Result:** **PASS** — `cargo check --target wasm32-unknown-unknown` clean (no warnings),
`trunk build --release` clean; the new `editor-marquee-drag-smoke` passes and all **6** prior editor
smokes stay green.

## What shipped
- **`apps/website-leptos/src/select_tool.rs`** (+247) — gesture math (pure; verified in-browser):
  - `enum LeftGesture { Pending(PendingLeft) | Move{ids,start_wx,start_wy,cam,dx,dy} | Marquee{start_x,start_y,start_wx,start_wy,cam} }`
    — the React `useSelectTool` union (`pending-left → move | marquee`). The frozen `cam` (M2/X-05)
    copied at the press drives every unproject in the gesture.
  - `compute_move_ids(hit, selection)` — React `:204`: drag an already-selected slot → move the whole
    selection; else move `[hit]`.
  - `drag_delta(cam, start_wx, start_wy, px, py)` — React `:226` `unproject(px) − startWorld`;
    non-finite (singular pixel matrix) → `(0,0)`.
  - `marquee_ids(cam, soa, start_wx, start_wy, end_px, end_py)` — React `:293`: unproject the release
    corner, **order** the world AABB (`pick_rect` returns empty on `max<min`), `PointIndex::pick_rect`
    → `soa.ids`; NaN-guarded.
  - `marquee_selfcheck(soa)` — Class-S peer of `pick_selfcheck`: `pick_rect` set-equals a brute box
    scan over a battery of world boxes.
  - `probe_marquee_json` / `probe_move_json` + 3 new `window.__editorSelection` fields
    (`marquee_selfcheck`, `probe_marquee`, `probe_move`) for the gate — each centres seed 0 and reads
    back the clamped view before projecting (mirrors the existing `probe_json`).
- **`apps/website-leptos/src/mission_editor.rs`** (+297/-56) — the pointer gesture machine:
  - `lmb: Option<PendingLeft>` → `left: Option<LeftGesture>`.
  - `onpointerdown` — wrap the press into `LeftGesture::Pending` (unchanged otherwise).
  - `onpointermove` — after the pan branch, promote a `Pending` past `DRAG_THRESHOLD_PX` (4 px) into
    `Move` (pick hit at the press corner) or `Marquee` (empty press) and capture the pointer; then
    drive the live preview (`engine.set_drag` for the drag delta, `engine.upload_marquee` for the
    rect), coalescing the delta/rect into the gesture. Ownership is `take → compute → put back`, so a
    `Pending→Move|Marquee` transition never aliases a `&mut`, and no `left` borrow is held across the
    inner put-back (the `if let` temporary-lifetime footgun).
  - `onpointerup` — `Pending` = the unchanged sub-threshold click; `Move` (if it moved) commits ONE
    `move_entities(ids, dx, dy, vec![0.0; n])` txn (one undo step), clears the drag overlay, re-binds
    the moved glyphs (`slots_bind_soa`), keeps the moved slots selected, then
    `yrs_persist::schedule_edit_persist`; `Marquee` (≥1×1 px box) replaces the selection with
    `marquee_ids` and hides the rect. Pointer capture released in both.
  - `onpointercancel` — drop the gesture without committing; clear the drag overlay / marquee rect.
- **`apps/website-leptos/src/yrs_persist.rs`** (+55) — `schedule_edit_persist(doc, id)` re-arms the
  SAME debounced + serialized writer the boot seam uses (get_bytes read at write time, cancel when the
  doc `Option` clears) — the first mutator-driven persist (S8, deferred by .17/.18). A
  `thread_local! EDIT_PERSIST_COUNT` (starts 0; boot persist calls `save_state_debounced` directly,
  NOT this) + `edit_persist_count()` on the `__missionPersist` bridge give the gate an honest signal
  that the *edit* re-armed the writer.
- **`.ai/artifacts/t159_gates/driver/smoke_marquee_drag_editor.mjs`** (new) — the `editor-marquee-drag-smoke` gate.

## Frozen-viewport / X-05
Every unproject in a gesture uses the `OrthoCamera` snapshotted at pointerdown (`PendingLeft.cam`,
carried into `LeftGesture::Move/Marquee`). The live `RenderEngine::unproject_xy` was deleted under
audit X-05; a live unproject would feed back as pan/zoom mutate mid-gesture. Marquee AABB is
flip-agnostic (deck `OrthographicView` has no rotation, so screen↔world is monotone per axis and
`min/max` over the two unprojected corners is exact); drag delta = `unproject(cur) − unproject(start)`
so the picked icon follows the cursor.

## Correctness note — Class R/S are SEMANTIC
- **Move = Class R.** The gate asserts the SEMANTIC per-slot position digest
  (`__missionPersist.slots_digest()` — id-sorted rows, bit-exact `f32` via `to_bits`) **changes** after
  the drag, not encode bytes. `move_entities` adds `(dx,dy)` to the dragged slot and sets `z = 0`
  (the DEM-not-ready byte-parity case; React `terrainZ` on the flat editor) — the seed's `xs` bits
  change, so its digest row changes.
- **Marquee = Class S.** `marquee_selfcheck()` proves `PointIndex::pick_rect` set-equals a brute-force
  box scan over the seeds; `probe_marquee()` then makes the CDP drag reproduce the SAME `marquee_ids`
  the handler runs (start world = `unproject(press corner)`, end px = release) — end-to-end parity on
  top of the Class-S check.

## GPU-preview lanes — visibility & the headless-WebGPU backend
`set_drag` / `set_selection` / `slots_bind_soa` early-return while `!atlas_ready` (the editor uploads
no slot atlas yet), so the drag/selection previews are **visual no-ops until a later atlas slice** (as
T-159.18's tint was); acceptance is bridge state (digest / selection), not pixels. `upload_marquee` has
no atlas guard (a self-contained polygon lane), so the marquee rect **does** render — except that on
**headless WebGPU** (lavapipe / SwiftShader-WebGPU) its `create_buffer_init` (`mappedAtCreation`) is
rejected (`"size N too large … when mappedAtCreation == true"`), a software-rasterizer limitation, NOT
a code defect: real WebGPU + the WebGL2 backend both draw it (T-151 shipped the lane on WebGPU). The
gate therefore forces **WebGL2** (`?force=webgl`, the `selfcheck_editor` precedent), whose `glBufferData`
path has no such limit — exercising the real upload path. The marquee/move LOGIC is backend-independent.

## Goldens (`/missions/smoke/edit`)
| Golden | Value |
|--------|-------|
| Seeded slots | 8 (`__missionDoc.slot_count()`) |
| Marquee box `[690,420,750,480]` around seed 0 (`s4`) | selects `["s4"]` (count 1) |
| Drag `s4` from `[720,450]`→`[760,450]` | `slots_digest()` changes; `s4` stays selected |
| `edit_persist_count()` after the move | `0 → 1` |
| dist `website-leptos-*_bg.wasm` | **5,247,496 B** (Δ **+14,518** vs T-159.18 `5,232,978`) |

## Gate results (all exit 0)
**New — `editor-marquee-drag-smoke`** (WebGL2):
```json
{
  "gate": "editor-marquee-drag-smoke", "path": "/missions/smoke/edit?force=webgl",
  "ready0": true, "ready": true, "marqueeSelfcheck": true,
  "marquee": { "rect": [690,420,750,480], "expectCount": 1, "count": 1, "ids": ["s4"], "ok": true },
  "move": { "id": "s4", "from": [720,450], "to": [760,450],
            "digestChanged": true, "selected": true, "editPersistFired": true, "c0": 0, "c1": 1 },
  "panics": [], "pass": true
}
```
**Regression (all `pass: true`, `panics: []`, exit 0):**
- `editor-smoke` — pass.
- `editor-selfcheck` — pass (webgl2).
- `editor-pan-smoke` — pass (webgpu).
- `editor-doc-smoke` — pass.
- `editor-persist-smoke` — pass.
- `editor-select-smoke` — pass.

## Build
- `cargo check --target wasm32-unknown-unknown` — clean (no warnings).
- `trunk build --release` — clean (`Finished release … success`; dist produced).

## Non-goals held (T-159.19)
- **M6:** live preview is direct per-move (`set_drag` / `upload_marquee`), no rAF coalescing — mirrors
  the incremental-pan channel (also per-move). Drag/selection previews are visual no-ops pending the
  atlas slice; only the marquee rect renders.
- **M7:** cluster drill, Attributes dblclick, undo stack UI, compiler save/export (.20), Eden docked
  shell, GpuTimer — all out of scope.

## Next
Ready for Cursor → **T-159.20** (save/export). Return: SHA + tag `T-159.19` + this log.
