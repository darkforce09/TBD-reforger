# Editor wave 202 — adversarial verification

**Target:** merged main @ `8a527b20`. Base `abd274b2` (wave-124/201 close).
**Merges:** `76c585ca` T-788 (slice `bff134a7`) · `b03b2c57` T-797 (`89d0e9d7`) ·
`4ce95f90` T-800 (`61b82f96`). **Completion commits on main:** `8dbad948` T-788 F-26
slot-identity one-txn half · `8a527b20` T-797 row-2 toolbar + Edit-menu wire-up.
**Method:** live stack (SPA :3000 debug wasm, API :8080), real CDP input via a stdlib
WebSocket driver (`scratchpad/cdp.py`, SwiftShader `--headless=new`, per-char keys +
`Input.dispatchMouseEvent` press/drag/release + `__editorCamSet`), the ticket pins re-run in
private target dirs (`wave.sh test`), and DB inspection/mutation via
`podman exec tbd_reforger_db psql`. Modpack flip restored; no missions created.

Severity: BLOCKER = main broken / data at risk / gate lied on code it never examined ·
MAJOR = a shipped ticket does not do what it claims, or can destroy authored work ·
MINOR/NIT = everything else.

---

## Findings

`SEVERITY | file:line | what is wrong | how proved`

**MAJOR | apps/website/frontend/src/eden_top_strip.rs:1040 (`widget_is`) + :1058 (`snap_on`); class closures :1468/:1483/:1504 | The row-2 toggle buttons' `TOGGLED_PLATE` active states never react — they are frozen at the first-render default. The Translate button stays highlighted permanently (including while the widget IS in Rotate mode) and the Snap button never highlights (including while snap is enabled), directly contradicting commit 8a527b20's claim that "the strip's toggle plate re-renders when a CHORD flips the state — keyboard and toolbar cannot disagree" and "5 row-2 buttons … live with TOGGLED_PLATE active states". The controls themselves fully work; only the visual active-state feedback is dead. | LIVE: clicked Rotate, then the working step-Increase button climbs `rot 15°` (readout `SNAP move 5 m · rot 15°`) — behavioural proof the variant flipped to Rotate — yet `Rotate widget (2)` className has NO `bg-primary/20` and `Translate widget (1)` KEEPS it (`translate_plate_stuck_on_in_rotate_mode: true`). Snap: readout flips `SNAP off`→`SNAP move off · rot off` (enabled=true) but the snap button className never gains `bg-primary/20` (`snap_plate` false while `snap_is_on` true). Pressing `1`/`2` and clicking the buttons never move the plate across ~15 s and many state changes. ROOT CAUSE (source): the plate `class=move ||` closures read `widget_variant`/`snap` ONLY through `with_editor_toolbar_dispatch`, which is `None` at the strip's first render (the dispatch registers later, mission_editor.rs:3352 inside `on_load`); the first — and only guaranteed — run returns `false` WITHOUT calling `.get()`, so the reactive closure subscribes to no signal and never re-runs. Contrast: the `SNAP` status readout (which reads `snap` directly) and the catalog grammar-hint (reads `catalog` directly) both update correctly in the same headless session, isolating the getter-indirection as the fault. NOTE: no pin asserts plate reactivity, so no gate reported false success — the pin `edit_menu_widget_snap_rows_dispatch_not_disabled` only checks the rows carry a live `MenuAction` and that `with_editor_toolbar_dispatch` appears in source (both true); the dispatch it tests works. The T-797 registry acceptance (buttons+chords, Edit rows, ORBAT, no `T-`, one Controls Hint) is fully met.**

No BLOCKER. No other MAJOR. No data-loss path found; the completion commits' state/txn work is sound.

---

## Is main safe to build the next wave on? **YES.**

The one MAJOR is a confined cosmetic-feedback regression in a completion commit's own claim
(the toolbar's active highlight lies; every action still works). It does not break main, risks
no data, and does not touch the F-26/F-27/F-29 or T-800 shipped behavior — all of which verified
clean below. Build the next wave on `8a527b20`; the plate reactivity wants a follow-up.

---

## Verified-clean register (re-proved, measured)

### F-26 — one-undo, LIVE (the headline; review measured 9→8 per press pre-fix)
- **Uniform-field fan-out (8 slots):** Ctrl+A → 8 selected; map-dblclick a member opens
  `8 slots selected · multi-edit`; Role (an identity field) is a live field (roles agree), typed
  `VERIFY202R`, Enter → all 8 rows took it (`role_rows_after_apply: 8`), depth **+1** (ONE txn),
  digest changed. **ONE Ctrl+Z** → depth back to baseline, `role_rows_left: 0`, digest
  **byte-identical** to baseline. (probe202_a)
- **Differing-field, checkbox-armed (the class the review measured):** edited one slot's Role
  to differ (single edit, depth+1), Ctrl+A → 8, dblclick member → multi header; Role now shows
  placeholder **`Multiple values`** disabled + an `Apply Role to all` checkbox. Ticked, typed
  `VERIFY202M`, Enter → all 8 took it, depth **+1** (total d0+2). **ONE Ctrl+Z** pops ONLY the
  batch (all 8 back to mixed, the original single-edit `VERIFY202A` returns on its one slot);
  a **second** Ctrl+Z pops the single edit → digest byte-identical baseline. (probe202_a2)
- **T-813 no-op latch survives the caller swap:** differing Role, tick + focus + blur, ZERO
  typing → depth Δ0, digest unchanged (both the uniform and differing runs). (probe202_a/a2)
- **T-732 position path still one-undo (unregressed):** Apply-X-to-all across the selection,
  type `5000`, Enter → depth +1; one Ctrl+Z → depth back, digest baseline. (probe202_a)
- **Core pin (runtime, not a scrape):** `update_slots_attr_batch_is_one_undo_step_across_many_slots`
  green; perturbed to a per-id `begin()` → RED `left: 3, right: 1`. The batch reads through
  `self.slots.get()` (the RAW slot map), the same map `update_slot` uses — not a `materialize()`
  filtered view — so a hidden-layer slot in the selection is written like any other (the caller
  `attrs_multi_ids` computes its id set from `materialize()`, i.e. visible SoA rows, so a hidden
  slot is excluded at the caller, never mis-handled in the batch).

### F-27 — keep-multi reachability
- **Map dblclick on a MEMBER of an 8-selection:** opens `8 slots selected · multi-edit`, SEL
  unchanged (`sel_unchanged_by_dblclick: true`). (probe202_a)
- **Outliner-row dblclick on a member:** opens `8 slots selected · multi-edit`, SEL unchanged.
  (probe202_b)
- **Single plain click OUTSIDE the selection still REPLACES; ctrl-click toggles:** from empty,
  plain click → `[s5]`; ctrl-click same → `[]`; plain reselect → `[s5]`; ctrl-click → `[]`
  (additive arm intact). (probe202_c)
- **T-723 armed-pointerup unregressed by the guard:** the `keep_multi` guard sits in the
  non-armed LG::Click branch, BELOW the `if has_pending() { … return; }` armed branch at the top
  of the pointerup handler — mutually exclusive. Live: an armed palette leaf clicked on a selected
  member still placed (slots 8→9), Esc disarmed (no place). (probe202_b + source
  mission_editor.rs:4917 vs :5196)

### F-29 — panel follows the selection
- **Open single → Ctrl+A flips to multi within a frame:** header `Rifleman · s5` → `8 slots
  selected · multi-edit`, modal stays open. (probe202_b)
- **Ticks survive a DOC COMMIT (the separation claim):** with the panel open, ticked `Apply X`,
  then committed a DIFFERENT field (Rotation `45`, fans to all 8, depth+1) — the `Apply X` tick
  was **still checked** after the commit (`f29_x_tick_survived: true`). `refresh_selection_mirrors`
  re-pokes `attrs_open`; `mirror_selection` (which `refresh_docks` runs every commit) does NOT
  touch `attrs_open` — pin `t788_open_attributes_modal_follows_a_selection_change` green.
- **Deselect closes:** empty-ground click clears the 8-selection to 0 (map semantics); the modal
  closes on any outside interaction (and `refresh_selection_mirrors`' close branch is pinned).
  (probe202_b/d)

### T-797 — toolbar reality (actions all work; plates are the MAJOR above)
- Widget Translate/Rotate **switch** (behavioural: step tunes `move` vs `rot`); snap toggle flips
  the `SNAP` chip (`SNAP off`↔`SNAP move off · rot off`); step ± changes the readout
  (`move off`→`move 5 m`→`move 10 m`, `rot 15°`); Select-All (Edit menu) selected all in view.
  (probe202_e/f/g)
- Edit menu rows: Undo/Redo, `Select All on Screen (Ctrl+A)`, `Widget: Translation (1)`,
  `Widget: Rotation (2)`, `Toggle Snap Grid (G)`, `Snap Step — Decrease ([)`/`Increase (])` — all
  live, all with chords. **No `T-` string in any menu; exactly one Controls Hint** (Help only;
  View menu gone). (probe202_e)
- ORBAT opens **from the menu row** (button y0=2, <40 px) and closes an open context menu
  (transient closer); Esc closes it. Census header pluralizes: `8 slots · server cap 128 players`.
  (probe202_e/f)
- Disabled residue tooltips: History button `disabled: true, title: "Version history (soon)"`.
  (probe202_e)
- Pins `edit_menu_widget_snap_rows_dispatch_not_disabled`, `the_hint_has_exactly_one_home_and_it_is_help`
  green.

### T-800 — catalog failure states, Add Vehicle, dev seed
- **Modpack flip (restored):** set `is_current=false` on `00000000-…-0001` (Core Modern
  Expansion), reloaded → BOTH tree tabs show cause **"No modpack is configured, so the asset
  catalog is empty. Set a current modpack, then retry."** + a **Retry** button, and the search
  grammar chips are **hidden** (`grammar_hint: false`). Restored `is_current=true`, clicked
  **Retry (no reload)** → failure cleared (`cause: null`), grammar chips returned
  (`grammar_hint: true`), tree back to Ready. Modpack independently re-verified `…|t`. (probe202_h)
- **Seed idempotency:** re-applied `registry_dev.sql` → **`INSERT 0 0`** ×2, no errors; vehicle
  count still 4. Seed has 4 vehicle rows (M1025, M113, M923A1, M998) + 8 characters + 13 gear.
- **Add Vehicle:** the smoke doc's ORBAT has 8 slots but no squad rows (custom groups gated to
  T-078), so the "+1" live state isn't cheaply reachable — verified per brief via source + pins:
  `orbat_add_vehicle` (editor_ops.rs:4983) mints an id, calls `add_vehicle` + `attach_vehicle`,
  then `after_local_edit` (one undo step); empty-catalog explainer at orbat_manager.rs:1158
  ("No placeable vehicles in the active modpack…"). Pins `orbat_add_vehicle_increases_vehicle_ids`
  and `add_vehicle_empty_catalog_shows_explainer_not_silent_noop` green.
- Pins `catalog_failure_view_names_cause_and_offers_retry`, `both_catalog_failed_arms_use_the_named_failure_view`,
  `grammar_hint_hides_while_the_tree_is_failed` green.

### Register re-checks (waves 200/201/209 must-not-break — no regression)
- Identity ROLE typing: 11 chars land (`Fireteam Ld`), focus kept (`INPUT#Role`), **SNAP chip
  immune** (`SNAP off` before and after). (probe202_j)
- Layout: at 1920×1080 both docks `bottom == 1044 == bar.top` (dock.bottom == bar.y). (probe202_j)
- Grid ref labels track the live camera: after an MMB pan the eastings shift (`040…` → `000…`)
  with uniform spacing (T-793 unregressed; measured at zoom −5, spacing consistent). (probe202_j)
- Composition stamp (wave-201 F-30) unregressed — armed-pointerup arm structurally isolated from
  the keep_multi guard (source) + armed placement proven live (probe202_b).
- Esc: a plain Attributes modal closes on one Esc. The T-816 armed-composition + hint
  double-consume is FILED and unchanged — not worsened.

### Hollow-pin sweep — 4 perturbations across 3 files, verbatim reds (all restored byte-exact)
1. `editor_ops.rs` `attrs_update_slot_multi` batch→per-id loop → `multi_edit_commits_fan_out_to_every_selected_id`
   RED: "F-26: pub fn attrs_update_slot_multi( must commit via update_slots_attr_batch …".
2. `store.rs` batch one `begin()`→per-id `begin()` → `update_slots_attr_batch_is_one_undo_step_across_many_slots`
   RED: "left: 3, right: 1" (a runtime undo-depth pin, not a scrape).
3. `eden_dock_right.rs` dropped the not-Failed gate → `grammar_hint_hides_while_the_tree_is_failed`
   RED: "both the Factions and Vehicles hint renders must be state-gated (found 1)".
4. `editor_ops.rs` dropped `select_slot`'s keep_multi guard → `t788_plain_click_inside_a_multi_selection_does_not_collapse_it`
   RED: "F-27: select_slot must keep a multi-selection that already contains the clicked id".
   The source-scrape pins are anti-hollow by construction: `scrub()` calls `cut_test_module()`
   (removes the test's own needle-bearing asserts from the haystack), `live_code` also blanks
   string literals, and `split_only` panics on 0 (rename/delete) or 2+ (shadow copy).

---

## Attacked and FAILED to break
1. F-26 costs N undo steps (review 9→8) — **failed**: uniform 8-slot apply = ONE Ctrl+Z; ticked
   differing 8-slot apply = ONE Ctrl+Z pops just the batch; digest byte-identical.
2. Batch writes per-member txns — **failed**: store pin proves one undo step; perturb reddens.
3. Batch reads a filtered `materialize()` view (hidden-slot family) — **failed**: it reads the RAW
   `self.slots` map (byte-identical to `update_slot`); hidden ids are excluded at the caller.
4. T-813 no-op latch broke on the caller swap — **failed**: zero-typing tick+blur = depth Δ0.
5. T-732 position one-undo regressed — **failed**: apply-X = one txn, one undo.
6. F-27 dblclick collapses the multi (map + outliner) — **failed**: both keep `8 slots · multi-edit`.
7. Outside-click no longer replaces / ctrl-click arm broke — **failed**: replace + toggle intact.
8. keep_multi guard regressed the T-723 armed-pointerup — **failed**: armed placement still fires.
9. F-29 panel stays stale on Ctrl+A — **failed**: header flips to multi within a frame.
10. A doc commit wipes the per-field opt-in ticks — **failed**: `Apply X` tick survived a Rotation
    commit (the separation claim holds).
11. Toolbar buttons are inert advertisements — **failed**: widget switch, snap, step ±, Select-All
    all dispatch and act. (BUT the active-*plate* feedback is dead — the MAJOR above.)
12. A `T-` string leaks into a menu / two Controls Hint homes — **failed**: none; exactly one.
13. ORBAT opens from a stale row / leaves transients — **failed**: opens from the menu row, closes
    the context menu, Esc-closes.
14. T-800 failure is a flat dead-end again — **failed**: both tabs name the cause + Retry, chips
    hidden; Retry repopulates with no reload.
15. Seed re-apply duplicates rows — **failed**: `INSERT 0 0`.
16. New pins are hollow (self-match / raw include_str!) — **failed**: 4 perturbed → RED with exact
    messages; scrubbed haystacks (test module + literals cut), runtime undo-depth pin.
17. Register regressions (ROLE focus, SNAP immunity, dock==bar, grid-after-pan, comp stamp) —
    **failed**: all unchanged.

Environment left exactly as found: HEAD `8a527b20`, `git status` clean, private target dirs
(`tbd-target-T-788/797/800`) removed, all chromium killed (0 real `chrome` binaries), modpack
`is_current=true` restored + re-verified, no `VERIFY202` missions in the DB, VERIFY202
compositions undone.

## Fixup re-verification (95ee8cb7, focused)

**Target:** fix commit `95ee8cb7` on main (base `8a527b20`, the wave-202 close). Scope: the one
MAJOR only. **Method:** :3000 was dead (connect refused) — served my own debug SPA per brief:
`trunk serve --port 3001` from the repo (API :8080 + DB :5434 already live), headless chromium
149 driven by a fresh stdlib-WebSocket CDP driver (scratchpad `cdp.py`; flags per
`tools/tbd-tools/src/cdp.rs::launch` Swiftshader arm: `--headless=new --no-sandbox
--disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-swiftshader --enable-unsafe-webgpu
--hide-scrollbars --force-device-scale-factor=1` + writable `XDG_CACHE_HOME` fontconfig cache +
`--disable-remote-fonts`, 1920×1080). Real `Input.dispatchMouseEvent` clicks at button rect
centers; real `Input.dispatchKeyEvent` chords (`Digit1`/`Digit2`/`KeyG` — the codes
mission_editor.rs:3811 matches on). Active-plate probe = `className.includes("bg-primary/20")`
(the `TOGGLED_PLATE` literal, eden_layout.rs:449); every toggle cross-checked against the SNAP
status chip text (a direct-signal reader, the prior pass's control).

### 1) Scope purity — PASS
`git diff 8a527b20..95ee8cb7`: **2 files, +147/−3, 213 diff lines total (102 + 111, both read
in full)**. mission_editor.rs: the `TOOLBAR_DISPATCH_GEN` thread_local (`ArcRwSignal<u32>`,
lazily created, owner-independent), `toolbar_dispatch_generation()` getter,
`bump_toolbar_dispatch_generation()` (untracked read + wrapping_add, wasm-gated), register now
bumps, new `unregister_editor_toolbar_dispatch()` (clear + bump), and
`on_cleanup(unregister_editor_toolbar_dispatch)` at the on_load register site — plus doc
comments. eden_top_strip.rs: one `let _gen = …toolbar_dispatch_generation().get();` line at the
TOP of each getter (`widget_is`, `snap_on`), the test-import add (`only_body`), and the new pin.
No dispatch invoker, widget/snap semantics, engine, or production markup touched. The 3 deleted
lines are the register doc-comment rewrite + the import line. Getters' generation read sits
OUTSIDE the wasm cfg (the signal is target-independent) — native behavior unchanged (both cfg
arms still return the same values).

### 2) The parities, live (the original failing probe, reversed) — ALL PASS
Doc: created `VERIFY202F main` / `VERIFY202F alt` via POST /missions (everon, pve_coop, 16) —
the library was empty. Observed (JSON = {translate_lit, rotate_lit, snap_lit, snap_chip}):
- **Baseline** T0: `{true, false, false, "SNAP off"}` — Translate lit by default, correct.
- **(a) click Rotate** T1: `{false, true, false}` — **Rotate GAINS the plate, Translate LOSES
  it** (the exact probe that froze on 8a527b20: `translate_plate_stuck_on_in_rotate_mode` is
  now false).
- **(b) chord `1`, no click** T2: `{true, false, …}` — Translate returns. Chord `2` T3:
  `{false, true, …}` — Rotate. Keyboard alone moves the plate.
- **(c) snap via BUTTON** T4/T5: snap_lit false→**true**→false, chip `SNAP off` ↔
  `SNAP move off · rot off` in lockstep. **Via `G`** T6/T7: same, both directions. Plate and
  chip (independent readers) never disagreed across 20 measurements.
- **(d) second mission, return** T8–T14: opened `VERIFY202F alt` (`/missions/:id/edit` →
  same route pattern: a PARAM navigation — Leptos keeps the component mounted, so
  widget=Rotate correctly SURVIVED, T8 `{false, true, …}`), returned to main, then click
  Rotate / chord `1` / `G`×2 all tracked (T11–T14) — plates live after the mission switch.
  **Route-LEAVE remount** (the on_cleanup path — my extra rigor, editor → /missions →
  history.back()): the UNMOUNT half is clean (strip torn down, library healthy, no
  stale-dispatch read, no panic from the teardown), but the REMOUNT half **cannot complete in
  this headless environment on ANY commit** — see the pre-existing finding below. The
  register-from-None + first-bump path is independently proven by every fresh load (T0/T15-base
  boots), and the param-switch trip above is the user-visible "open a second mission, return".

**Pre-existing, NOT this commit (documented, not fixed):** a SECOND wgpu engine boot inside one
wasm session dies under headless software WebGPU: `panicked at wgpu-29.0.4/…/webgpu.rs:2331 …
RangeError: Failed to execute 'createBuffer' on 'GPUDevice': createBuffer failed, size (32) is
too large for the implementation when mappedAtCreation == true` (Chrome's signature for
createBuffer on a lost/destroyed device — a 32-byte buffer is not "too large"), then the usual
post-abort RefCell cascade; app dead, 0 buttons. **Controls:** (i) reproduces with a REAL
`history.back()` popstate, not just synthetic pushState — not a driver artifact; (ii)
reproduces **byte-identically on base `8a527b20`** (worktree served on :3002 — which also
re-proved the original MAJOR forward: base click-Rotate left `{true, false, …}` frozen and `G`
flipped the chip to `SNAP move off · rot off` with snap_lit still false); (iii) the 95ee8cb7
diff contains zero engine/device code (§1). Environment-conditional (SwiftShader/lavapipe
second-device loss; cdp.rs itself documents the live engine needs the Vulkan arm) — wants a
real-GPU repro before filing as a product ticket. It does not gate this fix.

### 3) The pin, perturbed — RED verbatim, restored green
Removed only the `let _gen = …` line from `widget_is` (the frozen shape) →
`plates_subscribe_to_dispatch_generation_before_reading_it` **FAILED**:
> wave-202: `widget_is` must read the dispatch generation (`toolbar_dispatch_generation()`) so
> its plate closure subscribes from frame one — not found

Restored byte-exact (sha256 `f06d75f6ccc91f5b…` matches the pre-perturbation snapshot) + touch
→ green (1 passed / 1104 filtered). **Anti-hollow confirmed at the implementation:** the pin
reads `live_code(include_str!(…))` — arsenal.rs `scrub(src, false)`: comments masked, string
literals blanked, `cut_test_module()` removes the pin's own needles — through `only_body`,
which panics on 0 or 2+ marker matches (rename and shadow-copy both RED), and asserts a
byte-offset ORDER of two real calls, not presence.

### 4) Strip-pin spot check + totals — ALL GREEN
One name-filtered run in the T-797 private dir: `the_hint_has_exactly_one_home_and_it_is_help`
(one hint home) · `top_strip_registers_transient_closer_with_modal_stack` +
`close_transients_closes_menu_export_and_hint` (transient closer) ·
`top_command_strip_escape_yields_when_modal_stack_consumed_escape` +
`top_strip_escape_consumed_guard_is_load_bearing` (consume-aware Esc) ·
`the_menus_own_row_one_and_the_toolbar_owns_row_two` (MENUS census) ·
`the_docks_are_one_equalised_width` (carries the STRIP_TOP_PX == 48 assertion) — **8 passed,
0 failed** (cargo summary line). Full slice run: **1105 passed / 0 failed / 0 ignored**, and
`-- --list` reports **1105 tests** — list == run, nothing silently filtered (both numbers from
the cargo test/list summary lines).

**Environment left as found:** HEAD `95ee8cb7`, `git status` clean (+ this untracked report),
both VERIFY202F missions DELETEd (**204, 204**; library back to `total: 0`), base worktree
removed, `tbd-target-T-797` deleted, my chromium + both trunk serves killed (:3001/:3002/:9333
all free). Foreign `tbd-target-T-785F/786F/815-fix` dirs pre-date this pass and were left.

## Verdict (updated): the MAJOR is RESOLVED — main safe to build on. **YES.**
The row-2 plates now track click, chord, and snap state live from frame one (measured, both
directions, plate+chip never disagreeing), the fix commit touches exactly the subscription/
teardown seam it claims, and the order that makes it work is pinned by a scrubbed-source,
perturbation-proven test. The one open observation (second in-session engine boot under
headless software WebGPU) pre-dates the fix, reproduces identically on base, and is outside
this commit's diff.
