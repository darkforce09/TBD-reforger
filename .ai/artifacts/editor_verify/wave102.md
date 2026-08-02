# Wave 102 adversarial verification — T-640 / T-664 / T-665 (e9e4539f..1053acd9)

**VERDICT: 0 BLOCKER / 4 MAJOR / 3 MINOR / 10 NOTE — all suites green (frontend 437, core lib 465 all-features / 336 doc,mission, +5+5 integration, 3 doctests; clippy core clean), but two of the three slices ship a provable behavioral defect the slice's own tests cannot see, and T-640's re-march is measured 43–54× slower per fire than the path it replaced.**

Verifier: Fable 5, read-only (this file is the only write). Tests under
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` via distrobox-host-exec:
`cargo test -p website-frontend` → **437/437**;
`cargo test -p map-engine-core --features doc,mission,world` → **465 lib (1 ignored) + 5 + 5 + 3 doctests, 0 failed**;
`--features doc,mission --lib` → **336**; default features → **122**;
spot re-fires: `per_peak_selects_one_highest_closed_ring_each` → ok,
`hidden_and_locked_flag_flips_are_one_undo_step_each` → ok.
Empirics from a scratchpad harness crate (path-dep on `map-engine-core`, release build;
harness source in session scratchpad, not committed).

All three slices were cut from the same base `e9e4539f` (parallel worktrees, barrier merge
1053acd9) — this explains every count/line-number discrepancy in the claims (see F-11/F-16).

---

## F-1 MAJOR — T-640: the summit rule marks EVERY closed contour of a depression as a "summit ring"

**Evidence.** `summit_ring_indices` (`crates/map-engine-core/src/geometry/contours.rs:367-390`)
selects a closed ring iff no *higher*-level closed ring nests **inside** it. For a peak, higher
rings are inside lower ones — the rule picks the top ring. For a **depression** the containment
inverts: higher rings are *outside* lower ones, so **no ring of a closed basin ever contains a
higher ring, and every one of them qualifies**. Harness case A (crater in a 100 m plateau, floor
45 m, levels 50–90): all rings close, and `summit_ring_indices` selected **5 of 5** —
`[50, 60, 70, 80, 90]` — the entire crater renders in `CONTOUR_SUMMIT_RGBA`. Harness case B
(cone summit with a top crater) additionally shows the selected ring need not be the peak's top
ring (picked 60 m on a 100 m cone). The four shipped T-640 tests
(`contours.rs:510-594`, `vector_compose.rs:372`) cover bumps, ramps, and colour split only —
no depression, no crater, no on-edge probe.

**Impact.** On any Everon terrain with a closed basin above sea level (quarry, dune hollow,
inland basin at the marched interval) every contour of the basin draws in the darker emphasis
tint — the exact opposite of "the ONE emphasis … innermost closed contour of each peak"
(`dem_vectors.rs:31-33`). This is a geometric correctness failure of the slice's headline rule,
not an edge case of the implementation of that rule.

**Disposition.** Document, don't fix (wave rule). The fix direction for the follow-up ticket:
qualify a ring as summit only if the terrain *inside* it is higher than the ring's level
(sample the DEM at an interior point, or compare against the enclosing ring's level), or
equivalently require that the candidate is not itself contained in any *lower*-level closed
ring that contains no other candidates. Add depression + crater-on-summit fixtures.

## F-2 MAJOR — T-640: one re-march measured 43–54× the old cost — a 1–4 s main-thread freeze per band crossing at high zoom

**Evidence.** Harness case D, Everon-scale synthetic (1601² grid = the real
`6400/DEM_VECTOR_GRID_FACTOR(=4)` size, max ~390 m, island falloff), native release:

| interval | old `contour_segments` | new `contour_rings` + `summit_ring_indices` | ratio |
|---|---|---|---|
| 10 m (37 levels) | 15.5 ms (194k segs) | 660.1 ms + 4.9 ms (238 rings) | **43×** |
| 5 m (75 levels) | 25.1 ms (394k segs) | 1332.2 ms + 18.8 ms (489 rings) | **54×** |

Two structural causes, both admitted in comments but not measured: `contour_rings`
(`contours.rs:292-333`) sweeps the FULL grid **once per level** (old code: one sweep total,
`contours.rs:404-438`), and `chain_segments_into_rings` (`contours.rs:226-284`) is O(n²) in
per-level segment count with a linear scan per appended vertex. wasm typically runs 1.5–3×
slower ⇒ **~1–4 s single-threaded freeze**. `contour_grid_reductions` only reduces the grid at
≥50 m intervals, so the 5/10/20/40 m rungs all march the full 1601² grid.

**Impact.** The claim "never per frame, memoised by last_interval" is literally true and the
T-639 composition is correct — `contour_interval_for_zoom` (`lod_gates.rs:130-140`) *quantizes*
to the {5,10,20,40,80} ladder, so `last_interval` memoisation still gates re-marches to rung
crossings only (verified by reading both sides of the one-wave-apart collision; `push_contours`
also runs only on settle, pan-gesture-deferred, `world_assets/mod.rs:393-396`). But the T-639
band design makes a rung crossing happen at every ~×2 zoom, and each crossing went from
imperceptible (≤25 ms) to a hitch measured in seconds. The claim's own complexity statement
("O(levels×cells)") is accurate and is precisely the regression.

**Disposition.** Document. Follow-up: restore the single-sweep march (bucket segments per level
in one grid pass — the old loop already did this), and replace the chainer's linear scan with an
endpoint hash map (O(n)); both are local to `contours.rs`.

## F-3 MAJOR — T-665: hidden layers' slots VANISH from the Outliner and ORBAT docks; the slice's dim-styling and its own unit test assert a state the live pipeline can never produce

**Evidence.** The outliner's slot rows are adapted from the **materialized SoA**
(`editor_ops.rs:942-958` `slot_rows` → `core.materialize()`), and T-665's materialize now
*filters hidden slots out* (`store.rs:467-473`). `build_layer` then treats each hidden slot's id
as a dangling entry and skips the row (`outliner.rs:214-218` "A dangling id … is skipped").
Net: hiding a layer removes its slot rows from the Outliner tree entirely — and from
`build_orbat` too (`editor_ops.rs:976-980` feeds the same filtered list), so **squads lose
their member rows in the ORBAT dock when a workflow layer is hidden**. Meanwhile
`eden_tree.rs` ships explicit dimmed-slot styling for `hidden_effective`
("a slot on a hidden layer … renders dimmed") and `outliner.rs`'s new unit test
`own_hidden_flag_marks_folder_and_its_slots` (:412) asserts `s0.hidden_effective` on a slot row
— but the test hand-builds its `SlotRow` list, bypassing `slot_rows()`, so it passes while the
integration can never render a dimmed slot row. The folder-row dim/eye glyph is unaffected
(layer rows come from `small_maps_json`, unfiltered).

**Impact.** Contradicts the slice's own in-code spec (dim, not remove), degrades the ORBAT dock
(an organizational view) as a side effect of a spatial visibility toggle, and the ORBAT SL badge
/ member list silently changes with an unrelated eye toggle. Eden keeps hidden-layer entities
listed in the hierarchy, greyed.

**Disposition.** Document. Follow-up: feed the docks from an unfiltered slot enumeration (or a
`materialize_all()` twin), keep the SoA filter for the render engine only; the dim path and test
then become live. The test should build rows through `slot_rows()`.

## F-4 MAJOR — T-664: `Go Here` on empty ground never goes there — it recenters on the current selection (or silently no-ops), while its doc claims a `set_view` wiring to the clicked point

**Evidence.** `dispatch` (`context_menu.rs:465-467`) maps `GoHere` →
`editor_ops::center_on_selection()` for BOTH takes. For `MenuTake::EmptyGround`,
`target_ids` is empty and the clicked world point is never captured or passed
(`resolve_target`, `open`, `MenuState` carry only the screen pixel). `center_on_selection`
(`editor_ops.rs:346-380`) centers on the **live selection centroid**. So: right-click empty
ground with a selection elsewhere → "Go Here" jumps the camera to the *selection*, away from
where the user pointed; with no selection → an enabled row that does nothing. The variant doc
(`context_menu.rs:53-55`) says "**Enabled** (T-664 wires it to the engine `set_view`)" — the
clicked-point wiring does not exist; the inner comment at :461-464 concedes the no-op but not
the wrong-jump case. Eden's row teleports the camera to the clicked ground point
(`batch01_context_menu.md:121`). The engine API for it exists (`set_view(x, y, zoom)` — used
three lines away) and the handler already unprojects px→world for the pick, so the point was
available.

**Impact.** The first row of the empty-ground menu — one of only three enabled rows in the
slice — does the wrong thing in the common case (selection exists) and nothing in the other.
By the module's own enablement rule ("enabled iff its backing feature already ships") this row
should have been disabled or wired through the existing frozen-camera unproject.

**Disposition.** Document. Follow-up is small: thread the world coords of the click into
`MenuTarget` and call `set_view` for the empty-ground take.

## F-5 MINOR — T-664: `Attributes…` / `Edit Loadout…` are enabled but silently dead for a multi-selection target

**Evidence.** Right-click a member of a marquee multi-selection → `resolve_target` targets the
whole selection, no retarget (`context_menu.rs:326-332`; claim verified). Clicking
`Attributes…`/`Edit Loadout…` dispatches `open_attributes(target_ids[0])` /
`open_arsenal(...)`, both of which **early-return when `selection.len() > 1`**
(`editor_ops.rs:575-577`, :597-599 — the pre-existing dbl-click A1 suppress). Enabled row,
click, nothing happens, menu closes. `ATTR-MULTI-001` is named on the row as a forward interest,
but the row is not disabled for the multi case it cannot serve.

**Impact.** Dead click on the two headline enabled rows in an ordinary state (group selected).
**Disposition.** Document; either grey the rows when `target_ids.len() > 1` or dispatch on the
hit entity.

## F-6 MINOR — T-664: the menu's window keydown has no `in_editable_field` guard (T-662 discipline broken)

**Evidence.** `ContextMenuOverlay`'s window keydown (`context_menu.rs:507-541`) handles
Escape/ArrowUp/ArrowDown/Enter with `prevent_default` whenever the menu signal is `Some`,
with no `crate::mission_history::in_editable_field()` check — the guard both sibling handlers
use (`mission_editor.rs:1023`, `mission_history.rs:487`). A right-mousedown does not blur a
focused input in Gecko/Blink, so: focus a dock rename/number field, right-click the map (menu
opens, field still focused) → Arrow keys stop moving the caret and move the menu highlight,
Enter **fires a menu row** instead of committing the field.

**Impact.** Keyboard hijack in a reachable state; violates the wave-101/T-662 in-editable-field
convention the same file cites.
**Disposition.** Document; add the guard (Esc-close may deliberately stay global).

## F-7 MINOR — T-665: refused Transform edits display as accepted in the Attributes modal

**Evidence.** `update_slot_position` on a locked slot silently early-returns
(`store.rs:1376-1378`); the Attributes Transform tab commits via `attrs_update_position` on
blur/Enter (`attributes.rs:264-294`) and the fields hold a **one-shot snapshot**
(`attrs.get_value()` at open, `attributes.rs:260`) — no re-read after commit. On a locked slot
the user types X=500, the store refuses, the field keeps showing 500 until the modal is
reopened; the map, doc and a subsequent Save all carry the old value. Nothing in the modal
disables the fields or shows the lock (the outliner slot row does show a lock hint,
`eden_tree.rs:397-408`). Coherence answer for the cross-slice question: lock is transform-only
by spec, so Attributes deliberately opens and role/tag/stance edits land — that split is
defensible Eden semantics; the *stale display* of refused position edits is the defect.

**Impact.** UI lies about document state after a refused edit.
**Disposition.** Document; disable/annotate Transform fields when `locked_effective`, or
re-read after commit.

## F-8 NOTE — T-640: point-in-ring probe on the candidate's boundary flips selection (synthetic-only)

Harness case E: a higher closed ring wholly OUTSIDE the candidate but whose first vertex lies
exactly ON the candidate's edge causes the candidate to be rejected (ray-cast even-odd counts
the on-boundary probe as inside). Distinct iso levels cannot share a marching-squares crossing
point (same-edge crossings at different levels lerp to different points), so on real DEM data
this needs an exact vertex-on-chord coincidence — measure-zero in f64. Documented because the
code comment ("a single interior point settles containment", `contours.rs:363-365`) is silent
on the boundary case the even-odd test does not define.

## F-9 NOTE — T-665: "first entityIds match" is yrs-map-iteration order — nondeterministic under dual layer membership; outliner disagrees by construction

`materialize`'s reverse index (`store.rs:446-448`, first-wins via `or_insert`) and
`slot_first_layer` (`store.rs:2891-2903`) both resolve "the slot's layer" by **YMap iteration
order**, which is hash-order, not document order. All shipped writers keep membership unique
(`move_slot_to_layer` detaches from every folder first, `store.rs:2004-2036`; paste/add append
fresh ids), so dual membership has no single-user path today — but a doc that acquires it
(future writer, hydrate of a foreign payload, CRDT merge) hides/locks the slot
nondeterministically per run, and the outliner would render the slot TWICE (once per folder,
each with that folder's flags — `build_layer` iterates `entity_ids` per layer) while
materialize picks one. Inheritance itself agrees between core (resolve-at-read walk-up,
`layer_flag_effective`, cycle-guarded, `store.rs:2861-2885`) and outliner (OR-down,
`outliner.rs:196-199`) — verified equivalent for trees. The `hidden_layers` cache is a local
rebuilt per `materialize()` call — not stale-able (question D answered).

## F-10 NOTE — T-664: backdrop z-40 sits over the docks — first click on a dock while the menu is open is swallowed; RMB-while-open re-open is plausible but untested

The click-away backdrop (`fixed inset-0 z-40`, `context_menu.rs:568-576`) covers docks (z-20)
and strip menus (z-30, T-177), so a dock click while the menu is open only dismisses the menu
(standard menu UX; Eden's own behavior not determinable from the batch). RMB while open:
backdrop `pointerdown` closes synchronously (Leptos fine-grained set), the subsequent
`contextmenu` event should then hit-test the map container and re-open at the new pixel —
composes in theory with the container handler (`mission_editor.rs:1878-1922`), but no test or
browser check covers the pointerdown→contextmenu DOM-update ordering; needs one manual verify.
Right-click over a dock closes the menu and pops the native browser menu (pre-existing: only
the map container suppresses it).

## F-11 NOTE — claims integrity: T-665's test arithmetic is wrong ("8 store tests… 333 was 325"); the two frontend totals are both worktree-local; HEAD truth is 437 / 465

Measured: T-665 ships **7** store `#[test]`s (slice diff `6f479a41`), not 8; with base
doc,mission lib = 325, its worktree count was 332 — the claimed **333 was never measurable on
any tree** (HEAD doc,mission = 336 = 325 + 4 T-640 + 7 T-665). Frontend: T-664's "435 total"
= 423 + 12 ✓ in its worktree; T-665's "425 (was 423)" = 423 + 2 ✓ in its parallel worktree;
**HEAD = 437** = 423 + 12 + 2 (run: 437/437 green). T-640's "458 map-engine tests" ✓ =
465 − 7 (its worktree preceded T-665's core tests). "12 new tests" (T-664) ✓, "4 new tests"
(T-640) ✓ by diff count; all fired in the suite runs, two re-fired individually. The three
slices' totals are mutually consistent once read as parallel-worktree measurements — but
T-665's "8"/"333" is a miscount even in its own worktree.

## F-12 NOTE — claimed line numbers drift throughout (written against pre-merge trees)

Setters actual `store.rs:1953/:1971` (claimed :1894/:1913); move refusal `:2826` in
`move_entities_in_txn` (claimed :2796); `update_slot_position` guard `:1376` (claimed :1352);
`eden_tree.rs` toggles `:178` (claimed :171); `dem_vectors.rs` colours `:30-34` (claimed
:31-34); materialize filter `:467-473` (claimed :432 — that is the fn, the filter sits 35 lines
in). Substance verified at every drifted anchor; flagged so the next wave's prompts stop
inheriting stale numbers.

## F-13 NOTE — T-640 claim nit: `forest_mass.rs:159` depends on `compose_contour_hairlines`, not `contour_segments` — which now has zero production callers

`apps/website/frontend/src/world_assets/forest_mass.rs:159` calls
`compose_contour_hairlines` (unchanged ✓, the single-colour compose the claim meant).
`contour_segments` itself (`contours.rs:395`) is untouched ✓ but after T-640 its only callers
are its own tests — a retained Class-R oracle that is now production-dead alongside its private
`march_cell`. Not a defect; worth a deliberate keep-or-cut note in the next contour ticket.

## F-14 NOTE — T-664: module-wide `#![allow(dead_code)]` on context_menu.rs; `with_shortcut` and the `shortcut` field are dead

`context_menu.rs:32` blankets the module; `MenuEntry::with_shortcut` (:215-218) is never
called and no shipped row carries a shortcut (faithful to the batch — Eden shows shortcuts only
in submenus, which this slice doesn't open). The blanket allow will also hide future real dead
code in an 890-line module. Prefer targeted allows.

## F-15 NOTE — T-664: open menu is not closed by document-changing keys; stale-target window

While the menu is open, the editor keydown still runs (guards its own keys only): Delete
deletes the menu's target under it, Space recenters the camera — the menu stays open pointing
at a stale/dead target (dispatching Attributes on a deleted id then no-ops via `read_attrs` →
None). Eden closes the menu on any action. Cosmetic; add close-on-doc-mutation or close-on-any
-handled-key later. Backspace hide-chrome correctly leaves the menu up (mount verified OUTSIDE
the four `chrome_hidden` gates, `mission_editor.rs:2205-2212`, beside ConflictDialog ✓ claim).

## F-16 NOTE — pre-existing: `--features doc` alone does not compile (4 × E0433)

`cargo test -p map-engine-core --lib --features doc` fails: store.rs tests at
:5584/:5596/:5904/:5927 use `crate::mission::compile` without the `mission` feature. Present at
base e9e4539f (18 refs) — NOT this wave's. The gates always pass doc+mission together, so it
only bites feature-sliced local runs. Cheap fix: `#[cfg(all(test, feature = "mission"))]` on
those test mods, or `doc` → depends-on → `mission` for tests.

## F-17 NOTE — T-665 doc rot: "reads back the returned skipped count" — nothing is returned

`move_entities_in_txn`'s T-665 comment (`store.rs:2812-2815`) says a caller can read back a
skipped count; the function returns `()` and neither `move_entities` nor
`move_entities_and_vehicles` surfaces one. Harmless today (refusal is deliberately silent),
but the comment promises an API that does not exist.

---

## Question-by-question closure

- **A** — Rule ≠ innermost-per-peak: depressions select ALL rings (F-1, harness-proven);
  nested twin peaks: each peak gets its own top ring when a marched level separates them
  (harness C: 30 m + 20 m selected, outer/shared rings rejected); peaks not separated by any
  level collapse to the higher peak's ring only (cartographically defensible, unclaimed);
  vertex-on-edge probe misfires (F-8, synthetic-only); tests cover none of these (bumps/ramps
  only).
- **B** — Memo composes with T-639: the interval fn quantizes to the {5,10,20,40,80} ladder, so
  continuous m_per_px still yields band-crossing-only re-marches; no per-frame or per-tick
  re-march; the regression is per-fire cost, 43–54× (F-2).
- **C** — Transcription is verbatim: labels, order, and separator positions match the batch
  tables 1:1 for both takes (empty-ground 6 rows/2 seps, on-entity 14 rows/5 seps; Place
  Comment omitted on-entity per :221, Play-as-Character swap per :204). Deviations are the
  *enabled states* (Eden: all enabled; shipped: 3 enabled) — deliberate, on record, and
  ticket-tagged. Backdrop/dock z-order and RMB-reopen: F-10. Shortcuts: none at top level in
  Eden either; the shipped `shortcut` mechanism is dead (F-14).
- **D** — Dual membership: no shipped writer path; latent nondeterminism + outliner divergence
  documented (F-9). Hide-A-also-in-B: materialize hash-order pick vs outliner double-render.
  Undo: capture_timeout=0 (`store.rs:171`) ⇒ flag flip and rename are separate steps —
  claim holds (re-fired). materialize's hidden cache: per-call local, not stale-able.
- **E** — Locked-layer menu: Go Here/Edit Loadout/Attributes all ignore the lock, which is
  spec-coherent (transform-only) EXCEPT the Transform tab's stale display (F-7); drag preview
  still moves locked slots then snaps back (visible surface of "silent", acceptable). Esc/
  Backspace: menu survives hide-chrome ✓; menu keydown lacks the editable-field guard T-662
  established (F-6). T-639 spacing tests: still meaningful — they pin
  `contour_interval_for_zoom`, which T-640 did not touch; the two-tone compose changes colour
  and ring identity, not level selection.
- **F** — Counts reconciled (F-11): HEAD = 437 frontend / 465 core all-features lib / 336
  doc,mission / 458 was T-640-worktree-true. Fired-once: suite-level all green + 2 individual
  re-fires ok.
- **G** — One new blanket `#![allow(dead_code)]` (F-14); `contour_segments` production-dead
  (F-13); no dependency/license changes in range (no Cargo.toml/lock diffs); core clippy
  all-features clean; frontend wasm clippy = 46 warnings, all in non-wave files (gate runs
  clippy without `-D warnings`); z-order constants coherent (menu z-50 > strip menus z-30 >
  docks z-20; attrs backdrop z-50 cannot coexist with an open menu).
