# Editor wave 201 — adversarial verification

**Target:** merged main @ `2ad1ad41` (T-793 close). Base `b10c4c89` (wave-123/209 close).
**Merges:** `b2af39bc` T-807 · `47555784` T-791 · `2ad1ad41` T-793.
**Method:** live stack (SPA :3000 serving merged wasm `39f88235…`, API :8080), real CDP
input via `scratchpad/cdp.py` (per-char keys, dispatchMouse press/drag/release, MMB pan,
mouseWheel zoom), plus the ticket pins re-run in a private target dir. Served wasm confirmed
to carry `armed_composition_id`, `edge_label_key`, `SNAP` (grep -a on the dist blob).

Severity key: BLOCKER = main broken / data at risk / gate lied · MAJOR = shipped ticket does
not do what it claims or can destroy authored work · MINOR/NIT = everything else.

---

## Findings

`SEVERITY | file:line | what is wrong | how proved`

**NIT | apps/website/frontend/src/mission_editor.rs:3665 (+ eden_help.rs, no modal_stack::register) | With an armed composition AND the Controls Hint overlay open, a single Esc clears BOTH the overlay and the arm (not a one-layer-per-press ladder). | Live: arm VERIFY201G → open View ▸ Controls Hint → overlay open AND arm hint still live (`hint_after_overlay_open: true`); ONE Esc → `{overlay:false, hint:false}`. Mechanism: eden_help.rs registers 0 times in `modal_stack`, and the editor arm-cancel Esc is gated on `modal_stack::any_open()` (line 3665) which the Controls Hint does not trip, so the top-strip closes the overlay and the editor cancels the arm on the same keydown. Pre-existing (T-723 generic arm-cancel); T-791 only made it *visible* for compositions. No data risk — the arm places nothing; cost is one re-arm click.**

**NIT | apps/website/frontend/src/eden_toolbelt.rs:818 | Grid-ref labels recompute on the cursor signal (every pointermove) and the ~1 Hz zoom heartbeat, so a wheel-zoom with no pointer movement leaves labels stale up to ~1.4 s before they snap to <0.05 px accuracy. | Live: immediately after a wheel-zoom, max label error 680 px; after 1.4 s heartbeat, ≤0.05 px at all 3 zoom levels. Not the O-2 defect (which was *permanent* post-pan staleness) and outside T-793's acceptance (discrete zoom levels at rest, + every-frame PAN — both pass). Self-correcting; noted only for completeness.**

No BLOCKER. No MAJOR. The review's headline F-30 "silent dead-end stamp" is **not reproduced** —
the composition stamp works live (see below). T-791's slice claim (stamp already shipped; it
added the armed-state hint) is TRUE.

---

## Is main safe to build the next wave on? **YES.**

---

## Verified-clean register (re-proved claims with measured numbers)

### T-791 — armed composition places on map click; Esc disarms (F-30, the big one)
Full acceptance re-run LIVE with real CDP, seed doc `/missions/smoke/edit`:
- Saved a 3-entity composition (2× Rifleman + Grenadier, marquee-selected = exactly my 3),
  armed by clicking the saved row → **panel hint appears** ("Placing "VERIFY201C…"",
  "Click the map to stamp it at the cursor. Esc or right-click to cancel.").
- **Plain click** on the map: OBJ 11→14 (**+3**), slot_count +3, **SEL == the 3 stamped ids**
  (`n5,n6,n7`, disjoint from the 3 saved), depth +1 (**one txn**). Hint gone after stamp.
- **ONE Ctrl+Z removes all 3**: slot_count 14→11, depth −1 (one undo step). Confirmed
  `removed_all_3: true, one_step: true` even after a dblclick had changed the selection.
- **Second point** (review's "two separate points"): slot +3, depth +1. **Press-drag-release**
  gesture: slot +3, depth +1, camera unchanged (not read as a pan). Both stamp.
- **Re-arm → Esc → hint GONE → click map: OBJ unchanged** (slots & depth flat). Also RMB
  cancels the arm.
- **Stored elevation preserved through stamp (z-family):** authored member-1 Z = **12.500** m
  before save; the stamped member-1 reads Z = **12.500** m (offset kept, not 0.0-stamped);
  stamped Role = "US Rifleman" intact.
- Console/pageerror **clean** throughout (only the unrelated preload-`integrity` warning and
  pre-auth 401s); **0 panics**; digest returned exactly to baseline; comp rows undone away.

### T-791 × T-814 — Esc interplay
- arm + Attributes: opening Attributes requires a canvas dblclick, whose first click **stamps**
  (consuming the arm) — the two cannot coexist; not a defect, arm is never "unreachable".
- arm + Controls Hint overlay: arm survives opening; one Esc clears both — see NIT above.
- arm + RMB on canvas: RMB cancels the arm (Eden stamp-cancel) and opens the context menu.

### T-793 — grid reference labels derive from the live camera (O-2)
- Initial @ z −2 (4 m/px): 6 eastings, adjacent gap **exactly 250.0 px**, max error vs
  CUR-unproject oracle **0.00 px**.
- **240 m pan** (the review's failing distance): labels `040…090`, gap **250.0 px**, error
  **0.00 px** — the O-2 "70 px @ 4 m/px, two labels that cannot both be true" is fixed.
- **Mid-pan continuity:** two samples in one continuous MMB drag differ correctly
  (`040–090` → `050–100`), both 0.00 px error — every-frame update confirmed.
- **3 zoom levels at rest:** ≤0.05 px error each.
- **CUR is metre-accurate** (the acceptance oracle): shown `X 7692.000 m` vs predicted 7692.000
  (err −0.000), `Y 6640.000 m` (err 0.000); `Z −18.397` (DEM-sampled). `' m'` suffix present on
  X/Y, absent on the em-dash Z cell (`fmt_coord_eden`).
- Source: `fmt_coord_eden` wraps `fmt_coord` — number byte-identical, `' m'` is presentation
  only; `<For key=|l| l.key.clone()>` (position-quantised whole-pixel key), not `l.text`.

### T-807 — copy/consistency sweep (all seven)
- (a) Transform tab: X/Y/Z each render a `' m'` unit leaf (3 found) + `°` on rotation (1);
  values 3-dp. **No-op discipline:** open X on a value, blur without typing → depth Δ0, digest
  unchanged. **Field-Esc two-stage:** type "9" into X → Esc reverts to 5360 with dialog still
  open (depth Δ0, digest same) → 2nd Esc closes. No write on either.
- (b) ORBAT header pluralizes: live "**10 slots** · server cap 128 players"; the `== 1`
  conditional present (perturbation proves it — below).
- (c) DEM hint: live shows "Z is **sampled** from terrain elevation (DEM); edit it here to
  override." — stale "Z is manual until…" gone from rendered UI (the 2 source hits are the
  test's doc-comment + assembled `.concat()` needle, both scrubbed out of the pin haystack).
- (d) Library hero gate: API-created **draft** "VERIFY201 M1" appears in the library with a
  **Draft** status chip and **NO "Live Operation"** hero anywhere (`any_live_operation: false`).
- (e) Fresh mission console: **0** `[yrs-persist] refused to persist` WARN (measured on the
  API-created mission and on an emptied smoke doc). `BLOCKED_EMPTY` increment is structural —
  yrs_persist.rs:787 `saturating_add(1)` runs unconditionally inside `if !has_content`, BEFORE
  the `WARNED_EMPTY` once-per-id gate (line 792) that throttles only the `debug` line; the
  `blocked_writes()` bridge reads it. Data-safety intact: emptying a persisted doc left the
  stored blob's content in place (guard refused to clobber — `stored_has_content` stayed true).
- (f) Every disabled context-menu row exposes a non-empty title, both takes: "Play from Here"
  / "Play as the Character" → "Preview launch is a mod-side feature, not available in the
  editor"; Edit/Log/Grid/Select all titled. `empty/entity_disabled_missing_title: []`.
- (g) Bottom-right chip reads **SNAP** ("SNAP  off" default; "SNAP  move off · rot off" after G);
  no `"GRID  "` readout left in source; G-flip restores.

### Register re-checks (waves 200/209 machinery — no regression)
- Identity ROLE typing: 11 chars land ("Fireteam Ld"), node survives, focus kept
  (`INPUT#Role`), **SNAP chip immune** ("SNAP  off" before and after) — IMMUNITY invariant holds
  (chip now reads SNAP, adjusted).
- Multi-edit differing-field zero-typing blur: dialog header "2 slots selected · multi-edit",
  Role shows placeholder "Multiple values"; focus+blur without typing → depth Δ0, digest same.
- Transient closer: context menu open → dblclick entity → menu gone.
- ORBAT z_class: elementFromPoint at the ORBAT dialog top resolves into the overlay
  (`orbat_topmost_probe: true`); closes on Esc.
- Layout: at 1920×1080 both docks `bottom == 1044 == bar.top` (dock.bottom == bar.y); status
  bar 36 px flush to viewport bottom; bar-end hit-tests land in the bar subtree.
- Named pin `ui::tests::orbat_manager_overlay_derives_z_from_the_modal_stack` — **passes**.

### Pins / gate machinery
- Module suites green: 178 (eden_dock_right + editor_ops + …) + 8 (attributes t807) + 5 named
  (t793 keying, eden coord unit, orbat z, status-bar axis, live-camera property) — 0 failed.
- **Hollow-pin perturbation (2, restored byte-exact):**
  - `eden_toolbelt.rs:872` `l.key`→`l.text` → `grid_ref_for_is_keyed_by_position_not_text`
    FAILED with the O-2 message. Restored (git diff --exit-code clean).
  - `orbat_manager.rs:301` `== 1`→`== 2` → `cap_label_pluralizes_the_slot_count` FAILED with
    the F-17 message. Restored (git diff --exit-code clean).
  Both pins are load-bearing, not hollow. The T-791 `a_composition_captures_comments_and_
  authored_elevation` pin cross-checks the `"elevation"` contract string on BOTH the capture
  (editor_ops) and place (map-engine-core store.rs) halves via scrubbed `only_body` — robust.

---

## Attacked and FAILED to break
1. T-791 stamp is dead (review F-30) — **failed**: stamp works live, +3 / SEL==3 / one-undo,
   plain click AND drag AND second-point all stamp; elevation preserved.
2. Stamp writes per-member txns (F-26 regression) — **failed**: one txn, one undo step.
3. Stamp zeroes stored elevations — **failed**: Z 12.500 preserved through save→stamp.
4. Esc leaves the arm stranded / hint lies — **failed**: Esc clears arm + hint; RMB too.
5. O-2 labels frozen after a 240 m pan — **failed**: 250.0 px gap, 0.00 px error, mid-pan
   every-frame.
6. Grid labels drift across zoom at rest — **failed**: ≤0.05 px at 3 levels.
7. `' m'` suffix corrupts CUR metre-accuracy — **failed**: CUR err ≤0.000 m, byte-identical.
8. T-807 attrs no-op writes on blur / field-Esc commits — **failed**: depth Δ0, digest same.
9. Draft shows "Live Operation" hero — **failed**: Draft chip, no hero.
10. Debug downgrade swallows real yrs refusals / clobbers a good record — **failed**: counter
    increment is pre-throttle; stored content kept when doc emptied.
11. Disabled context rows silent — **failed**: all titled, both takes.
12. SNAP chip immunity broken during ROLE typing — **failed**: chip unchanged, focus kept.
13. New pins hollow (self-match / raw include_str!) — **failed**: 2 perturbed → RED with exact
    messages; scrubbed haystacks + runtime needles.
14. ORBAT overlay not topmost (z_class) — **failed**: elementFromPoint lands in the overlay.

Environment left exactly as found: HEAD `2ad1ad41`, `git status` clean, private target
`tbd-target-T-793` removed, all chromium killed, VERIFY201 compositions undone + "VERIFY201 M1"
API-deleted (verified absent from the library).
