# Editor wave 203 — adversarial verify

**Target:** merged main `87e1ff94` (base `7791e20a`). SPA :3000, API :8080, Postgres `tbd_reforger_db` :5434.
**Tickets:** T-792 (Esc cancels zone/trigger draw) · T-789 (Save Version dialog) · T-809 (one merged asset tree per faction + recently-placed + placed-vehicles-in-outliner + recorder seam).
**Method:** stdlib-CDP driver over a fresh headless chromium (playwright chromium-1228, `--headless=new`, SwiftShader; fresh page/profile per probe — second in-page engine boot dies under headless software WebGPU, pre-existing, not filed). Live DOM/rect reads, real `Input.dispatch*`, the DB `is_current` flip (restored), and a `wave.sh test` pin-perturbation pass into a private verifier target dir (deleted).

---

## Findings

`SEVERITY | file:line | what is wrong | how proved`

**MAJOR | apps/website/frontend/src/eden_top_strip.rs:1919 (+ :3236) | T-789's Save Version dialog renders OFF THE TOP of the viewport at BOTH 1920×1080 and 1366×768 — the ticket's headline acceptance ("version input rect fully within viewport, y>=0") is violated live; this is the exact pre-fix bug the ticket claimed to resolve. | Live CDP `getBoundingClientRect` on a fully-booted map page: version input at `y=-22, bottom=4` @1920×1080 and `y=-183.7` @1366×768 (both `y<0`); dialog top-left at `y=-148`/`y=-304`. Computed `top:23.5px`. ROOT CAUSE: the `position:fixed` dialog is mounted inside the top strip's glass `DIV.pointer-events-auto bg-surface-container-lowest/55 … backdrop-blur-xl` (class const at :3236, from T-637) whose `backdrop-filter:blur(24px)` becomes the containing block for the fixed dialog, so `top-1/2 -translate-y-1/2` centers on the 48px strip, not the viewport. CAUSATION PROVEN: setting that ancestor's `backdrop-filter:none` snapped the input from `y=-22` to `y=423` (in view). Screenshot `save_booted.png` shows only NOTES + textarea + Save visible, title/immutability line/version field clipped above the top edge. The pin `is_clamped_on_screen_by_construction` (:3757) is GREEN but hollow for this failure — it asserts the class STRING (`fixed top-1/2 … -translate-y-1/2`, forbids `top-full`), which is present, not the rendered rect. T-789's own change (this wave switched the dialog from `top-full` to `fixed top-1/2` while leaving it mounted inside the backdrop-filter strip instead of portaling via the T-786 modal plumbing the ticket suggested) is what produces the off-screen result.**

**MINOR | apps/website/frontend/src/eden_dock_right.rs:1724-1742 (cause branch :746) | With `is_current=false` (API 404 "no current modpack configured"), the live failure view shows the GENERIC "The request to the registry failed." cause and keeps the side chips (BLUFOR/OPFOR/INDFOR) visible — NOT the no_modpack-specific "No modpack is configured…" message with chips hidden that attack 3d expects. | Flipped `modpacks.is_current=false`; the no_modpack probe endpoint `GET /registry?limit=1&offset=0` returns 404 `{"error":"no current modpack configured"}` (curl-confirmed), so the `no_modpack.get()` branch at :746 SHOULD fire — but two fresh-token pages both rendered the else-branch text and left the chips up. The grammar hint (`class:`/`mod:`) DID hide, and the failure view + Retry + (post-restore) repopulate all work. Possibly an AuthStore-timing/headless artifact in the dock's re-probe Effect (`let Some(auth) = auth else return`); wants a non-headless confirm. Not data-affecting.**

**MINOR (pre-existing, NOT this wave) | apps/website/frontend/src/mission_editor.rs:2763 · mission_commands.rs:820 | The Save Version prefill is a static `"0.1.0"` with no auto-bump from the mission's persisted latest version; `save_now` never bumps `save_semver`. Reopening after saving 0.1.0 re-proposes 0.1.0 → a 409 immutable-version collision. The ticket's "keep the 0.1.0→0.2.0 auto-bump" is not met. | `save_semver` is only ever set at :2763 (static init) or by the dialog's `on:input` — no other write site exists (grep of the whole frontend). Live: saved default 0.1.0, reopened → prefill still `0.1.0` (×2 reopens); backend `current_version.semver` stayed `0.1.0` (second same-semver save 409'd). The init line is >1300 lines from this wave's mission_editor hunks — untouched by the wave, so a pre-existing limitation, not a T-789 regression.**

**MINOR/NIT (pre-existing) | apps/website/frontend/src/mission_editor.rs (dblclick container listener ~5790) | A double-click on a LEFT-OUTLINER row (or any dblclick whose world coords miss an entity) opens the empty-ground asset PICKER (T-647 PLACE-003) *underneath* the Attributes dialog it also opens — because the left dock is a DOM descendant of the map's dblclick container and the `dblclick` event bubbles (the chrome stops `pointerdown`, not `dblclick`). | `container.contains(outlinerRow) === true`; a native `dispatchEvent(new MouseEvent('dblclick',{bubbles:true}))` on the "US Rifleman" row was received by the canvas container (`container-dblclick@83,135`) and produced `attrH2=1 AND placeAsset=1`. Esc then peels them one-per-press (picker first, then Attributes). Pre-existing (both PLACE-003 and the outliner dblclick predate the wave); now ALSO reachable via the wave-new placed-vehicle outliner rows (eden_tree.rs footer dblclick→open_attributes).**

**NIT | apps/website/frontend/src/mission_history.rs:289/532 | Placing a vehicle does not increment the OBJ readout (OBJ = `slot_count`, characters only); attack 3a's wording "drag-places (OBJ+1 as a vehicle)" is not literally met. | Drag-placed M1025 → OBJ stayed 3, but the vehicle DID place: it appears in the left outliner `PLACED VEHICLES / M1025_M2`, the Vehicles-tab Placed strip (`M1025 Humvee (M2) / 6360.0,5714.0`), and heads recently-placed. So the vehicle is really placed; OBJ is a slot-only counter. Functional, wording nuance only.**

**Note (not a finding) — T-816 (1g):** armed composition + Controls Hint → ONE Esc clears BOTH (arm + hint together). This is the KNOWN wave-201 NIT-1 (T-816 still `queued`, not scoped to this wave). Attack asked "not WORSENED" — behavior is identical (single-press-clears-both), NOT worsened. The hint is not a modal_stack participant, so it cannot claim the Esc; unchanged this wave.

---

## Is main safe to build the next wave on? **YES.**
Main compiles, the full frontend suite is 1122/0, no data is at risk, the DB was left as found, and the gate pins genuinely examine the shipped code (proven by perturbation below). The one MAJOR is a contained runtime layout bug in a single dialog (T-789's on-screen clamp does not hold) — it does not break the build or other tickets, but T-789's clamp deliverable should be re-opened (the fix is portal-the-dialog-out-of-the-backdrop-filter-strip, or apply an explicit `max(0)` viewport clamp; a class-string pin cannot guard it — a live rect assert is needed).

---

## Verified clean (measured)

**Attack 1 — T-792 Esc ladder (live, per-press):**
- (1a) Circle armed, centre clicked, Esc → zone count UNCHANGED (0→0), "click the rim" GONE, next click placed nothing. Re-arm drew a FRESH circle that completed (zone count 0→1).
- (1b) Polygon after 2 vertices, Esc → "of 3 vertices" hint gone, count unchanged (1), next click placed nothing. (1b2) Panel **Cancel** button still cancels an armed draw.
- (1c) Trigger circle: armed → centre set → Esc → "Centre set" gone, next click placed nothing (byte-identical page text before/after) — confirms the **shared `Pending::Zone` state machine** cancels the trigger draw too.
- (1d) Modal ladder peels EXACTLY one layer per press: dump showed `{attr, picker}` → Esc → `{attr}` → Esc → `{}` (dialog/picker closed topmost-first, draft survives to its own press). A dialog-open fires the T-814 transient closers so the hint cannot co-sit under a dialog (correct).
- (1e) Marker armed ("Placing a Dot marker") → Esc → disarmed (page text back to baseline), next click placed nothing.
- (1f) Armed composition ("Placing "V203 Team"") → Esc → disarmed, next click placed nothing.

**Attack 2 — T-789:**
- (2a) On open, `document.activeElement` === `input[aria-label="Version"]`, value `0.1.0` (real CDP focus read).
- (2b) 8 Tab + 8 Shift+Tab = **16/16** kept `document.activeElement` inside the dialog subtree (`dialog.contains(activeElement)`), both edges wrap.
- (2c) Save 0.2.0 (persisted: backend `current_version.semver=0.2.0`), close, reopen → **no stale "Saved vX" line** (0 matching nodes).
- (2e) Save open + Esc → closes Save alone in ONE press, no other dialog opened. T-814: hint open → Save opens via strip button → hint closed (True→False).
- (clamp: see MAJOR above.)

**Attack 3 — T-809 (with the T-800 seed; 4 vehicle rows present in `registry_items`):**
- (3a) NATO → US_Army → **Vehicles** folder holds the 4 seeded leaves: M1025 Humvee (M2), M998 Humvee (Transport), M923A1 Cargo Truck, M113 APC (M2). Drag-placed M1025 → appears in the left outliner + Placed strip + recently-placed head.
- (3b) Palette char place → recently-placed head "US Rifleman" (deduped to one). Composition STAMP of a 3-member selection → OBJ 3→6 AND exactly ONE new recently-placed entry titled **"Alpha Trio"** (the composition title) at the head.
- (3c) Asset search spans the merged tree: `Humvee` auto-expands to the M1025/M998 leaves; `class:character` filters the merged tree to the 8 US_Army characters (no vehicles). Star affordance (`star_border`) present on every merged row incl. vehicles.
- (3d) Failure view via `is_current=false`: shows "Could not load the asset catalog" + **Retry**; Retry stays failed while false; after restoring `is_current=true` a fresh page loads the catalog and the merged tree repopulates (US_Army present). Grammar hint hides during failure. (Cause-text/chips nuance → MINOR above.)
- (3e) Placed vehicle listed in the LEFT outliner (`PLACED VEHICLES / M1025_M2`); clicking the row selects it (SEL 0→1, by doc-presence); delete removes the row (present→false — genuinely gone).
- (3f) Crew-editor stopgap: Vehicles tab keeps the Placed strip rendering the placed vehicle with crew-item count + delete (observed live; pin `vehicles_tab_is_kept_as_a_filtered_view_with_the_crew_strip` green).

**Attack 4 — hollow-pin perturbation (3 across 3 files, restore + touch, verbatim reds):**
Baseline: 10/10 targeted pins GREEN. Perturbed:
1. `mission_editor.rs:3860` `cancel_zone_draw()` → `has_pending()` ⇒ `eden_zones::t792_escape_arm_cancels_the_zone_draw` **FAILED** ("the editor keydown must call editor_ops::cancel_zone_draw() … cancel_pending alone leaves the draft armed").
2. `eden_top_strip.rs:1919` `top-1/2` → `top-full` ⇒ `is_clamped_on_screen_by_construction` **FAILED** (:3769).
3. `editor_ops.rs:5887` removed `record_placed(` from `place_at_impl` ⇒ `eden_dock_right::off_dock_placements_feed_recently_placed_through_the_recorder_seam` **FAILED** (:5234).
Non-perturbed pins stayed GREEN in the same run (surgical, independent). Restored byte-exact (`git status` clean) + touch → all 3 GREEN again.

**Attack 5 — register rechecks (pins, all in the suite):** composition stamp is one-txn / one-undo (live: OBJ 6→3 on one Undo, 3→6 on Redo). Pins present & green: `plates_subscribe_to_dispatch_generation_before_reading_it`, `the_docks_are_one_equalised_width` (STRIP_TOP_PX==48), `grammar_hint_hides_while_the_tree_is_failed`, `recently_placed_is_head_first_deduped_and_capped`, `factions_tab_draws_the_merged_tree`, `a_merged_leaf_press_feeds_recently_placed`, `favourites_and_history_share_one_tab`, `vehicles_tab_is_kept_as_a_filtered_view_with_the_crew_strip`, `merged_tree_reaches_a_vehicle_leaf_under_the_nato_subtree`, `placed_vehicles_are_listed_in_the_outliner_with_slot_affordances`, the four T-789 pins, the three T-792 pins.

**Full website-frontend suite: 1122 passed / 0 failed / 0 ignored / 0 filtered.**

## Attacked and FAILED to break
T-792 circle/polygon/trigger/marker Esc-cancel (incl. shared state machine, re-arm-fresh, panel Cancel) · the one-layer-per-press Esc modal ladder · armed-composition Esc · T-789 focus-first · T-789 16-press Tab trap · T-789 fresh-state-on-reopen · T-789 Esc-closes-Save-alone · T-814 hint-closes-on-Save-open · T-809 merged per-faction tree + 4 seeded vehicle leaves · drag-place vehicle · palette/composition recorder (one titled entry, dedup) · composition one-txn/one-undo · `class:`/plain search over the merged tree · outliner placed-vehicle list+select+delete · failure view + Retry + restore-repopulate + grammar-hide · the 3 perturbed pins (each RED then restored GREEN) · the full 1122-test suite.

## Environment left as found
HEAD `87e1ff94`; `git status` clean (this report is the only addition). `modpacks.is_current=true` (flipped false twice for 3d, restored both times; verified `t`). Seed intact (4 vehicle `registry_items`). All `VERIFY203 *` missions DELETEd (0 remain; API 204s). Chromium killed (:9333 free). Private target dir `~/.cache/tbd-target-wave203-verify` removed.

---

## Fixup re-verification (940698e8+93dc7397, focused)

Re-verify of the one MAJOR only (Save Version dialog centered on the strip's `backdrop-filter` containing block). HEAD `93dc7397`, tree clean + this report untracked. Same stdlib-CDP method as the full pass (fresh chromium-1228 `--headless=new`, SwiftShader, fresh page/profile per probe, one engine boot per browser); builds and the gate ran host-side via `distrobox-host-exec` (this session's container has no `cc`) into private dirs `~/.cache/tbd-target-wave203r-verify` + `~/.cache/tbd-wave203r-scratch` + wave.sh's `~/.cache/tbd-target-T-789` — all deleted after.

**1 · Scope purity — CLEAN.** `87e1ff94..93dc7397` is exactly three commits, three files:
- `940698e8` → `apps/website/frontend/src/eden_top_strip.rs` (+95/−15: `use leptos::portal::Portal` + `<Portal>` wrap of the dialog view; `size_line` → `StoredValue` so the Portal `children` closure is `Fn`; pin renamed `is_clamped_on_screen_by_construction` → `dialog_carries_the_centering_classes_rect_is_smoke_proven` with corrected class-guard-only prose + a `body.contains("<Portal>")` sentinel) and `tools/tbd-tools/src/smokes.rs` (+132: `smoke_save_dialog_rect`, `EDITOR_SUITE` 18→19, `run_smoke` arm).
- `1ce55808` → `.ai/tickets/registry.json` ONLY (+86/−1): T-820/T-821/T-822/T-823 — my four prior findings filed 1:1.
- `93dc7397` → `smokes.rs` rustfmt only (two const-string rewraps + one `wait_for` chain break; zero logic).
Structural invariance holds in the diff: overlay markup, `version_ref` focus-in, `dialog_ref` Tab trap, fresh-state effect, semver prefill, `save_now` wiring and the strip's window-level Esc listener appear only as unchanged context — the Portal tags and the `size_line` access are the only view changes.

**2 · The rect, live (real CDP against :3000, fresh browser per viewport) — IN VIEWPORT at both.**
- **1920×1080:** Version input `top=332.82 bottom=357.78 left=544.96 right=667.84` (ih=1080) → fully in-viewport (was `y=-22`). Label `313.62..328.98` visible. `activeElement === input[aria-label="Version"]` TRUE. 8 real Tabs (`Input.dispatchKeyEvent` rawKeyDown/keyUp) stayed inside the dialog. One real Esc closed it.
- **1366×768:** Version input `top=176.82 bottom=201.78 left=267.96 right=390.84` (ih=768) → fully in-viewport (was `y=-184`). Label `157.62..172.98` visible; focus TRUE; 8 Tabs trapped; Esc closed.
- **Mount proof:** live parent chain of the dialog is `DIV>BODY` — no `backdrop-blur-xl` ancestor anywhere (pre-fix the chain ran through the STRIP_ROWS glass).

**3 · The smoke is real — registered, green at HEAD, and it catches the revert.**
- Read in full: `smoke_save_dialog_rect` drives BOTH shipping viewports via `Emulation.setDeviceMetricsOverride` (reflow before each read), asserts live `getBoundingClientRect` containment (`top>=0 && bottom<=innerHeight`), label-visible, activeElement===input, 8-Tab trap, Esc-close — 6 booleans × 2 viewports = the 12 `checks_pass(&bool_checks, 12)` requires (`*_top/_bottom/_innerHeight` are numeric diagnostics, correctly excluded from the count). Registered BOTH ways: in `EDITOR_SUITE` (19 entries, glob position before `save-export`) which `editor_suite()` iterates (smokes.rs:3568, the `make leptos-gates` chain), AND dispatchable via the `run_smoke` match arm — no gate-lies gap in either direction.
- **Baseline (isolated HEAD dist):** `gate smoke save-dialog-rect --dist …/dist-head` → `"pass": true`, exit 0, 12/12 booleans; its numbers (`v1920 332.82/357.78`, `v1366 176.82/201.78`) match my independent driver to the fraction.
- **Perturbation (Portal wrap deleted → dialog re-nested under the glass, dist rebuilt):** `"pass": false`, **EXIT=1** — `v1920_inViewport=false` with `v1920_top=-183.68, bottom=-158.72`; `v1366_inViewport=false` with `v1366_top=-183.68`; `labelVisible=false` at both; focus/tab-trap/Esc STILL true — the failure is purely the containing block, exactly the class the old class-pin waved through, and the negative-y capture matches the fix agent's own perturbation claim (−183.7). (The original live bug measured −22@1920/−184@1366; the re-nested rebuild lands −183.68 at both — same off-top failure class, the sign is the assertion.)
- **Restore:** `git checkout` byte-exact (`git diff --exit-code` clean at `93dc7397`) + `touch`. Green at restored bytes = the baseline run (identical source), AND the :3000 watcher's rebuilt bundle re-probed live green (`1366×768: 176.82/201.78 in-viewport, mount DIV>BODY`).

**4 · Strip spot-checks — ALL GREEN.**
- Name-filtered pins via `wave.sh test --slice T-789 -p website-frontend` (private dir, deleted): **11 passed / 0 failed** (1111 filtered of the 1122 suite) — `t692_help_surface::the_hint_has_exactly_one_home_and_it_is_help` · `t786_dialog_closes_popovers::{every_dialog_open_path_closes_the_controls_hint, close_transients_closes_menu_export_and_hint}` · `t726_top_strip_esc_stack::{top_strip_escape_consumed_guard_is_load_bearing, top_strip_registers_transient_closer_with_modal_stack, top_command_strip_escape_yields_when_modal_stack_consumed_escape}` · `t668_state_vocabulary::plates_subscribe_to_dispatch_generation_before_reading_it` · `t789_save_version_dialog::{dialog_carries_the_centering_classes_rect_is_smoke_proven, version_input_takes_focus_on_open, clears_stale_status_on_the_reopen_edge, traps_tab_within_the_dialog_subtree}`.
- Live Esc ladder (fresh browser, 1920×1080): hint opened from its ONE home (Help → "Keyboard Shortcuts (Controls Hint)") → Save opened via the strip button → hint auto-closed (T-814 transient closers) → ONE Esc closed Save ALONE (hint still closed, no other overlay) → hint reopened alone → Esc consumed by the hint. All eight observations true: `hintOpen, saveOpened, hintClosedBySaveOpen, escClosedSave, hintStillClosed, noSaveH2, hintReopened, escClosedHint`.

**NIT (new, documented only, not fixed):** `tools/tbd-tools/src/bin/gate.rs:42` — the `gate smoke` clap doc-comment's name list predates this smoke (and `:49` still says "All 17 editor smokes" vs the array's 19). Cosmetic; the dispatch itself is proven above.

**Verdict: the wave-203 MAJOR is RESOLVED and main is safe** — rect-proven live at both shipping viewports, guarded by a real, registered, revert-catching smoke, with the honest-prose pin + Portal sentinel in place and the T-726/T-814 Esc ladder and transient closers behaviorally unchanged.
