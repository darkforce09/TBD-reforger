# Wave 144 — adversarial verification (MERGED MAIN, ab805357)

Verifier: Claude (Fable 5), 2026-08-08. Base de30a70e (marker 120). Merges b3dd5717 (T-781, widened)
and 7705f840 (T-783). All cargo routed through `distrobox-host-exec`, private
`CARGO_TARGET_DIR=/home/Samuel/.cache/wave144-verify-target` (NOT /tmp) — **deleted after the run**.
Working tree carried ONE pre-existing uncommitted change when I arrived: `.ai/tickets/registry.json`
(the operator's T-784 draft, next_id 784→785). Left untouched. Every perturbation below was applied,
run, and reverted with `git checkout --`; final `git status` shows only that registry draft.

## Harness discipline receipts

| Suite | `--list` total | Run total | Result |
|---|---|---|---|
| map-engine-core `--lib --all-features --no-fail-fast` | **639** | **639** (638 pass, 1 ignored, 0 fail) | PASS |
| website-frontend (native) | **1052** | **1052** (all pass) | PASS |

- Core 639 = expected 639. The 1 ignored is `mission::flatten::tests::regen_compiler_shaped_fixture`
  ("manual golden regeneration") — standing, not a wave artifact.
- **`dem::peaks::tests::everon_peaks_max_above_350` RAN and PASSED on main** (asset is real here,
  not an LFS pointer). Confirmed by name in the run output.
- Frontend 1052 = 1050 base + 2. Derivation: measured 1052 by `--list` AND by run; the wave diff
  contains exactly two new frontend `#[test]` fns (`a_composition_captures_comments_and_authored_elevation`,
  `the_seam_mechanism_is_defined_exactly_once_in_the_crate`) — counted with `grep -c '^+    #\[test\]'`
  over `git diff de30a70e..HEAD -- apps/website/frontend`. Core likewise +2 (the two T-781 behavioural
  pins), 637→639.
- wasm32: `cargo check --target wasm32-unknown-unknown -p website-frontend` **clean** (8 warnings,
  all pre-existing dead-code in editor_ops/mission_commands/ui — none in the wave's files).

## FINDINGS

### F-1 | MAJOR (pre-existing, disclosed — NOT a wave-144 regression) | apps/website/frontend/src/editor_ops.rs:4220, 4244–4252 | mint uniqueness is proven against a universe that omits hidden slots — a place can silently upsert a hidden slot away

**Evidence.** `mint_id` (editor_ops.rs:4220) and `mint_ids` (4244) build `existing` from
`core.materialize().ids`. `materialize()` (crates/map-engine-core/src/doc/store.rs:740–786)
**drops** every slot on a hidden layer (T-665) and every `editorHidden` slot (T-701) before any
column is pushed. `next_id` is `Cell::new(0)` per editor mount (editor_ops.rs:314) — the doc comment
on `mint_id` itself names the trap: "an IDB restore can bring back a document that already used
`n0`". So: restore a mission holding slot `n0` on a hidden layer → `existing` does not contain `n0`
→ the next place mints `n0` → `place_composition` / the place path inserts a row under that id.
Insert is an **upsert**: my scratchpad probe (attack 4, below) proved a second `place_composition`
under an existing comment id silently replaces the row ("FIRST" → "SECOND"); slots use the same
`self.slots.insert` idiom (store.rs:2453). The operator's tucked-away hidden slot is overwritten —
role, position, loadout — inside the new placement's undo step.

**Impact.** Silent destruction of operator work, reachable by: hide a layer, reload, place. It is
the exact T-665 lesson (SoA omits hidden slots) that T-781's own capture code cites and avoids —
but the mint's uniqueness set was never given the same treatment. Pre-existing at base (mint_id and
the vehicles/entities union both predate wave 144); T-781 only **narrowed** the class by adding
`commentsById` (correct, and load-bearing per attack 4). The slot half remains open.

**Disposition / fix shape.** In both `mint_id` and `mint_ids` (one shared helper is the right
shape), build the slot half of `existing` from `core.slots_json()` keys — the exact-f64 rows that
include hidden slots, the same source `capture_selection_entities` was pointed at for the same
reason — instead of `materialize().ids`. ~6 lines, no contract change. Not fixed here (verifier
does not fix); flagged for the wave-close fix pass since the NO-DEFERRAL regime applies.

### F-2 | MINOR | apps/website/frontend/src/eden_dock_right.rs:1723–1728, 1738 | the composable-comments gap is TOTAL in the UI, and the new panel copy advertises it — the found_not_fixed (and the T-784 draft) understate it

**Evidence.** The slice's found_not_fixed says the only lane into selecting a comment is the T-697
selection-filter apply. Verified, and it is **worse**: (1) outliner comment row is `ROW_STATIC`
with no click→selection (eden_tree.rs:792); (2) the map glyph has no pick path (`route_target`,
mission_editor.rs:2271, has slot/vehicle/entity/zone arms and **no comment arm**, so a dock-left
search hit for a comment renders **inert** via `hit_is_routable`); (3) the T-697 lane itself cannot
introduce a comment: `set_selection_ids`' only caller is `eden_dock_left::apply_selection`
(eden_dock_left.rs:1622–1626), fed exclusively by facet chips that **narrow the current selection**
(`selection_facets` over `selection_entities`). I checked every `ctx.selection.borrow_mut()` write
site (editor_ops.rs 496/753/1196/1280/2839/4105/5638/5669; mission_editor.rs 3214/4434/4805/4976):
all draw from the SoA, slot-filtered `written` lists, or the comment-armless router. **No lane in
the shipped UI ever puts a comment id into the selection.** The wave's new panel copy ("Selected
comments are captured too…", "Select one or more placed entities **or comments**…") therefore
describes an affordance an operator cannot exercise.

**Impact / disposition.** Per the operator's calibration this is an absent-but-would-work
affordance → FILED, not fixed in-wave, and the operator has already drafted T-784 (uncommitted
registry edit) for exactly this. Two corrections for that draft: its consequence line ("the ONLY
lane … is the T-697 selection-filter apply") credits a lane that only narrows — from the UI the
feature is reachable **zero** ways, not one; and the two new eden_dock_right sentences should be
re-checked once T-784 lands so the copy and the click agree. The T-781 code itself is sound and
fully pinned; this is scope accounting, not broken code.

## PERTURBATION RECEIPTS (all reverted)

1. **Remount guard (item 11).** Deleted the `is_same_registration` check from the single
   `validation_panel::unregister_seam` (unconditional `slot.take()`). Full frontend suite:
   **exactly 2 failures / 1050 pass** — `ruler_tool::t778_seam_lifecycle::an_older_owners_cleanup_does_not_clobber_a_newer_registration`
   and `validation_panel::f5_seam_lifecycle::an_older_owners_cleanup_does_not_clobber_a_newer_registration`.
   Reddens in BOTH files, ALONE, from ONE edit — the tests measure the guard and nothing else, and
   the two tables genuinely share one body now.
2. **Seam copy in a NEW file (item 8).** Planted `src/zz_seam_copy_probe.rs` containing
   `pub(crate) fn install_seam_later()` — a file `main.rs` never declares, named with the wave-142
   decoy superstring shape. `the_seam_mechanism_is_defined_exactly_once_in_the_crate` went RED:
   `Found: ["validation_panel.rs x1", "zz_seam_copy_probe.rs x1"]`. The walk is a real recursive
   `read_dir` from `CARGO_MANIFEST_DIR/src` (I count 82 `.rs` files there, so the >40 sanity bound
   is meaningful, not trivially green on a wrong root).
3. **Copy parked below a test module (item 8, raw half).** Appended `#[cfg(any())] fn
   unregister_seam_parked() {}` at ruler_tool.rs EOF — below its test module, invisible to
   `live_code`, never compiled. The RAW count went RED: `Found: ["ruler_tool.rs x1",
   "validation_panel.rs x1"]`. The scrubber-blind-spot argument holds.
4. **Core-side key rename (item 3).** `g_num(fields, "elevation")` → `"elev"` at store.rs:2449.
   Caught TWICE: core behavioural pin `placing_a_composition_keeps_each_entrys_authored_elevation`
   FAILED (637/639), and the frontend cross-crate pin FAILED with "place_composition must read back
   the SAME elevation key the capture writes". The agreement constrains BOTH directions (the pin
   checks capture and place against one literal), not one-way.

## SMUGGLE ATTACKS — a composed comment vs the compiled mission (all FAILED to break)

Scratchpad probe crate (outside the repo) linking `map-engine-core` with `mission`+`doc` features,
token `CMT-SMUGGLE-XQZ9`:

- **Place → compile:** token absent from `flatten_mod_document_json` output.
- **Hydrate-then-compile, two generations deep:** `compile_payload` → `hydrate` into a pristine doc
  → compile again (twice). Comment survives every hydrate on the editor payload (asserted); token
  absent from the mod bytes in every generation.
- **Layer route:** `move_comment_to_layer` then compile — token absent AND the comment **id**
  absent (no dangling entity id; flatten's squad rows `filter_map` slot ids against `editor.slots`,
  store.rs layer membership never reaches `EditorPayload`, which declares no layers key at all).
- **JSON-injection-shaped title:** a title crafted as `","entities":[{…alias:TOKEN…}],"x":"` placed
  via the comment arm — did NOT reach the mod document (everything is serde-serialized, nothing
  concatenates payload JSON) and round-trips **verbatim** on the editor payload.
- **Structural mechanism confirmed at source:** `mission::flatten::EditorPayload`
  (flatten.rs:907–938) declares `zones/entities/vehicles/settings/editor/environment` only — no
  `comments`, no `editorLayers`, no `#[serde(flatten)]` catch-all anywhere in flatten.rs; the
  `payloadExtras.comments` projection is promoted to the payload ROOT `comments` key by
  `compile_payload` and then dropped by that same absence. The shipped test's leak probe
  (entities[] re-route) ran green, so the absence assertion is non-vacuous.
- **Core-level id collision (item 5):** two `place_composition` calls sharing an id — the second
  UPSERTS the first note away at core level, proving `mint_ids`' widened `commentsById` union is
  the load-bearing guard (and feeding F-1's slot-half evidence).

## THE REST OF THE CHECKLIST

- **T-781 elevation vs dz (item 2):** clean at every site. Capture's comment arm reads
  `position.z` (northing) into `y` beside every other kind's `y`, sets `elevation: 0.0`, and OMITS
  the key from the entry; `place_composition`'s comment arm consumes neither `rot` nor `elev`;
  `slot_z` (height) is only applied to slot/vehicle/entity rows. Round trip pinned by the shipped
  core test (offsets `drop±dx/dz`, position keys exactly `["x","z"]`) — ran green here.
- **Backward compat (item 4):** `g_num` map_or(0.0) ⇒ absent `elevation` places at ground —
  pinned by the no-key control entry, ran green. The only other arm changes are the `elev`
  argument; horizontal clamp, faction seeding, layer filing untouched. Old compositions carry no
  `"comment"` kind, and unknown kinds still skip.
- **OWNS deviation (item 6): justified and confined.** Declared owns (wave_plan.tsv:572) =
  store.rs + eden_dock_right.rs. `editor_ops.rs`: the capture is the only producer of composition
  entries, so a place-side arm alone is unreachable — the HARD GATE case; the 89-line diff is
  capture + `mint_ids` + doc prose, nothing rode along (read in full). `arsenal.rs`: 2 lines,
  `editor_ops.rs:2072→2142` and `:2088→2158`, forced mechanically by
  `t739::arsenal_cites_live_set_loadout_lines`, which **derives** the live line numbers from
  `editor_ops.rs` source at test time (position of `pub fn set_loadout` / its `after_local_edit`
  line) — I verified line 2142 IS `pub fn set_loadout` and 2158 IS the tail call, and the pin still
  constrains (any future drift reddens it). Correct numbers, still-live pin.
- **T-783 pure refactor (item 9): byte-identical.** Extracted the deleted ruler_tool bodies at
  base and diffed against validation_panel's at base (modulo the `pub(crate)` keyword): identical.
  Diffed validation_panel's base bodies against merged main: unchanged except visibility. Doc
  comments moved/reworded only.
- **Re-export (item 10):** `pub(crate) use crate::validation_panel::install_seam` is the only
  `install_seam` item in ruler_tool — no local definition survives, so nothing shadows. `los_tool`
  imports `crate::ruler_tool::install_seam`, `world_assets` calls it by path; nobody outside
  validation_panel names `SeamCell` or `unregister_seam`. Native suite green + wasm32 check green
  covers both targets.
- **Wave-127 z-rule:** zero occurrences of `update_slot_position` / `move_entities_and_vehicles`
  in the wave diff — no new caller, no `z = None` on an x/y write.
- **T-782 reserve (`max_players` compile behaviour):** no mission/compile/flatten/eden_settings
  file in the wave diff; the only `maxPlayers` occurrences added are read-only meta fixture strings
  inside the two new core tests. Compile path untouched.
- **Hygiene:** `git diff --diff-filter=A de30a70e..HEAD` is EMPTY (no files added at all — so no
  `.py`, no scratch/`tmp_*` test files); no `tmp_`/`scratch` symbols in the diff.

## Is `main` safe?

**Yes.** Both suites green at full expected counts with non-vacuous cross-checks, the everon peaks
test ran and passed, both tickets do what they claim, and every perturbation reddened exactly the
tests that claim to pin it. F-1 is real and can destroy operator work, but it is latent, requires a
restore + hidden slot + place, and was equally present at base — nothing this wave shipped widened
it (T-781 narrowed it for comments). F-2 is scope accounting the operator has already ticketed.

## VERIFIED-CLEAN REGISTER — attacked and FAILED to break

1. Comment into the compiled mission via straight place → compile. FAILED (token absent).
2. Comment into the compiled mission via hydrate-then-compile, ×2 generations. FAILED.
3. Comment into the compiled mission via layer membership (`move_comment_to_layer`), token AND id. FAILED.
4. Comment into the compiled mission via squad `slotIds` (static: `filter_map` against `editor.slots`). FAILED.
5. Comment into the compiled mission via `payloadExtras.comments` promotion (lands on the undeclared root key). FAILED.
6. Comment into the compiled mission via a JSON-injection-shaped title. FAILED (serde end-to-end; verbatim round-trip).
7. A serde catch-all (`#[serde(flatten)]` / raw-bytes passthrough) in flatten.rs that would defeat the absence. NONE FOUND.
8. The absence assertion being vacuous — the shipped `entities[]` leak probe, re-run green on main. HOLDS.
9. elevation/northing axis confusion at any of the four read/write sites (capture slots, capture comments, place arms, comment_row). NONE FOUND.
10. Cross-crate key rename on the CORE side sneaking past — caught by two independent tests. HOLDS (both directions).
11. Old (pre-T-781) composition placing differently in any way other than z=authored-0.0. NONE FOUND (control pinned, arm diff read in full).
12. Two placements colliding through `mint_ids` with comments in the doc — the widened union blocks it (core-level upsert proven, which is what makes the union load-bearing; slot-half residue filed as F-1).
13. A second seam mechanism hidden in a NEW file, under a decoy superstring name, or parked below a test module. ALL REDDEN THE PIN.
14. The pin's >40-file sanity being trivially green on a wrong walk root (82 real files; recursive read_dir verified in source). HOLDS.
15. The remount-guard tests measuring anything but the guard (guard removal → exactly the 2 remount cases red, nothing else). HOLDS.
16. T-783 smuggling a semantic change under "pure refactor" (byte-diff of all three body generations). NONE.
17. Re-export shadowing or a wasm32-only resolution break (`cargo check --target wasm32-unknown-unknown` + native suite). NONE.
18. A new z=None x/y write, a `max_players` compile-path touch, an added `.py`, a scratch test. NONE.
19. Vacuous suite pass: `--list` totals cross-checked against run totals on both suites; core run included `--all-features` (T-747) and `--no-fail-fast`. NOT VACUOUS.

Verification-run hygiene: private target dir deleted; perturbation edits and probe artifacts
reverted/removed; working tree back to exactly the found state (registry.json T-784 draft only).
