# Wave 103 — adversarial verification (T-076 / T-631 / T-641)

**VERDICT: 1 BLOCKER · 2 MAJOR · 2 MINOR · 4 NOTE** — all suites green at HEAD (core 345/474, frontend 442, peaks 10+1png, text_layout 13); all three fired-once claims re-fired successfully by this verifier; working tree left clean; nothing committed.

Range `e75b7377..HEAD` = three slice merges (434d05a4 T-076, 4a605031 T-631, 9700f170 T-641), disjoint file sets, merge commits add nothing (range diffstat 1206 insertions = 629+364+213 exactly). Every perturbation below was applied temporarily and reverted; `git status` clean at finish.

---

## BLOCKER-1 (T-076) — crew mutators are dead or destructive on any hydrated mission

**Evidence.** `load_row` (store.rs:3257) stores nested row objects as **opaque `Any::Map`**, "exactly like `ydoc.entityToYMap`" — its own doc comment and the T-345 `read_any_map` idiom (store.rs:3290) say nested objects are *not* tracked YMaps after hydrate. All three crew mutators (`assign_crew_seat` :938, `set_vehicle_crewed` :991 is scalar and fine, `clear_crew_seat` :1006) match **only `Out::YMap`** on `crew`. Probes (temporary tests, run then reverted):

- `probe_unboard_after_reload_works` **FAILED** — after `compile_payload`→`hydrate`, `clear_crew_seat` no-ops: crew stays `{"driver":"s0","gunner":"s1"}`.
- `probe_board_after_reload_keeps_existing_crew` **FAILED** — `assign_crew_seat("v0","commander","s2")` on a reloaded doc hits the `_ =>` arm and **replaces the whole crew map**: result `{"commander":"s2"}` — driver+gunner silently destroyed, and that wipe round-trips into the next save.
- `probe_eviction_reaches_hydrated_crew` **FAILED** — the one-seat-per-slot scan skips `Any::Map` crews: s0 ends up seated in v0 (hydrated) *and* v1 simultaneously — the exact invariant the commit claims is "enforced HERE, not in the UI".

**Impact.** This is the mainline path, not an exotic one: `mission_hydrate::adopt_payload` → `core.hydrate()` (mission_hydrate.rs:590, called from :306/:331/:363/:395) runs on every server-adopt boot and on conflict "load server". Only the same-browser IDB-restore path (`apply_update`, YMap-preserving) escapes. Open a saved mission on any other machine/session → first board wipes the crew, unboard is dead, the invariant is silent. The shipped `crew_and_crewed_survive_save_and_reload` test cannot see this because it only **reads** after reload — it never mutates. The commit's "rides the row exactly like cargo" is also false on this axis: `set_vehicle_cargo` (store.rs:834) writes cargo as a whole `Any` value (hydrate-proof read-modify-write); crew is the only nested *tracked* map on the row.

**Disposition.** Fix required before the crew surface is real: either write crew via the T-345 whole-value idiom (`read_any_map` → mutate → insert whole map, like cargo/briefing) or normalize `crew` to a YMap on hydrate. Also extend the round-trip test to mutate after reload. Documented only; nothing fixed per wave rules.

## MAJOR-1 (T-631) — Continue-without-map before hydrate settles resurrects the boot overlay forever

**Evidence.** Every task-driven boot write routes through the sticky `advance` (mission_editor.rs:1241, :1423, :2454, :2467) — verified by grep of all `boot.set|boot.update` at HEAD — *except* the deliberate operator exit: Continue does `boot.set(BootPhase::Ready)` (:2375). `Ready` is not sticky. The doc-hydrate task's tail runs **after two awaits** (IDB restore + `hydrate_from_server`) and then executes `if world_ready.get() { hand_over } else { boot.update(advance(LoadingMap)) }` (:1238-1241). `world_ready` is set only inside the engine **Ok** arm's world task (:1384) — with a failed engine it is false forever.

**Impact.** Engine init fails fast (no adapter is milliseconds); the error card is up while server hydrate is still in flight (seconds). Operator clicks "Continue without map" → `Ready` → overlay down → hydrate finishes → `advance(LoadingMap)` from `Ready` = `LoadingMap` → the spinner overlay **comes back and never leaves** (no rendezvous will ever fire; the bar state has no Retry/Continue buttons). The editor stays usable underneath (`pointer-events-none` on the bar) but is permanently occluded by a half-full boot bar — precisely the "misleading event" class the ticket exists to kill, re-opened through its own escape hatch. The native tests cannot see it: they never model the operator exit.

**Disposition.** Needs a guard (e.g. a `boot_settled`/continued flag the doc task checks, or make the Continue exit sticky-aware). Document-only here.

## MAJOR-2 (T-076) — deleting a boarded slot leaves a dangling, persisted crew reference; the UI masks it

**Evidence.** `remove_slot` (store.rs:1327), `remove_slots` (:1526) and the squad-op path scrub nothing: grep finds no crew reference outside the three mutators + tests. Probe `probe_removed_slot_leaves_dangling_crew` confirms: after `remove_slot("s0")`, v0 crew is still `{"driver":"s0"}`. The ghost rides `small_maps_json` → `compile_payload` (vehicles carried wholesale, compile.rs:229) → the saved payload. In the panel, the seat `<select>`'s `prop:value` has no matching option and no `selected` attr lands anywhere, so the browser shows the first option "— empty —" — the UI reports vacant while the doc says seated.

**Impact.** Persistent data-integrity leak on a routine flow (delete a soldier who crews a vehicle); the future roster compile drop will resolve seats against nonexistent slot ids. Not a crash; renders "gracefully" but dishonestly.

**Disposition.** `remove_slot`/`remove_slots` (and squad-removal path) should evict the removed ids from all crew maps in the same txn — same shape as the eviction scan that already exists in `assign_crew_seat`. Document-only.

## MINOR-1 (T-076) — "Same undo step as the place" is false; an unmanned place is three undo steps

**Evidence.** editor_ops.rs:1755 claims the `crewed` stamp shares the place's undo step "(same borrow scope)". Undo grouping is per-*transaction* (capture_timeout_millis = 0, store.rs:160-171), not per borrow scope; `add_vehicle`, `set_vehicle_faction`, `set_vehicle_crewed` each open their own `begin()`. Probe: add+faction = **2** steps (pre-existing wart), +crewed(false) = **3**; manned (`crewed(true)` = remove of absent key = empty txn) adds none.

**Impact.** Ctrl+Z after placing an unmanned vehicle re-mans it; the second undo strips the faction; the third removes it. Slice extends a pre-existing 2-step wart to 3 and documents it wrongly. No test pins vehicle-place step count (the existing pin covers slot places only).

**Disposition.** Fix the comment at minimum; batching the place chain into one txn is the real cure.

## MINOR-2 (T-076) — aspirational "(or holds Alt)" comment

editor_ops.rs:141 says vehicles place crewed "unless the operator turns the switch off (or holds Alt)". No Alt/alt_key handling exists anywhere on the place path (workspace grep; the only alt_key uses are copy/paste guards and the undo-gate). Stale/aspirational comment describing an unimplemented affordance — will mislead the next slice.

## NOTE-1 (T-641) — count narrative and density table need their derivations stated

- "peaks 7→10 tests": actual `#[test]` count is **8→11**; both ends silently exclude the `#[cfg(feature = "png")]` `everon_peaks_max_above_350`, which did **not** vanish — it still passes under `--features …,png` (11.3 s, re-run by this verifier). Delta +3 is honest; the endpoints count default-features only.
- "~29–35 labels/viewport" is the **synthetic saturated field's** 1280×720 count from `screen_space_density_holds_across_zoom` (ideal grid cells ≈ 41 at the 150 px pitch; greedy packing yields 29–35). It is an upper bound on density, not an Everon prediction: island-wide `PEAK_LABEL_MAX = 48` plus the real peak population means far fewer labels per viewport at z ≥ 0 (a 1280×720 viewport at z=0 is 1280×720 m of a 12800 m island holding ≤48 labels total). Sanity per the wave question: 150·2^(2) = 600 m at z=−2 → 12800/600 ≈ 21 across the island width — consistent as a bound.
- Camera relation re-cited, not re-derived: wave101.md:158 (T-639 verifier) — `ortho.rs:105-111` zoom = log2(px/m), scale = 2^zoom; struct fields unchanged at HEAD (camera/ortho.rs:105-111). `declutter_invariant_holds` reads sep through `height_label_min_sep_m` → tracks the new screen-space rule automatically (not the old world-space one).

## NOTE-2 (T-631) — the "negative non-sticky pin" is a simulation, but the positive pins have real teeth

`without_stickiness_the_reason_is_overwritten_which_is_the_bug` drives the test's *own* fold (`sticky: false` branch assigns raw), never a perturbed production `advance`. On its own that would be vacuous-adjacent. Verified compensation: de-stickying the production `advance` (temporary edit) fails `engine_failure_reaches_the_error_state…` and `a_second_failure_cannot_overwrite…` — the positive tests do exercise production code and fire. No action needed; recorded so nobody mistakes the negative test for the load-bearing one.

## NOTE-3 (T-631) — Retry/Continue edge answers

- Retry (`location.reload`, :2350-2360) has no flush-before-reload; the IDB writer is a 5 s debounce (yrs_persist.rs:138). In practice no doc edit is reachable while the Failed overlay is up: the card's backdrop blocks the pointer; keyboard mutators need a selection/clipboard that cannot exist yet; the restored doc's undo stack is empty (INIT). Risk ≈ nil today; becomes real if a Retry button ever appears post-Continue.
- Failed cannot recur after Continue: `RenderEngine::create` runs once per mount; no writer of `Failed` remains (rAF never started). The dead-pane badge (`map_disabled`, :1422, badge :2417) is armed at the Err instant and never cleared — as designed.
- Backspace/chrome-hide stays live in the continued state (keydown installed at mount, :1067-1097, engine-independent); `try_borrow_mut` fix (:2508) is sound — contended frame skips + re-schedules, `f` is not held at that point.

## NOTE-4 (T-076) — seat-picker honesty details

Options are **all** placed slots (`placed_slot_choices` = `slot_rows`), so the eviction the store performs is expressible in the UI; but nothing marks a slot as already seated elsewhere, so a cross-vehicle move is silent. Re-assigning the same slot to its own seat writes a redundant (value-identical) undo step. Panel re-render wiring is correct: every crew op → `after_local_edit` → `after_doc_change` → `refresh_docks()` → `doc_tick` bump (mission_history.rs:452, editor_ops.rs:1009); vehicles panel tracks `doc_tick` and re-reads `vehicle_rows()` per tick. `crewed` is write-only this slice (nothing reads it back; VehicleRow has no field) — acceptable per the compile-half split and labeled as such in both the doc comment and commit message. DockRight (and the panel) sit inside the `chrome_hidden` gate (:2237) — hidden by Backspace, seat state is doc state, survives remount.

---

## Verified claims ledger

| Claim | Verdict |
|---|---|
| T-076 evict+assign one txn / one undo step | **TRUE** — single `begin()`; probe: one undo restores evicted seat AND removes new one |
| T-076 borrow soundness of scan-while-mutate | **TRUE** — collect-then-mutate, sequential borrows |
| T-076 `remove_vehicle` takes crew with the row | **TRUE** (probe; no residue in snapshot) |
| T-076 `small_maps_json` carries `crew` | **TRUE** — `to_json` recursion on the vehicles root; compile carries rows wholesale |
| T-076 6 store tests + 1 panel test + round-trip | **TRUE** — all named, all pass |
| T-076 fired-once (eviction) | **RE-FIRED** — `evict.clear()` perturbation fails exactly `crew_slot_occupies_one_seat_across_all_vehicles` |
| T-631 all boot writes through `advance` | TRUE for tasks; Continue bypass is deliberate but unguarded (→ MAJOR-1) |
| T-631 sticky fold + 4 native tests | **TRUE**; re-fired: de-stickied production `advance` → 2 tests fail |
| T-631 441 frontend tests | TRUE at slice 4a605031; **442 at HEAD** = 441 + T-076's seat_model test (reconciled) |
| T-641 zoom-band hypothesis refuted (`..=`, −2.0 in band) | **TRUE** — peaks.rs:84, asserted in hardened test |
| T-641 sep = 150·2^(−z), invariant tracks it | **TRUE**; `min_sep_scales_with_zoom` pins 150/300/37.5 |
| T-641 fired-once "45 px < 150 px @ z=−2" | **RE-FIRED VERBATIM** — fixed-world-sep perturbation → `z=-2: min screen sep 45.0px < Eden pitch 150px` (150 m const on the 60 m lattice → min kept dist 180 m × 0.25 px/m) |
| T-641 text_layout 13/13 picks up sep | **TRUE** — 13/13 pass |
| T-641 labels.rs drive-by? | NO — named in the ticket summary; comment-only changes |
| Cross-slice: materialize with no engine | SAFE — `after_doc_change` engine access is Option-guarded |
| Cross-slice: T-641 labels behind Failed | SAFE — `world_assets::bootstrap` only runs in the engine Ok arm (:1373) |
| allow() growth | +1, the documented `cfg_attr(not(wasm32), allow(dead_code))` on `advance` |
| clippy | map-engine-core (doc,mission,world, all-targets): clean. website-frontend native: 91 pre-existing warnings, none in range-touched lines (CI gates wasm32 clippy) |

## Suite counts at HEAD

- `map-engine-core --features doc,mission`: **345 passed, 1 ignored** (+5+5+3 aux/doctests)
- `map-engine-core --features doc,mission,world`: **474 passed, 1 ignored**
- `map-engine-core …,png everon_peaks`: **1 passed** (11.3 s)
- `website-frontend`: **442 passed** (includes 4× t631, 1× seat_model, 6× store crew via core)
- `map-engine-render text_layout`: **13 passed**

*Verifier process note: three temporary perturbations + one temporary probe module were applied and fully reverted (`git status` clean, range diffstat unchanged). The git-lfs post-checkout hook errors during restore are cosmetic (no LFS paths touched).*

---

## Re-verification of 5f92cc4a

**VERDICT: CLEARED — BLOCKER-1 resolved through the real path; no new defect. 3 NOTE.** Adversarial pass on `5f92cc4a` ("T-076 fix: crew mutators hydrate-proof", HEAD, exactly `crates/map-engine-core/src/doc/store.rs` 213+/27−, no drive-bys, no import churn; `load_row` and compile untouched). All probes below were temporary edits to the tests mod, run, then reverted — `git status` clean under `crates/` at finish, suite re-confirmed **349 passed + 1 ignored** at pristine HEAD (claim 345→349 TRUE); clippy `-D warnings` (doc,mission,world, all-targets) clean; `website-frontend` **442/442** unchanged.

**The three original failure modes re-fired independently at HEAD — all pass** (`probe_original_three_failure_modes_at_head`, built on `compile_payload → hydrate`, not on the shipped tests): post-hydrate unboard clears the seat (collateral seat kept); post-hydrate board preserves the loaded crew AND the merged crew survives a SECOND save/reload (the wipe no longer round-trips); the one-seat scan evicts from a hydrated crew (no soldier in two vehicles). The shipped `hydrated_with_crew` helper routes through the REAL serializer — `crate::mission::compile::compile_payload(&small_maps_json(), &slots_json(), false)` → fresh core → `hydrate` (store.rs:5333) — not a hand-built `Any`, and its precondition asserts pin the hydrated read.

**New-hole check (same vehicle = eviction source AND assign target): NOT PRESENT.** `assign_crew_seat` orders evict-writes BEFORE the target read (store.rs:972–985, one `TransactionMut`), and a yrs same-txn read observes the prior write, so there is no stale-snapshot lost update. Probed (`probe_same_vehicle_evict_and_assign_no_lost_update`): hydrated `v0 {driver:s0, gunner:s1}`, `assign("v0","commander","s0")` → `{gunner:s1, commander:s0}`, driver vacated, bystander kept. The multi-evict-same-vehicle variant (one slot corruptly in two seats, sequential RMW per entry) also heals correctly (probe C). The top guard (:952) returns before any eviction when the target vehicle is missing — no partial-evict.

**Mixed-shape doc: both arms scanned.** `probe_mixed_ymap_and_any_crews_both_scanned` put a hydrated `Any::Map` crew (v1) and a hand-authored pre-fix-shaped tracked `YMap` crew (v2, the same slot in TWO seats — corruption BLOCKER-1 could really produce) in one doc; a single assign evicted from BOTH (v1 via the Any arm, v2 via the YMap arm, v2's emptied crew key removed per the omit idiom) and preserved v0's other loaded seats.

**Undo.** Post-hydrate cross-vehicle board that evicts = exactly **ONE** undo step (`undo_depth` +1); one undo restores BOTH vehicles' prior crew (evicted seat back, new seat gone, unrelated seats intact); redo re-applies both. Clearing the LAST seat (key removed by `write_crew_map`) is one step and fully restorable (undo re-inserts `crew` with the prior seat; redo re-removes); no-op clears (absent seat / absent vehicle) add NO step thanks to the `remove(..).is_some()` guard. (`probe_undo_depth_and_restore_after_hydrate`.)

**Deep-convert.** The `Any::Map` arm clones the whole map — ALL values preserved, non-strings included (probed: an `Any::BigInt` occupant survives a neighboring seat's RMW). The `YMap` arm keeps every `Out::Any` value (a nested plain-map occupant survives, probed) and silently drops only nested TRACKED values (`Out::YMap`/`YArray` inside a YMap crew) — unreachable by every writer in the codebase (the pre-fix writer inserted `&str` only; `load_row` writes whole `Any`; the post-fix writer writes `Any::Map`). Call-site claim verified by grep: the only doc-level crew readers/writers are the three mutators + the two helpers; `editor_ops.rs:1872` reads crew off `small_maps_json` JSON (shape-agnostic), panel reads the row struct.

**Fire-once RE-FIRED, louder than claimed (→ NOTE-R1).** Applying the commit's exact perturbation (Any::Map arm → `HashMap::new()`) fails **8** tests, the 4 new hydrate tests plus 4 pre-existing fresh-path crew tests — because post-fix the FRESH path also routes through the Any arm (the first board writes `Any::Map`, so the perturbed second board wipes the first seat during setup). Consequently the failure lands at the helper's "v0 driver hydrated" precondition, **not** at test 2's "SURVIVES" assertion with `{"commander":"s0new"}` as the commit message describes. Same defect signature (crew read as Null → wipe), inexact description of the failing assert. Substance of the claim holds.

- **NOTE-R1** — fire-once narrative inexact (precondition fires first; 8 failures not 1). No action needed; recorded so the commit message isn't read as a precise re-fire recipe.
- **NOTE-R2** — the `read_crew_map` `YMap` compat arm is **permanently untested**: post-fix, no shipped test (and no production path except an apply_update restore of a PRE-FIX IDB blob) ever creates a YMap crew, so the suite exercises only the Any arm — the migration arm my probe C validated could regress silently. Cheap pin available: a permanent test that hand-authors a YMap crew (as probe C did) and boards/unboards through it. Also: nested tracked values in a YMap crew would be dropped on first RMW — currently unreachable, worth a line if a future writer ever nests tracked types under crew.
- **NOTE-R3** — semantics deliberately narrowed, correctly: after its first post-fix mutation even a freshly-authored crew becomes a whole `Any::Map` (write_crew_map always writes whole), so per-seat CRDT merge granularity is gone (whole-map LWW) — identical to cargo/briefing, i.e. the sanctioned T-345 idiom this wave's disposition recommended; fine for the single-user editor. Separately, the test-path hydrate is itself 1 LOCAL undo step (`save_and_reload` doesn't bracket with `set_origin_init` — production's JS wrapper does): irrelevant to the shipped tests (none undo), but future undo-asserting tests on `hydrated_with_crew` must measure depth as deltas, as the re-verify probes did.

*Re-verifier process note: two temporary edits (5-probe test block; fire-once perturbation) applied and fully reverted via `git restore` (LFS post-checkout hook error cosmetic, as before). Nothing committed; working-tree doc/registry modifications pre-dating this pass left untouched.*
