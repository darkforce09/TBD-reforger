# Wave 129 — adversarial verify (T-754 · T-759 · T-737)

MERGED MAIN @ `f1c46457` (base `cf9e5272`; merges `4a701be5`, `5b945fce`, `f1c46457`). Main left
byte-untouched (`git status` clean, HEAD unchanged). All verification ran on the HOST via
`distrobox-host-exec` in **private target dirs** (`~/.cache/tbd-target-verify129{,-base,-perturb}`
— never the shared `tbd-target`), with perturbations in a **separate worktree at HEAD**
(`~/.cache/tbd-verify129-perturb`) and the base count in its own worktree at `cf9e5272`. All three
target dirs and both worktrees are **deleted**.

## Suite reconciliation — the brief's number is wrong, the code is right

| measurement | value | how |
|---|---|---|
| HEAD run total | **937 passed / 0 failed** | `cargo test -p website-frontend`, private dir |
| HEAD `--list` total | **937** | same binary, cross-checked as ordered |
| base (`cf9e5272`) `--list` | **922** | isolated worktree + its own private dir |

922 + 10 (T-754) + 0 (T-759, rewrote in place) + 5 (T-737) = **937**. The wave record's "Suite: 932"
is an arithmetic/transcription error (it is 922+10 with T-737's 5 dropped), not a missing test.
`--list` and the run agree exactly; nothing is hidden. → finding F4.

`cargo test -p map-engine-core --all-features` on MAIN: `everon_peaks_max_above_350 ... ok` (the
worktree-only LFS failure did not appear on main, as required).

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL — all four get fixed this wave)

### F1 — MAJOR | `apps/website/frontend/src/mission_editor.rs:1801` (`route_target`, no Entity arm) — **claim 5 CONFIRMED: validation-panel rows for placed OBJECTS are styled clickable over a click that selects nothing**
**Evidence.** (a) The engine emits placed-object findings with a subject id: `placed_asset_refs`
(`crates/map-engine-core/src/mission/validate.rs:1330-1347`) pushes
`("/entities/{i}/{field}", entity_id, asset)` for every top-level `entities[]` row (the
`entitiesById` copy), consumed by `rule_asset_resolves` at `validate.rs:1388-1407`.
(b) `PanelFinding::is_selectable` (`validation_panel.rs:106-108`) is `subject_id` non-empty, and
`finding_row_view` (`validation_panel.rs:868-880`) styles `cursor-pointer hover:` from exactly that.
(c) `route_target` (`mission_editor.rs:1801-1830`) tries slot SoA → `vehiclesById` → `zonesById` and
**has no `entitiesById` arm**. Repro run in the perturb worktree:
`route_target(json!({"entitiesById":{"e1":{"position":{"x":1.0,"y":2.0},...}}}), "e1", …) == None`
— test `verify129_entity_subject_does_not_resolve` PASSED. (d) The click discards the `false`
(`select_finding_subject`, `validation_panel.rs:950-952`) — not even a toast.
**Impact.** With a live catalogue, an `ASSET-RESOLVES` finding on a placed prop/composition renders
a pointer-styled row whose click does nothing — the exact wave-115 MAJOR class T-754 was shipped to
kill, alive on the validation panel. Reachable today (unlike zones, the engine DOES emit these).
**Disposition — fix this wave.** Add an `Entity` arm to `route_target` at
`mission_editor.rs:1826` (between the `vehiclesById` and `zonesById` lookups, or after zones —
order is free, the id spaces are disjoint): resolve `root["entitiesById"][id]["position"]{x,y}`
(the map is in `small_maps_json`, `doc/store.rs:541-549`) to `RouteTarget::Vehicle`-shaped
coordinates (either reuse `Vehicle` or mint `Entity { x, y }`), and in the registered closure
(`mission_editor.rs:~2624-2652`) handle it exactly like the Vehicle arm (selection + `set_view`) —
vehicles already ride that path. Extend `t754_router_resolves_zones` with the entity fixture (my
repro test is the ready-made pin, assert flipped to `Some(...)`).

### F2 — MAJOR (narrow but reachable) | `apps/website/frontend/src/eden_dock_right.rs:1124-1128` — **claim 2: the zone-selection hook is never unregistered, so an unmounted Zones panel reports a selection that did not happen**
**Evidence.** The hook is registered in `DockRight`'s body (`:1124`) and there is **no `on_cleanup`
unregister anywhere** (`register_select_zone` `:1017`, `route_select_zone` `:1026`). DockRight is
mounted behind the `chrome_hidden` gate (`mission_editor.rs:4495-4515`, `.then(...)`), and
Backspace flips that gate with **no modal-stack guard** — only `in_editable_field()` and `!modk`
(`mission_editor.rs:2791-2794`). `MissionSettingsDialog` deliberately survives the hide
(`mission_editor.rs:4560-4565`, "must survive a hide-interface toggle"). So: open Mission Settings
→ All Settings, press Backspace, click a zone row. `route_target` resolves; `route_select_zone`
finds the **stale** hook and returns **true**; the hook sets `zone_selected` and `tab` — both
DockRight-local (`:1119`, `:1083`) and therefore **disposed** → silent no-ops
(reactive_graph-0.2.14 `traits.rs::Set::set` warns only when `failed && !is_disposed()`; disposed
writes are silently dropped) — while `collapsed` is the **page-owned** `dock_right_collapsed` prop
and DOES flip, mutating hidden-dock state. The router then centres the camera and reports `true`.
No selection exists; on chrome restore DockRight remounts with `zone_selected = None`.
**Impact.** Breaks the ticket's own iff ("clickable IFF clicking selects something") and
`route_select_zone`'s documented contract ("returns whether the panel was there to select it") in a
state reachable with two keypresses. Bounded: camera still centres, and the window closes on
chrome restore (remount re-registers).
**Disposition — fix this wave.** Immediately after the register at `eden_dock_right.rs:1128`, add an
`on_cleanup` that clears `SELECT_ZONE` **only if it still holds this registration** (clone the
`Rc` into the cleanup, compare `Rc::ptr_eq` before `take()` — the guard makes the fix safe against
either dispose/re-register ordering on remount). An unmounted panel then answers `false`, the
router returns `false`, and the settings row click falls back to the `OWNER_UNRESOLVED_NOTE`
toast — the documented residue. Pin it: register a hook, drop it via a scoped owner (or call the
cleanup fn directly), assert `route_select_zone` is `false` again.

### F3 — MINOR | `apps/website/frontend/src/arsenal.rs:242-248` (`attachment_errors` messages) — **claim 14 CONFIRMED: two stranded attachments on ONE row still render identically**
**Evidence.** `attachment_errors` pushes `RowError { key, message }` where `key` is the **weapon
slot** and the message names only the slot label — the attachment `rn` is never printed. Repro test
in the perturb worktree (two attachments on `primary`, neither accepted):
```
["Primary — Attachment not compatible with the selected Primary",
 "Primary — Attachment not compatible with the selected Primary"]
```
`verify129_two_stranded_attachments_on_one_row_are_distinguishable` FAILED exactly as the ticket's
`found_not_fixed` predicted.
**Impact.** The ticket's defect (indistinguishable refusals) survives whenever one weapon strands
two attachments — key AND message identical, so `refusal_line` cannot help.
**Disposition — fix this wave.** Name the attachment in the message at `arsenal.rs:243-246`, both
arms: `format!("Attachment {rn:?} requires a {label} pick")` and
`format!("Attachment {rn:?} not compatible with the selected {label}")`. Key untouched (the row
prefix stays correct). Add the two-attachment fixture to `mod t737` (my repro test, assertion kept
as `assert_ne!` — it goes green with the fix and is the pin).

### F4 — NIT | wave close record — **"Suite: 932" is wrong; the true number is 937**
**Evidence.** Measured above: base 922, HEAD 937 (run AND `--list`, private dirs). 922+10+0+5=937.
**Disposition — fix this wave.** Record 937 (and base 922) in the wave-129 close entry of
`.ai/artifacts/editor_factory_run.md`. No code change.

---

## ADJUDICATIONS the brief demanded

- **Claim 5 (validation-panel dead clicks): CONFIRMED** — F1, with repro.
- **Claim 6 (T-655 benefit overstated): CONFIRMED** — `grep -ic "zone"` over
  `crates/map-engine-core/src/mission/validate.rs` = **0**. No rule emits a zone subject today; the
  zone widening is forward-serving. (F1 shows the surface that is NOT forward-only: entities.)
- **Claim 8 (negative needle on raw `SRC`): SOUND, doubt refuted.** A negative over the widest
  haystack can only over-report: a `set_view(` appearing in the test module or a comment would FAIL
  the test (over-strict), never hide. The needle itself is split. Scrubbing could only remove hits.
- **Claim 12 (Apply-path extension): justified, not creep.** Same `RowError` contract, display-only;
  the "Buffered loadout from s1" framing survives intact under the prefix
  (`apply_refusals_name_the_row_as_well_as_the_source`, green; red under perturbation P5).

## VERIFIED-CLEAN REGISTER — re-proved, not trusted

1. **Suite integrity:** 937 run vs 937 `--list` (cross-checked), 0 failures; base 922 measured in an
   isolated worktree; all in private CARGO_TARGET_DIRs, since deleted. The shared-dir hazard never
   touched this verification.
2. **T-754 correspondence (claim 1):** perturbed `owner_is_routable` back to
   `subject_id().is_some()` → BOTH pins red (`a_row_is_clickable_iff_the_router_resolves_its_subject`,
   `the_affordance_and_the_click_ask_the_same_question`) — the exact wave-115 MAJOR reproduces and
   is caught. Disagreement hunt across settings rows found none *on that surface* (Mission rows
   inert ✓, zone/slot id spaces disjoint ✓, build-root vs click-doc race closed by the `doc_tick`
   rebuild — see 4); the two cross-surface disagreements found are F1 and F2.
3. **T-754 zone shapes (claim 3):** shapeless (`z-shapeless`) → `None` ✓ and deleted (`z-deleted`)
   → `None` ✓ (unit tests, re-read of `zone_centre` `mission_editor.rs:1837-1860`). Third-shape
   hunt: malformed circle coords (falls through to absent polygon → `None`), non-array/short
   polygon vertices (filtered → empty → `None`), empty ring (`None`), garbage root (`None`) — all
   inert, none panic. Nothing unresolvable resolves.
4. **Deleted-under-open-dialog really re-renders inert:** `AllSettingsDialog`'s whole body is inside
   `move || { let _ = doc_tick.get(); … }` (`eden_settings.rs:1927-1954`), so a zone delete bumps
   `doc_tick` and rebuilds the rows against the new root. The residue race is within one tick and
   is toast-covered.
5. **Claim 4 (ZONES_TAB):** `ZONES_TAB = 3` = the previous literal on both sites (button + panel
   arm, diff-confirmed 1:1 swap); production-half counts: `tab_btn(ZONES_TAB,` ×1,
   `ZONES_TAB => zones_panel(` ×1, `tab_btn(3,` ×0. No off-by-one; the routed click raises the same
   index the panel renders under.
6. **T-759 red-checks (claim 7):** three production usages individually deleted in the worktree —
   `camera_snapshot` (wasm-only), `__editorCamSet` (wasm-only), `parse_locations_json` — each turned
   its pin RED with the pin's own message, while this file still **names all three in comments**
   (lines 182/847/859/861/1206/1225), proving prose no longer satisfies them. (My first attempt was
   a false GREEN caused by my own rename leaving the needle as a substring — corrected to real
   deletion; the pins were never at fault.)
7. **T-759 count + scrub choice (claim 9):** 13 positive needles confirmed — 9 in
   `the_dock_has_two_tabs_and_defaults_to_layers` (1 code + 8 testids), 4 in
   `the_index_and_the_fly_to_reuse_the_shipped_paths` — both formerly-hollow tests fixed, the
   `t637_density`/`t697` modules were already sound. `live_code` vs `live_source` correct per
   needle: code facts (`RwSignal::new(LeftTab::Layers)`, `parse_locations_json`, `camera_snapshot`)
   on `live_code`; shipped literals (8 testids, the locations URL, `__editorCamSet`) on
   `live_source`.
8. **Prose-trap sweep over the wave's 15 new pins (claim 10): none prose-satisfiable.**
   eden_settings/mission_editor/arsenal pins read `class_r_scrub` output (comment-stripped — proven
   incidentally: the `refusal_line` count==2 pin passes despite T-737 adding two comments naming
   `refusal_line` inside `ArsenalTab`). eden_dock_right's `production()` DOES keep comments, but
   every needle is code-shaped, occurs exactly once and only in code, and perturbation P7 (deleting
   the register block, prose left intact) turned `the_hook_selects_the_zone_and_shows_it` RED.
9. **T-737 perturbation (its own RED claim):** `refusal_line → e.message` unconditionally → 3 tests
   red (`two_stranded_rows…`, `apply_refusals…`, `the_export_import_asymmetry_still_holds`).
   Claim 11 verified by the green suite (distinguishable rows; schema faults keep `IMPORT_DOC_KEY`
   and their JSON-pointer messages byte-unchanged).
10. **Claim 13:** `try_export` byte-unchanged — the wave's arsenal hunks are `@636` (additions after
    `IMPORT_DOC_KEY`), `@1387`, `@1476` (the two `.chain` render lines), `@5849` (tests); nothing
    inside `try_export`. The asymmetry pin exists, passes, and reads the refused bytes.
11. **Wave-127 z-rule:** zero occurrences of `update_slot_position` / `move_entities_and_vehicles`
    in the whole wave diff; `zone_centre` is read-only; the router's only engine writes are
    `set_view`/`on_camera_changed` (camera, not entity positions). No violation.
12. **Python:** wave diff touches exactly the five owned files; `git ls-files '*.py'` empty; on-disk
    `find` (excluding node_modules/target/.git) empty. Nothing python-shaped landed.
13. **everon_peaks on MAIN:** PASS (worktree-only LFS failure confirmed absent from main).
14. **Falsification attempts that found nothing:** slot/zone id-collision precedence (slot arm still
    wins, pinned); `route_target` panics on garbage roots (total, pinned); a second `cursor-pointer`
    spelling in `setting_row_view` (literal-kept scan, none); a second registered router (count==1
    pin over `live_code`, anchored past the early `#[cfg(test)]` in `registry_session` so the
    haystack is non-empty); thread_local test-order hazard on `SELECT_ZONE` (libtest runs each test
    on a fresh thread; the honest-report test's pre-assert is safe).

**Not re-run:** the 30/30 browser gate (needs full Chrome; taken from the wave record — every claim
it covers that I could reach natively was re-proven above).

## Is `main` safe to build the next wave on?

**Yes** — with the four findings queued for THIS wave's no-deferral fix pass. Nothing is broken at
base behavior: 937/937 green, no data-loss path, no gate that reports on code it never examined
(the one false-GREEN I hit was my own perturbation's substring bug, and the pins proved sound once
the deletion was real). F1/F2 are affordance lies in the exact class this wave was hunting — real,
repro'd, and bounded — and F3/F4 are a message field and a log number.

— wave-129 verifier, private dirs `tbd-target-verify129{,-base,-perturb}` deleted, worktrees removed,
main untouched.

## Focused re-verification of the fix pass

Scope: the six fix commits only (6508e5d1 F1, f6a2b687 F2, 961dea6c F3, dac84b03 F5, a875182d F6,
6445a6ed F7) on merged main. Harness: private `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-reverify129`
(deleted after use), cargo routed through `distrobox-host-exec`.

**Suite cross-check: `--list` total 954 = run total 954 = expected 954. 954 passed, 0 failed.**
`cargo test -p map-engine-core --all-features`: 625 passed, 1 ignored (`flatten.rs:4775`,
`#[ignore = "manual golden regeneration"]` — intentional, pre-existing), plus 5+5+3 in the
secondary binaries, all green. **`everon_peaks_max_above_350` ran and PASSED** (named run shown,
not inferred from the summary line).

### FINDINGS

**RV-1 — MAJOR | apps/website/frontend/src/eden_dock_left.rs:546, :1412-1414, :1421-1427, :2438-2466
— THE FOURTH AFFORDANCE/CLICK DIVERGENCE: dock-left search-hit rows decide clickability from a
hardcoded kind list that predates the router's Zone and Entity arms.**
What is wrong: `DocKind::is_selectable()` (line 1413) returns `matches!(self, DocKind::Slot |
DocKind::Vehicle)` and is the affordance gate for the dock-left search results (line 546). Its doc
comment (1406-1410) claims "`route_select_by_subject_id` resolves a `subject_id` … in the slot SoA
and then in `vehiclesById.position`. There is no third lookup." That was true before this wave's
slices; it is false on merged main: T-754 (260b789e) gave the router a Zone arm and F1 (6508e5d1)
gave it an Entity arm. So a search hit of kind `Zone` or `Object` renders INERT
(`dock-left-search-hit-inert`, `aria-disabled`) while clicking it WOULD succeed — DockLeft is only
mounted with chrome visible, i.e. exactly when the Zones panel is also mounted, so the zone click
would land every time the row is visible. This is the inverse direction of F6's divergence (inert
row over a live click) and is the wave's central invariant broken on a FOURTH surface: the same
click goes through `crate::validation_panel::route_select_by_subject_id` (line 560), the same
resolver whose answer the validation panel (F1), the probe (F6) and aggregated settings (F7) now
all share — this surface never asked it.
Compounding it: the operator-facing refusal text (`unselectable_reason`, 1421-1427) now states a
falsehood — "the editor's click-to-select router resolves slots and vehicles only" — and the pin
`only_the_kinds_the_router_resolves_are_live_affordances` (2438) is green because it hardcodes the
SAME stale list ("T-754: the router has no arm for a {} …") instead of pinning the correspondence
against the router. The pin asserts a fact that stopped being true two commits into this wave and
cannot go red for it.
How proved: read `is_selectable`/its call site/`unselectable_reason`; confirmed `route_target` has
Zone and Entity arms on main (F1 diff + `t754_router_resolves_zones::every_arm_resolves…` green);
confirmed dock-left's click goes through `route_select_by_subject_id` (line 560); confirmed the pin
text pins the kind list, not the correspondence.
Fix shape: at line 546 decide the affordance per-id via `crate::validation_panel::subject_id_routes(&hit.id)`
(the registered availability probe — the identical move F7 made for eden_settings), keeping `DocKind`
for icon/badge only; make `unselectable_reason` state the router's real refusal (or derive it from the
probe's `false`); rewrite the 2438 pin as a two-directional, non-vacuity-guarded correspondence pin in
the shape of `eden_settings::t754_click_affordance::a_row_is_clickable_iff_the_router_resolves_its_subject`.
Note: Marker/Trigger/Comment/Layer hits remain genuinely unroutable (no router arm) and must stay inert.
This predates the fix pass (T-697 era) but was made false BY this wave's own widenings; under the
wave's thesis ("a fourth divergence is a MAJOR") it must not close with the wave green.

**RV-2 — NIT (pre-existing, out of fix-pass scope) | apps/website/frontend/src/eden_layout.rs:583 —
unused import `chrome_hidden`** (plus `NodeKind` eden_tree.rs:1056, `EventHub` event_manager.rs:30,
dead `auth_error_copy`). Warnings in the native test build; eden_layout.rs:583 blames to T-638
(a4e4dcc8), and none of the six fix commits touch these files — NOT the stalled agent's residue.
Listed only so wave 130's model does not rediscover them as new.

No other findings. Each numbered attack, with method and outcome:

1. **Fourth divergence hunt** — found (RV-1). Beyond it: swept every `cursor-pointer` site in the
   frontend (eden_tree rows: callbacks passed from the owning mount, no seam; context_menu: `enabled`
   computed beside the action; vehicles panel / attributes: local-state toggles; orbat/events/etc.:
   non-editor pages) — no other affordance answers a different question than its click.
   The prompt's specific candidates, each falsified: *Zones panel mounted but collapsed* —
   `install_select_zone` runs at DockRight body top level (eden_dock_right.rs:1181) on body-owned
   signals (`zone_selected`:1172, `tab`:1136), `collapsed` only swaps the wrapper class
   (mission_editor.rs:4635-4638), and the hook itself un-collapses (`collapsed.set(false)`) — click
   works, probe true, consistent. *Target panel mid-unmount / chrome toggling between probe-read and
   click* — reactive updates are synchronous; the click re-asks `available` (reads `chrome_hidden`
   at click time) and `route_select_zone`'s own report is kept as ground truth behind it (double
   gate, both honest). *Row rendered before the probe registers* — rows exist only after
   `register_payload_source`, which runs in the same synchronous `on_load` block BEFORE
   `register_route_probe`/`register_select_by_id` (mission_editor.rs:2536/2660/2664), and the probe
   registers before the actor. *Stale id after undo/redo/restore* — `resolve` re-reads the live doc
   per call; the F1 comment explicitly refuses `doc_ver` memoisation because IDB restore does not
   bump it. *Slots on a hidden layer* — still in the SoA; a click selects and centres (does
   something), so the affordance is not a lie.
2. **F6's mirror** — CAN'T currently be wrong: the only `DockRight` mount is
   mission_editor.rs:4640 behind exactly `(!chrome_hidden.get()).then(` (4634); `eden_chrome::DockRight`
   is a bare re-export (eden_chrome.rs:32); `install_select_zone` is unconditional in the DockRight
   body; no error boundary or second gate between them; the hook unregisters at that same owner's
   cleanup (F2), so hook-live ⟺ DockRight-mounted ⟺ !chrome_hidden. The
   `the_zone_liveness_oracle_is_the_dock_right_mount_gate` pin holds mirror to subject (<400 squashed
   chars). Residual risk accepted and documented: the pin's `rfind` could in principle be satisfied by
   a DIFFERENT `(!chrome_hidden.get()).then(` within 400 chars if the mount's own gate changed — but
   the adjacent gates are >400 chars apart today and the F6 row of the behavioural table backstops it.
3. **F5's idiom, verified in the crate sources, not the claim**: reactive_graph-0.2.14
   `RwSignal::eq` → `ArenaItem::eq` → `NodeId` (arena_item.rs:26-30), `NodeId` is a slotmap
   `new_key_type!` (owner/arena.rs:15), slotmap-1.1.1 `KeyData { idx, version: NonZeroU32 }` with
   documented ABA protection — a recycled slot is NOT mistaken for its predecessor. Also verified
   `Owner::cleanup` runs cleanups BEFORE removing arena nodes (owner.rs:555-579), so
   `try_with_value` inside the cleanup is valid, as the code comments claim. Perturbations run by me
   on a scratch copy of main (restored, tree clean): (a) identity guard removed from
   `unregister_seam` → `an_older_owners_cleanup_does_not_clobber_a_newer_registration` red ALONE
   (2 passed, 1 failed); (b) `on_cleanup` removed from `install_seam` →
   `unmount_unregisters_every_seam…` red (shape 2, plus shape 3). Exactly as the commit claims.
4. **F1+F5 interaction** — the probe registration lives inside `canvas_ref.on_load`, which tachys
   implements as `Effect::new` (tachys-0.2.18 node_ref.rs:39-56); the closure therefore runs under
   the effect's owner (a child of the page owner), so `on_cleanup` HAS an owner in production —
   unregistration is real, not test-only. The effect cannot re-run after registration (it tracks
   only the NodeRef signal; the canvas at 4564 is ungated and never remounts within a page life), so
   the seams are not torn down mid-session. A page REMOUNT runs the body → new `on_load` effect →
   re-registers. Not permanently inert.
5. **F7's negative pin is not hollow** — `owner_is_routable` (1891) and its call site (2043) sit
   BEFORE the first `#[cfg(test)]` (2157) where `live_code` truncates; all 9 `route_target(`
   occurrences in eden_settings.rs are at ≥3033 (test modules); the pin also positively requires
   `subject_id_routes` in `owner_is_routable`'s body, so a truncation bug would fail it, not
   vacuously pass. Crate-wide `route_target(` callers: mission_editor.rs (1 live — pinned),
   validation_panel.rs (docs only), eden_settings.rs (tests only) — no fourth RESOLVER copy; the
   fourth AFFORDANCE copy is RV-1's kind list, which never calls the router at all.
6. **F3** — two stranded attachments on one row now render distinguishably (`{rn}` in both message
   arms), the new test covers hosted + hostless arms and asserts the T-737 row prefix survives;
   `two_stranded_rows_render_as_two_distinguishable_refusals` (T-737's case) still present and
   green. No regression.
7. **Pins run, not read** — all named pins executed individually and green (36 tests via filtered
   run): T-754's settings pins (7 incl. F7's two), `t754_router_resolves_zones` (5 incl. F1's count
   pins), T-655's two wiring pins, `t662_input_traps::chrome_hidden_signal_gates_the_five_mounts`,
   F2's and F5's lifecycle triads, F6's three, T-737's five, `w129_the_panel_asks_the_router`'s three.
8. **Wave-127 z-rule** — `git diff f1c46457..HEAD` contains zero added `update_slot_position` /
   `move_entities_and_vehicles` calls. Clean.
9. **Stalled-agent residue / hygiene** — the six commits touch exactly the five expected .rs files;
   no `.py`, no `tmp_*`/scratch survivors; eden_settings.rs compiles with zero warnings (the four
   suite warnings are RV-2's pre-existing files); working tree on arrival = registry.json (modified)
   + the two orchestrator files, and it is left exactly so.
10. **F6 behavioural perturbation** — Zone narrowing gutted (`if false`) on a scratch copy: the
    table fails on precisely `[zone, Zones panel unmounted]` "probe and click disagree"; source pins
    stay green, proving the table (not only the source pins) carries the invariant. Restored.

### Are the wave-129 fixes complete and correct — **yes** for what the six commits claim and touch:
every fix does what its message says, every perturbation reddens where promised, the F5 idiom's
slotmap/cleanup-ordering claims verify against the vendored crate sources, and the lifecycle
machinery is live in production (owner present under `on_load`). **But the wave's own thesis is not
yet fully enforced**: RV-1 is the predicted fourth divergence, on a surface the fix pass never
visited.

### Is main safe to close this wave and build wave 130 on — **yes to build on, no to close green
without RV-1**: nothing is broken-broken (main compiles, 954/954 + 625 green, no data at risk, no
operator work destroyed — RV-1's failure mode is a working click denied and a false explanation,
not a lie that eats work). Under the NO-DEFERRAL regime RV-1 must be fixed before close; the fix is
one call-site swap + one string + one pin rewrite, fully specified above, with F7's commit as the
exact template.

### VERIFIED-CLEAN REGISTER (falsification attempts in every category where nothing was found)
- **Fourth divergence**: full-crate `cursor-pointer` sweep (40+ sites triaged), all seam consumers
  enumerated (`subject_id_routes` / `route_select_by_subject_id` / `route_select_zone` callers), all
  six prompt-listed candidate states individually attacked (collapsed panel, mid-unmount, chrome
  race, pre-probe render, hidden layer, undo/restore stale id) — one found (RV-1), rest clean.
- **F6 mirror**: hunted a second DockRight mount, a wrapper conditional in `eden_chrome`, an error
  boundary, a collapse-unmount, a non-chrome gate — none exist; the re-export and single gated mount
  verified by read, the gate-adjacency by the squash pin's own logic.
- **F5**: attempted to break the slotmap-version claim and the cleanup-before-node-removal claim
  against vendored reactive_graph 0.2.14 / slotmap 1.1.1 source — both hold; attempted the two
  perturbations — both redden exactly as documented, the guard case ALONE.
- **F1/F5 production liveness**: attempted to show `on_cleanup` has no owner under `on_load` (which
  would make F5 test-only theatre) — refuted by tachys source; attempted to show the effect re-runs
  and tears seams down mid-session — refuted (single-tracked NodeRef, ungated canvas).
- **F7 hollowness**: attempted to place `owner_is_routable` or a live `route_target` call outside
  `live_code`'s reach — refuted by line arithmetic and an independent grep.
- **Suite honesty**: `--list` vs run cross-checked (954=954), named tests shown running by name,
  everon peak test run in isolation, the one ignored test identified and adjudicated.

— focused re-verifier, wave 129 fix pass. Private dir `tbd-target-reverify129` DELETED. Main left
exactly as found (perturbations applied to working copies were `git checkout`-restored and
re-verified clean). Wave 130's model: RV-1 is the only open item; everything else above is proved,
not presumed.
