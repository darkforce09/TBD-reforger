# Wave 111 — adversarial verification (T-645 · T-655 · T-693)

**VERDICT: 1 BLOCKER · 2 MAJOR · 4 MINOR · 7 NOTE.** Range `a167ba54..369aefa3` (three slice merges).
Suites at HEAD: map-engine-core `--features doc,mission` **453 + 1 ignored** (+5+5+3 doctest bins) green ×2
(before and after all perturbations); website-frontend native **690/690** green ×2; validate **68/68 under
BOTH `--features mission` and `--features doc,mission`**. Three re-fires (one per slice) went RED with the
predicted shapes and were restored byte-exact; `git status` at exit = the pre-existing operator-log
modification only. Verification used temporary scratch perturbations + one temporary integration-test file
(`crates/map-engine-core/tests/wave111_adversarial.rs`), all removed; nothing committed.

---

## BLOCKER-1 (T-693) — repeat-merge id collision: the template-twice case silently corrupts the doc, and the doc-comment claims the opposite

**Evidence.** `RemintMap::ensure_fresh` (store.rs:4163-4170) mints `mrg-{seq}-{old}` from a `seq` that
starts at 0 **per merge call**, with **no check against ids already resident in the doc**. The header
comment (store.rs:4138-4139) claims fresh ids "cannot collide with the resident doc's ids, with the
incoming ids of THIS payload, **or with a second merge's ids**". The third clause is false whenever two
merges make the same dedup decisions — which is exactly the ticket's primary scenario (NEW-F4
"ORBAT-from-template primitive": resident BLUFOR Alpha, merge the BLUFOR-Alpha template twice to add two
fireteams). Adversarial probe (temp integration test, since removed): source = compiled BLUFOR/Alpha
{s0, s1}; resident = BLUFOR/Alpha {res0}. Merge 1: faction+squad dedup (0 seq consumed), slots mint
`mrg-1-s0`/`mrg-2-s1`. Merge 2: identical dedup ⇒ identical seq alignment ⇒ mints **the same
`mrg-1-s0`/`mrg-2-s1`** — yrs `MapRef::insert` **overwrites** the merge-1 rows (any edits made to them
since merge 1 are silently destroyed) and `append_id` appends duplicates:

```
alpha.slotIds = ["res0","mrg-1-s0","mrg-2-s1","mrg-1-s0","mrg-2-s1"]   // 5 refs, 3 slot rows
rep2 = { "slots_added": 2, ... }                                        // report over-counts: net 0 rows
```

My first probe (different side ⇒ merge-1 CREATES the squad) did NOT collide — only because create
decisions consume seq and dedup decisions don't, shifting the alignment. The safety is coincidental, not
constructed. A sibling probe confirmed the growth direction is fine: re-merging a compiled doc that already
holds `mrg-` ids re-mints `mrg-N-mrg-M-old` with no collision (assertion test passed).

**Impact.** Silent document corruption on the second application of the same template into the same side:
duplicate squad membership (ORBAT tree/compile emit dupes), lost slot rows/edits, lying MergeReport.
Reachable today only via `window.__editorCommands.merge_mission_json` (the UI is residue — see MAJOR-1),
so it is latent, but the follow-up wiring slice will hang a menu item on this primitive without store.rs in
its owns.

**Disposition.** Must fix before the merge UI wiring slice. Fix shape: `ensure_fresh` needs the resident id
space (probe the doc's maps for the candidate, or thread a resident-id `HashSet` collected in Pass 0, or a
doc-persisted merge counter) and should bump `seq` until free. Same mechanism also fixes the intra-payload
duplicate-id over-count (MINOR-4's second half). One function + one call-site thread; the 8 existing merge
tests all stay green (none merges twice with aligned dedup).

## MAJOR-1 (T-693) — `merge_mission_now` skips the post-mutation sequence: a wired merge would be invisible until the next unrelated edit

**Evidence.** `mission_commands.rs:528-595`: after `core.merge_mission_payload_json(...)` it calls ONLY
`crate::mission_history::set_dirty(true)` (+ toasts). Every other mutation site funnels through
`after_local_edit()` → `after_doc_change` → materialize → engine `slots_bind_soa`/`vehicles_bind` →
`refresh_signals` → `editor_ops::refresh_docks()` (which is the ONE `doc_tick` bump site,
editor_ops.rs:1831) → persist schedule → HUD. The merge does none of it: merged slots/vehicles never reach
the GPU (invisible on the map), outliner/ORBAT trees stale, `doc_tick` never bumps (T-655's validation
panel does NOT re-check after a merge — the cross-slice D answer is ZERO ticks, not one), the IDB persist
is not scheduled, OBJ count stale. The in-code comment "mirrors every other edit" is false — it mirrors
only the dirty flag. Repo-wide grep: `merge_mission_now` / `other_missions` have **zero call sites** (the
menu item + Ctrl+M are declared residue and genuinely absent — 0 hits in the range), so this is latent; the
smoke bridge masks it because its own doc tells callers to undo immediately (undo runs the full
`after_doc_change`).

**Impact.** The first wiring slice inherits a merge that reports success by toast while the map shows
nothing. **Disposition:** add `crate::mission_history::after_local_edit()` after the borrow drops (replacing
`set_dirty(true)`, which `after_doc_change` already does); fold into the wiring slice or fix with BLOCKER-1.

## MAJOR-2 (T-655) — the always-on panel re-compiles the whole document per edit burst; at the 360k north star that is seconds of main-thread freeze

**Evidence.** The registered payload source (mission_editor.rs, T-655 block) is
`compile_payload(&core.small_maps_json(), &core.slots_json(), false)` — the full O(n)
serialize-parse-build — executed on EVERY debounced re-eval (each `doc_tick` burst, 250 ms trailing) plus
once at mount. Measured (temp probe, native **release**, host CPU; generated 40-man-squad ORBAT):

| n slots | small_maps_json | slots_json | compile_payload | evaluate | total |
|---|---|---|---|---|---|
| 30,000 | 2 ms (0.6 MB) | 105 ms (6.4 MB) | 161 ms | 22 ms | **~290 ms** |
| 120,000 | 11 ms (2.5 MB) | 463 ms (25.8 MB) | 631 ms | 108 ms | **~1213 ms** |

Linear (4.0× slots → 4.2× time; evaluate itself linear, no quadratic rule found) ⇒ **~3.6 s at 360k native
release; wasm typically 1.5–3× ⇒ ~5–10 s of main-thread freeze 250 ms after every edit burst, and at editor
open** (the initial `set_timeout(run_eval, 0)` pass). This is the exact cost class the corpus engineered
away per-edit (T-062 `incPatchPlan` O(k) patches; T-066 moved this same compile off the React main thread —
an offload NOT carried into the Leptos rewrite, where `compile_payload` runs main-thread but only on
explicit Save/Export clicks, mission_commands.rs:296/363/395). The panel changes the cadence from
per-explicit-save to per-edit. Secondary: the expanded list renders ALL findings as flat DOM buttons (no
cap/virtualization — 3,000 rows at my 120k probe).

**Impact judged honestly against both scales.** Typical ORBAT missions (≤1k slots): ≤10 ms per re-eval —
imperceptible, the design is fine. The 360k T-060/T-062 north-star scale: the editor's post-edit
interactivity collapses. **Disposition:** MAJOR, not BLOCKER (typical missions unaffected; the debounce
does collapse bursts — verified, see NOTE-2/D). Fix directions for the ticket: incremental/windowed eval,
a slot-count circuit breaker, worker offload, or a doc-side compiled-payload cache.

## MINOR-1 (T-655) — "a rule panic can never crash the editor" is a native-only guarantee; on the shipped wasm target `catch_unwind` catches nothing

**Evidence.** `evaluate_source` wraps the engine in `std::panic::catch_unwind` (validation_panel.rs:478).
No `panic = "unwind"`/profile override exists anywhere (workspace + frontend Cargo.toml, Trunk config), and
on `wasm32-unknown-unknown` Rust panics abort (trap) regardless of profile — unwinding is unsupported on
the target, so `catch_unwind` cannot intercept; `console_error_panic_hook` logs and the wasm instance dies.
The commit message and three code comments state the wrapper as a guarantee ("a rule panic can NEVER crash
the editor"). The real defense is T-657 rule totality; the wrapper protects only native tests. **Impact:**
overstated safety claim; behavior on target is unchanged by the wrapper (harmless dead belt). Also, when the
wrapper DOES engage (native), the panel renders the quiet "No issues" empty state — a false clean rather
than an "unavailable" state. **Disposition:** reword the claims; consider a visible degraded state.

## MINOR-2 (T-655) — the no-severity-on-clean pin does not bind the panel's own evaluation seam

**Evidence.** Perturbation: made `evaluate_source` unconditionally append a fabricated Warning row →
`cargo test -p website-frontend validation_panel` = **20/20 GREEN**. `a_clean_payload_produces_an_empty_panel`
and `..._with_a_supplied_catalogue` call `default_registry().evaluate*` directly (engine level), and
`evaluate_source_runs_the_engine_and_flattens` only asserts a known finding EXISTS — so a panel-layer
fabrication/duplication bug is invisible to the suite despite the doc's "asserted at the panel level"
claim. The correct pin is `register_payload_source(clean) + evaluate_now() == []` (the thread_local seam is
host-testable — `click_to_select_is_a_no_op_without_a_registered_router` already uses the sibling seam).
**Disposition:** one-test follow-up; restored after the probe.

## MINOR-3 (T-645) — the >10 confirm is silent about the k-step undo it precedes

**Evidence.** `confirm_bulk` (editor_ops.rs): `"This will {verb} {n} entities. Continue?"`. A 50-slot
circular apply is honestly documented IN CODE as 50 undo steps (T-732 — verified real: k
`update_slot_position` txns, `capture_timeout_millis = 0`), but the one user-facing moment where that
matters says nothing — while T-693's merge toast explicitly says "(Ctrl+Z to undo.)" for its genuinely
one-step op. The operator who confirms a 50-entity pattern and dislikes it faces 50 Ctrl+Z with no warning.
The dispatch's question "is the confirm text honest about that" — no. **Disposition:** append one sentence
to the confirm ("Undo will revert one entity at a time until T-732") or fold into T-732's acceptance.

## MINOR-4 (T-693) — squad dedup keys on the faction KEY, not the faction row; duplicate in-payload ids overwrite and double-count

**Evidence (probes).** (a) Resident: faction (key `US`, name "US Army") with squad Alpha. Incoming: faction
(key `US`, name **"US Marines"**) with its own Alpha. Result: `factions_created: 1, squads_merged: 1` — the
Marines faction row is created **EMPTY** (`squadIds: []`) while its Alpha and slot fold into **US Army's**
Alpha (side-key match, store.rs:3162-3165/3222-3227). A cross-faction fold plus an orphan empty faction row
in one merge. Defensible under "a side is a side", but the report can't distinguish it and the empty
faction is UI litter. (b) Two slot rows sharing one id: both reserve one mint; the second row **overwrites**
the first (`role B` won), `slots_added: 2` (net 1), and the squad's `slotIds` holds the same id twice —
the same insert-overwrite + duplicate-append mechanism as BLOCKER-1, triggered by a malformed payload
instead of a second merge. **Disposition:** fold (b) into the BLOCKER-1 fix (a seen-set per merge);
document (a) or key squad dedup on the post-dedup faction id.

## NOTE-1 (T-693) — priority-A3 answer: dangling ownerId drops (verified); the re-mint is kind-blind; reserved-but-skipped is unreachable

Probe: a trigger `ownerId: "ghost"` (no such id anywhere — its slot row was skipped for missing `id`, and a
skipped row never enters the RemintMap) lands **without** `ownerId` — dangling-drop confirmed, per the
claim. But `ownerId: "z0"` where z0 is a ZONE in the payload is **kept**, re-minted to `mrg-1-z0` — the
RemintMap is kind-blind, so a wrong-kind reference (not "dangling" by their definition) survives as
consistently re-minted garbage-in-garbage-out. The feared reserve-then-skip window does not exist: only
object-rows-with-id reserve a mint, and every such slot/vehicle/zone/trigger row also lands (no later skip
path) — verified by reading every skip branch in the write pass.

## NOTE-2 (T-655) — debouncer: burst collapse is real; one theoretical early-fire strand

The Debouncer is a pure oracle (`bump`/`should_fire`/`take_fire`) + real timer cancellation per bump
(`h.clear()` then re-arm), so a burst genuinely collapses to one trailing eval
(`debounce_a_burst_collapses_to_one_trailing_fire` + the arm/cancel code both check out — the D answer).
Strand: if the 250 ms `set_timeout` ever fires marginally EARLY, `should_fire` = false and nothing re-arms
— the pending eval (and the "re-checking…" flag) strands until the next doc_tick. Browsers effectively
never fire early; latent, zero-cost to harden (re-arm on the false branch).

## NOTE-3 (T-645) — priority-C answer: Arrange enablement is an open-time snapshot; stale but safe

The `disabled` closure reads `selection_count()` — a plain thread_local read, no reactive dependency — so it
evaluates once when the dropdown renders (at open; the code comment admits this). Selection changes while
the menu is open (Del, etc.) leave stale enablement. Safe by re-validation: every entry point re-resolves
the live selection and no-ops (`< 2` / `< 3` / empty), so the worst case is a clickable no-op (their own
T-668 rule, violated only inside the stale window) or a dead-but-would-work row. The Arrange dropdown is
`z-50` (above the panel's `z-30`); the panel adds NO window listeners of any kind (no Esc — the T-726 pile
gains no new member; grep: zero `add_event_listener`/keydown in validation_panel.rs).

## NOTE-4 (T-645) — no-op orients and fully-locked selections still fire the history tail

`orient_selection` never reads current rotation: orienting an already-north selection North writes k txns =
k undo steps of nothing. `commit_positions` sets `any = true` even when `update_slot_position` silently
refuses (T-665 transform lock, store.rs:2015-2017 returns `()`), so a pattern over a fully-locked selection
fires `after_local_edit` (dirty + doc_tick + dock rebuild) with an unchanged doc. Same class as wave-110
NOTE-12, carried; the T-732 one-txn API is the natural place to fix both.

## NOTE-5 (bookkeeping) — three count-claims do not reconcile; everything else does, exactly

Refuted: **"Store 12/12"** (T-693 added 8 store tests; filter `merge` matches 16 incl. pre-existing; no
grouping yields 12). **"664 frontend"** (T-655's dispatch figure; actual at that commit = 687 = 645
(wave-110 base) + 22 (T-645) + 20 (T-655)). **"ONE-LINE validate.rs fix"** (+16/−6 across the rule — the
semantic change IS the single pointer-shape swap to positional `/editor/squads/{idx}` with a defensible
enumerate restructure; verified the fix itself is correct and matches the sibling rules). Reconciled exactly:
HEAD frontend 690 = 645+22+20+3 ✓; place_helpers 22 ✓; validation_panel 20 ✓ (18 module + 2 wiring pins);
mission_commands 8 module total (5 pre + 3 new) ✓; validate 68/68 both feature sets ✓; the chrome-gate pin
still counts **7** live gates ✓ (raw grep says 12, but the scrubber cuts test modules + literals — the
range's +1 raw occurrence is the t655 test's own needle literal, not a gate).

## NOTE-6 (F) — false/stale comments introduced or newly falsified this wave

Fresh false: validation_panel.rs:32 "a drag that bumps the tick 60× a second" — drags commit once at
release (T-159.19); no 60 Hz doc_tick source exists (zone bumps are click-driven). Falsified by findings:
the merge doc-comment's "cannot collide … with a second merge's ids" (BLOCKER-1); `merge_mission_now`'s
"mirrors every other edit" (MAJOR-1); the T-666 convention line "every mutation site bumps doc_tick" — the
merge is now a mutation site that doesn't. Carried un-fixed from wave 110, third/fourth sighting:
mission_editor.rs:3767 "scale bar + grid refs … reserved, not built" (both are built and mounted below);
the gate pin still NAMED `chrome_hidden_signal_gates_the_five_mounts` while asserting seven.

## NOTE-7 (F) — allow() growth +5, all precedented; zero drive-bys

`#![allow(dead_code)]` module-wide on validation_panel.rs — the ruler_tool idiom, but ruler_tool's carries
an explanatory citation and this one is bare; 3 × `#[allow(dead_code)]` garrison kernel items, each citing
the scope discovery (verified real: world store/residency expose buildings only as GPU vertex buffers +
`last_chunk`; no building-at-point accessor exists; garrison's `to_world` matches `obb_corners`'s rotation
math exactly, so the kernel will wire cleanly); 1 × `too_many_arguments` on `merge_shape_rows`
(precedented). Diff touches exactly the 8 owned files; validate.rs is the sanctioned fix; no Ctrl+M, no
MergeMission menu enum, no unrelated hunks. PatternKind lives in ungated place_helpers so the native MENUS
const compiles ✓ (claim verified).

---

## Priority-question outcomes not already covered

- **A4 (undo/redo):** merge → undo → REDO replays the whole txn byte-stably — post-redo `small_maps` and
  `slots` parse-equal to post-merge (ids stable, crew/trigger refs intact), second undo restores pre-merge
  exactly. Assertion test PASSED (temp probe).
- **A5 (deadlock class):** CLOSED in this range. The two non-field handles (`entityOrder`, `markers`) are
  hoisted before any txn (store.rs:3128-3129); the dedup read txn closes (3170) before `begin()` (3263);
  everything under the write txn uses struct-field MapRefs or the passed `&mut txn` (`append_id`,
  `merge_shape_rows`); `begin()` is a plain `transact_mut_with`. Repo grep of the whole range: no other new
  `get_or_insert_map`, and no new code opens a store txn outside store.rs.
- **C (scatter hull):** total on degenerate input — sort/dedup then early-return `< 3` distinct points;
  rejection sampling bounded (32 tries) with bbox fallback; `point_in_convex_hull` returns false for
  `< 3`; collinear monotone-chain output degenerates to 2 points; NaN tolerated via `unwrap_or(Equal)`.
  Verified by reading + the shipped goldens (`convex_hull_of_square_with_interior_point`,
  `fill_area_deterministic_and_contained`).
- **D (tick fan-in):** T-645 pattern = **one** doc_tick (single `after_local_edit` tail; the k txns don't
  bump), its k-step UNDO = k ticks collapsed by the debouncer; T-693 merge = **zero** ticks (MAJOR-1);
  T-655 never touches editor_ops. Z-order: panel `z-30` < context-menu backdrop/menu `z-40`/`z-50` and <
  strip dropdowns `z-50` — subordinate everywhere, and a context-menu click over the panel correctly hits
  the dismiss backdrop.
- **E (re-fires, perturb → RED → restore → GREEN):** T-645: circular floor `5.0 → 4.0` → 1 RED
  (`circular_golden_radius_floor_and_cardinals`), the slice's claimed fired rule. T-655: `Rollup::of`
  miscount (Error→Info) → 3 RED (`rollup_counts_by_severity` + both chip-text pins). T-693: crew re-mint
  bypass → 1 RED (`merge_remints_references_consistently`) with **exactly** the claimed shape — left
  `Some("s0")` vs right `Some("mrg-3-s0")`. All restored byte-exact; both suites re-run green after.

## Re-verification of 9ebcb8a9

**VERDICT: CLEARED — BLOCKER-1, MAJOR-1, MINOR-4 all fixed as claimed; 0 new findings.** Diff = exactly
`store.rs` + `mission_commands.rs` + `validation_panel.rs` (291/40/12 lines, no drive-bys).

**1 · Minting (BLOCKER-1).** `with_reserved` is seeded from all **10** maps the write pass can insert
into — slots/squads/factions/editor_layers/vehicles/entities/zones/triggers/compositions/markers — and
those are verifiably the ONLY insert targets (factions/squads/slots/layers/vehicles/entities direct;
zones via `merge_shape_rows`; triggers/compositions/markers in their own blocks). Key-IS-id confirmed on
both sides for the two doubted maps: merge inserts key on `new_id` with `id` field mirroring it, and the
resident side writes markers via hydrate `load_row` (keys by row `id`, store.rs:4786) and compositions
via `add_composition` (store.rs:1375, keys by `id`). `ensure_fresh` loops `seq` past every `taken` hit
AND records each mint in `taken` before mapping it; every written id passes through the table in the
Pass-0 reservation loop, so intra-payload duplicates are covered by the same guard. (Maps excluded from
the universe — meta/loadouts/items/objectives/payloadExtras/entityOrder — are never insert targets of
the merge, so exclusion is correct by construction.)

**2 · Probe re-run (temp test, removed).** Template twice into a resident-matching doc: **5 distinct
slot rows**, `slotIds` = 5 with **no duplicates**, every membership id resolves to a live row, reports
sum to the real growth (2+2 = 5−1), and the two merged vehicles crew **distinct** minted slots that both
resolve. THRICE: 7 rows / 7 memberships / 3 vehicles, still duplicate-free; every minted id is
single-prefix `mrg-<n>-<orig>` (no `mrg-…-mrg-…` chaining on template re-merge — the payload's ids never
change) with n small; seq is a u64 bumping over a finite `taken`, so exhaustion is unreachable.

**3 · Re-fire.** Perturbed `ensure_fresh` back to the naive pre-fix mint →
`merge_same_template_twice_lands_alongside_no_overwrite` went RED at **exactly** the claimed shape
(`no overwrite: left 3 / right 5`). Restored; `git diff` vs HEAD clean for all code.

**4 · MAJOR-1 tail.** `merge_mission_now` borrow ordering verified safe: the EDITOR_CTX borrow is
released inside the first `with` closure (it only clones the doc Rc), the post-await doc borrow is
scoped to the `report_json` block and dropped at `};`, and `after_local_edit` then borrows HISTORY_CTX
(a different RefCell) with only immutable, statement-scoped `ctx.doc.borrow()`s inside
`after_doc_change` — no RefCell double-borrow path. The new source comment's claim is accurate, and the
`class_r_merge_mission_now_runs_the_after_local_edit_tail` pin (scrubbed source, both needles) guards
the regression.

**5 · MINOR-4 dedupe placement.** `append_id` has 8 callers: 3 merge sites + add_slot (slotIds,
entityIds), add_squad (squadIds), slot-reassign dest, vehicle-assign. None relies on duplicate entries —
membership arrays are set-semantics by contract, and the one order-sensitive caller (reassign,
store.rs:821) removes from the source array BEFORE appending, so the dedupe is behavior-neutral there.
Masking check: the only symptom the dedupe suppresses is a `[..,id,id]` array; a future
two-rows-one-mint regression would still surface through the row-count + report-sum assertions the new
tests pin. NOTE-level observation only, no action needed.

**6 · Suites at HEAD.** map-engine-core `--features doc,mission`: **456 + 1 ignored** (+5+5+3 doctest
bins) green; website-frontend: **691/691** green — both exactly as the commit claims. Exit state:
working tree = the pre-existing operator-log/doc modifications only; probe test and perturbation both
removed.
