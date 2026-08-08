# Wave 130 — adversarial verify (T-723 · T-760 · T-769)

**Verified HEAD:** `0c930b321bd107c8da0548cdf9a77ec184f3ae93`  
(`0c930b32` wave 130 fixup: rustfmt mission_editor + mission_history)

**Wave base:** `9bcc4758` (wave 106 CLOSED — editor wave 129) — ancestor of HEAD.  
**Merges in wave:** `c0505a9e` (T-723), `5bc5e60a` (T-760), `5ed4e4f1` (T-769), `0c930b32` (rustfmt fixup).

**Method.** HOST cargo only. Private targets under `~/.cache/tbd-target-wave130-{verify,perturb,base}` (deleted before this report). Mutations only in detached worktrees at HEAD / base (`~/.cache/tbd-verify130-{perturb,base}` — deleted). Main checkout left byte-clean (`git status` empty; HEAD unchanged).

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD `--list` | **966** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD run | **966 passed / 0 failed** | same binary; `--list` == run |
| base (`9bcc4758`) `--list` | **954** | isolated worktree + private dir |
| `t723_armed_place::*` | **11** | listed + all green |
| `map-engine-render` | **61 / 61** | private dir |

954 + 11 (T-723) + 1 (T-769 `the_windowed_scroller_is_measured_h_full_not_a_fixed_budget`) = **966**.  
T-760’s new pins live in `map-engine-render` (lane-order / `ALL_LANES`), not in the frontend suite count.

**Wave claim “965/965” is wrong** (off-by-one transcription). Code + suite agree at 966. → finding F2.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | `apps/website/frontend/src/mission_history.rs:331` + `:391` — **T-760 claim “after_doc_change + rebind_engine_from_doc feed markers” is unpinned hollow wiring**

**Evidence.**  
(a) Repo-wide, the only call sites of `e.markers_bind(...)` are those two lines (rebind + `after_doc_change`). Place/undo paths reach the lane only via `after_local_edit` → `after_doc_change` (no other binder).  
(b) Perturbation in the HEAD worktree: replace both `e.markers_bind(&mxy, &mtints);` with `();` → `cargo test -p website-frontend` still **966/966 green**; `cargo test -p map-engine-render` still **61/61 green**.  
(c) Narrower attack: strip only the `after_doc_change` feed (leave rebind) → frontend suite still green under a `markers` filter (no T-760 Class-R needle exists).  
(d) Lane-order / `ALL_LANES` pins in `draw_order.rs` stay green without any live feed — they never examine `mission_history`.  
(e) Contrast: T-769’s measured-height pin **does** go RED when production `h-full` is reverted to `height:420px` (see verified-clean). T-760’s feed has no equivalent pin.

**Impact.** Today’s main *has* the feed and markers can render, but the ticket’s critical claim is gate-invisible. A one-line regression (or a merge that drops the `after_doc_change` arm) ships with a green suite while place/undo leave `LaneRole::MissionMarkers` stale or empty — authored briefing markers vanish from the map with no test failure. This is the standing hollow-pin class (wave 127–129 family #2) applied to T-760’s must-ship feed.

**Disposition — fix this wave.** Add a Class-R / scrubbed-source pin on `mission_history.rs` that requires `markers_bind` inside **both** `rebind_engine_from_doc` and `after_doc_change` (and preferably that `marker_lane_xy_tints` is the sole arg builder). Optional stronger pin: a pure unit test that builds a tiny doc with one briefing marker row, runs the xy/tint helper, and asserts non-empty packs. Perturbation must go RED when either feed line is removed.

---

### F2 — MINOR | wave record / ticket claim “list/run 965/965” — **suite arithmetic is 966, not 965**

**Evidence.** HEAD `--list` = 966; run = 966; base `9bcc4758` = 954; +11 T-723 +1 T-769 = 966.  
**Impact.** Misleading verify log / handoff number only; no functional risk.  
**Disposition — fix this wave.** Correct the wave close note / ticket verify string to **966/966** (and note map-engine-render 61 separately if T-760 pins are cited).

---

### F3 — MINOR | `apps/website/frontend/src/eden_dock_right.rs:2922` — **stale comment: “a release over chrome drops it”**

**Evidence.** Markers panel armed-state banner comment still says the one-shot arm is dropped on chrome release. T-723’s contract (and wasm `ArmedUp::KeepArmed` at `mission_editor.rs:4169-4172`) is the opposite: off-canvas LMB must **not** `cancel_pending`. Admitted untested dock `pointerdown` arming still relies on KeepArmed for click-then-click.  
**Impact.** Doc lie next to live UI; can re-introduce the wave-106 MAJOR-1 “fix” of cancelling on chrome release.  
**Disposition — fix this wave.** Rewrite the comment to match KeepArmed (Esc/RMB cancel; chrome release keeps the arm).

---

### F4 — NIT | `engine.rs:4436` vs `mission_history.rs:424` — **xy naming `[x0,y0]` vs `[x,z]`**

**Evidence.** `markers_bind` docs say interleaved `[x0,y0,…]`; the feeder pushes briefing `x` then `z` into the second float (map horizontal plane). Behavior is consistent with slot packing; names disagree.  
**Impact.** Reader confusion only.  
**Disposition — fix this wave** (one-line doc align) or leave; not behavioral.

---

## Claim attacks (by ticket)

### T-723 — armed-placement pointerup

| claim | result |
|---|---|
| button 0 only places; KeepArmed off-canvas | **HELD** — `decide_armed_pointerup` + 11 sequence tests green; adversarial extras (button 3 Ignore; full chrome down+up KeepArmed; Esc after KeepArmed) all passed then removed |
| always clear stranded left on armed up | **HELD** — wasm `left.take()` before decide; sequence tests ClearLeft |
| RMB / Esc disarm | **HELD** — wasm `button==2` → `cancel_pending`; Escape arm before measure seam; sequences green |
| KeepArmed enables click-then-click with dock pointerdown | **HELD** as designed; admitted untested dock `on:pointerdown` still present (`eden_dock_right` palette/composition/markers) — not a defect given KeepArmed |
| picker arms on `click` not place-at-row screen pos | **HELD** — diff `on:pointerdown` → `on:click` in AssetPickerOverlay |
| `select_tool::may_promote_pending` | **HELD** — wasm pointermove calls it (`mission_editor.rs:3894`); mirrors `armed_place::may_promote` |
| 11 event-sequence tests | **HELD** — all 11 listed and run |

### T-760 — briefing markers lane

| claim | result |
|---|---|
| `LaneRole::MissionMarkers` between Zones and SquadLinks | **HELD** — `mission_markers_sit_between_zones_and_squad_links` green; order asserts |
| `markers_bind` via `pack_icon_instance` + `SLOT_GLYPH_DISC` + `upload_slot_role_lane` | **HELD** — body inspection |
| **not** on `slots_bind_soa` / pick bridge (`last_ids`) | **HELD** — `markers_bind` never writes `last_ids`; soa body has no `MissionMarkers` |
| `ALL_LANES` length pin | **HELD** — removing `MissionMarkers` from `ALL_LANES` → `all_lanes_covers_every_variant` RED |
| after_doc_change + rebind feed | **CODE PRESENT, UNPINNED** → **F1 MAJOR** |

### T-769 — measured dock tree height

| claim | result |
|---|---|
| windowed scroller `h-full min-h-0`, not fixed 420 | **HELD** — live class + pin |
| `CONTAINER_H_FALLBACK` only until measured | **HELD** — renamed; pin forbids `CONTAINER_H:` driver |
| smoke v3 formula `min(total, ceil(H/16)+12)` + `H>420` + fills parent | **HELD** — matches `ROW_H=16`, `OVERSCAN=6` (`2*OVERSCAN=12`); python equality over H∈[1,2000] |
| class_r_scrub pin goes RED on fixed-420 regression | **HELD** — worktree mutation → pin FAILED |
| false CONTAINER_H comment corrected | **HELD** — comments describe fallback + smoke coupling |

---

## Standing attack surfaces (wave 127–129)

| surface | attack | result |
|---|---|---|
| 1. z=None flatten on x/y write | GREEN pins `an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` + placement twin; force `keep_z_rows` → `None` → pin RED | **FAILED to break** (fix still holds) |
| 2. Hollow source pins | T-769 delete/revert measured scroller → RED; T-760 delete `markers_bind` feeds → still GREEN | **T-760 feed hollow → F1**; T-769 solid |
| 3. Stale thread_local hooks / `Rc::ptr_eq` | `t754` zone seam + affordance suite 14/14 green; no new hook in this wave’s owns | **FAILED to break** |
| 4. Affordance clickable IFF | `subject_id_routes` / t754 affordance pins green (entity arm present from wave 129) | **FAILED to break** |
| 5. Shared CARGO_TARGET_DIR lies | private `~/.cache/tbd-target-wave130-*` only; `--list`==run 966; dirs deleted | **FAILED to break** |

---

## Main safe to build the next wave on?

**yes** — with **F1 fixed in-wave** (orchestrator NO-DEFERRAL). Main is not broken today; the open risk is regression-blind T-760 feed wiring, not a live data-loss bug on HEAD.

---

## Verified-clean register (claims re-proved)

- T-723: 11/11 `t723_armed_place` + 3 adversarial sequences; wasm button filter / Esc / KeepArmed wiring inspected; picker `on:click` arm confirmed.  
- T-760: lane order; `ALL_LANES` perturbation RED; pick-bridge isolation (`last_ids` untouched); `pack_icon_instance` + `SLOT_GLYPH_DISC`; feed **present in source**.  
- T-769: formula ≡ smoke; fixed-420 perturbation RED; scrub pin live.  
- Suite: 966 list == 966 run; base 954; map-engine-render 61.  
- Main: `0c930b32`, clean tree after cleanup.

---

## Attacked and FAILED to break

1. **KeepArmed / button filter / Esc / RMB / ClearLeft** — native sequence machine + adversarial chrome/button-3/Esc-after-keep cases.  
2. **Picker place-at-row screen position** — arm moved to `click` (diff + comments).  
3. **MissionMarkers pick-bridge pollution** — `markers_bind` does not write `slot_bridge.last_ids`.  
4. **Lane order / ALL_LANES completeness** — order green; completeness pin catches removal.  
5. **T-769 measured-height formula vs smoke v3** — arithmetic identity; fills-parent / H>420 contract intact.  
6. **T-769 hollow pin** — production revert goes RED (pin is real).  
7. **z-flatten / keep_z_rows** — pins green; hollow attack on `keep_z_rows` goes RED.  
8. **Affordance / subject_id_routes / zone hook cleanup** — t754 suites green; no new dead-click in wave owns.  
9. **Private-target list/run mismatch** — none (966==966).  
10. **Admitted dock pointerdown arming** — still true, but KeepArmed makes click-then-click work as claimed (could not break the intended contract without changing dock owns).

---

## Focused re-verify (194cdebe)

**Scope.** Fix commit only: `194cdebe` (F1/F3/F4). Do not fix / commit / ticket. Main left byte-identical (`HEAD` = `194cdebe`; tracked tree clean; only untracked prior verify artifact).

**Method.** HOST cargo. Private `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-w130-reverify`. Perturbations only in detached worktree `~/.cache/tbd-verify130-reverify` @ `194cdebe` (removed after). Main never mutated.

### Suite (F2 note-only)

| measurement | value |
|---|---|
| Prior full verify (pre-fix `0c930b32`) | **966** list == **966** run |
| Post-fix HEAD (`194cdebe`) | **967** list == **967** run |
| Delta | **+1** = `mission_editor::t760_markers_bind_feed::rebind_and_after_doc_change_both_feed_markers_bind` |
| `map-engine-render` | **61 / 61** |

F2 remains note-only: the pre-fix “965” claim was wrong at 966; after the F1 pin lands, live totals are **967/967**. No functional issue.

### Attacked claims

| id | claim | result |
|---|---|---|
| **F1** | Class-R pin `t760_markers_bind_feed` — delete either `markers_bind` feed → RED; both present → green | **HELD** |
| **F3** | `eden_dock_right` Markers armed banner comment matches KeepArmed (Esc/RMB cancel; chrome/off-canvas LMB keeps arm) | **HELD** |
| **F4** | `engine.rs` `markers_bind` docs say map-plane `[x0,z0,…]` (not misleading `y0`) | **HELD** |
| **F2** | suite arithmetic note | **HELD** as note — now 967 post-pin |

**F1 perturbation evidence** (worktree; restore + `touch mission_editor.rs` so `include_str!` rebuilds):

1. Baseline both feeds → pin **ok** (exit 0).
2. Strip `after_doc_change` feed only → pin **FAILED** exit 101 with `T-760: after_doc_change must call markers_bind`.
3. Restore; strip `rebind_engine_from_doc` feed only → pin **FAILED** exit 101 with `T-760: rebind_engine_from_doc must call markers_bind`.
4. Restore both → pin **ok** (exit 0). Hist sha restored; both live call sites at `:331` and `:391`.

**F3 evidence.** Live comment at `eden_dock_right.rs:2922`: `T-723 KeepArmed: Esc/RMB cancel; off-canvas LMB keeps the arm`. Old “release over chrome drops it” string **gone**. Aligns with `ArmedUp::KeepArmed` / “off-canvas must NOT cancel_pending” in `mission_editor.rs`.

**F4 evidence.** `engine.rs:4436` — `map-plane xy ([x0,z0,…]; feeder pushes x then z)`. Feeder doc at `mission_history.rs:424` already `[x,z,…]`. Old `[x0,y0,…]` wording removed from this API doc (vehicles still use `[x0,y0,…]` — unchanged, out of scope).

### Fix-commit regression scan (`git show 194cdebe`)

Touches only: `eden_dock_right.rs` (1 comment), `mission_editor.rs` (+51 test-only pin module), `engine.rs` (1 doc line). **No** production logic / bind-path / z-write changes.

| surface | result |
|---|---|
| New stale `thread_local` | **none** in diff |
| New hollow pin | **FAILED to re-hollow** — F1 pin goes RED on either feed delete |
| New z=0 / keep_z flatten | **none** in diff (only removed misleading `y0` doc token) |

### Findings this pass

**0B / 0M / 0m / 0N** — expect clean; all attacked fixes hold.

### Main safe to build the next wave on?

**yes**

### Attacked and FAILED to break (this focused pass)

1. **F1 hollow feed** — deleting either `markers_bind` feed turns `t760_markers_bind_feed` RED; both present green.
2. **F3 chrome-drop comment lie** — rewritten; old string absent; KeepArmed wording present.
3. **F4 y0 naming lie on markers_bind** — docs now map-plane x,z.
4. **Fix commit sneak regressions** (thread_local / hollow / z=0) — none introduced; only comments + native Class-R pin.

**Cleanup.** Private target + verify worktree deleted after this section was written.
