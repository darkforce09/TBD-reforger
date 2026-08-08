# Wave 142 — adversarial verification (T-778 · T-779 · T-780 · 00c5d01b)

Verifier: Claude (Fable 5), 2026-08-08. MERGED MAIN at `00c5d01b`, wave base `3d2aad32`.
Harness: private `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-verify142` on the HOST via
`distrobox-host-exec` (deleted after this report — see the close). Tree left byte-clean; every
perturbation below was restored and re-verified with `git status --porcelain` (empty).

## Measured totals (list vs run, both reported — never trusted)

| Suite | `--list` total | run total | verdict |
|---|---|---|---|
| website-frontend | **1041** | **1041 passed, 0 failed** | agree, matches expected 1041 |
| map-engine-render | **67** | **67 passed, 0 failed** | agree, matches expected 67 |

`dem::peaks::tests::everon_peaks_max_above_350` **PASSES on main** (11.66 s — real DEM decode, so
the LFS blob is a real file here, not a pointer). Derivation note: the test is behind
`--features png`; a bare `cargo test -p map-engine-core everon_peaks_max_above_350` reports
`0 passed … filtered out` because the test is not compiled at all — that is a feature gate, not a
skip, but do not read the bare invocation as a pass.

`cargo check -p website-frontend --target wasm32-unknown-unknown` on main: clean (see close).

---

## FINDINGS

### F-1 · MAJOR · `mission_editor.rs:2553` / `:3524-3531` / `:4664-4683` — `selected_connection` is never reconciled with the document or with the non-map selection routes, so map **Delete can fire at a stale edge**

**What is wrong.** T-780's map edge selection (`selected_connection`, page-local signal at
`mission_editor.rs:2553`) is cleared in exactly three places: a pointer HIT (`:4668`), a
non-additive pointer MISS (`:4682`, re-set), and the Delete arm itself (`:3527`). Nothing clears it
when the **document** changes out from under it, and nothing clears it when the selection is
changed through a **non-map route**. The in-code claim "the two selections can never both be live
… mutually exclusive by construction" (`:3518-3520`) only holds for map clicks — the construction
does not cover `editor_ops::select_slot` (the Outliner row, `editor_ops.rs:2709-2726`) or the
`select_by_id` router, neither of which knows the signal exists.

**How proved (source, exhaustive).** `grep -n selected_connection mission_editor.rs` → 6 sites
total: declaration (2553), native ack (2558), Effect read-only (3292), Delete arm (3525-3527),
pointer arms (4668, 4682). No reconcile anywhere; the lane Effect's own comment concedes "a stale
selection is inert: it tints nothing" — inert for the TINT, not for the Delete arm that consumes
the same id.

**Two concrete failure modes:**

1. **Delete removes the wrong thing.** Pick an edge on the map (slot selection cleared), then
   select a slot from the **Outliner** (`select_slot` sets the slot selection and highlights the
   row; `selected_connection` still `Some(edge)`). Press Delete. The arm (`:3525`) matches
   `Some(id)` first and deletes the **edge** — the highlighted slot the operator is looking at
   survives, an amber line somewhere else vanishes. Both selections were live at once; the
   exclusivity claim is false.
2. **A stale id re-manufactures the T-779 defect in the same wave that fixed it.** Select an edge,
   then Ctrl+Z the edit that created it (or delete its endpoint from the panel, or let an IDB
   restore swap the doc). The edge is gone from the document; `selected_connection` still holds its
   id. Press Delete: the arm calls `editor_ops::delete_connection(stale_id)`
   (`editor_ops.rs:3614-3634`), whose guard is `connection_count() == 0` — **not** "does the doc
   hold this id". With any other connection present, `remove_connection(stale_id)` no-ops in the
   doc (`store.rs:2151-2154`, unit return, yrs map-remove of an absent key), yet `delete_connection`
   returns **true** → `after_local_edit()` → the mission is **dirtied**, `doc_ver` bumps, a persist
   is scheduled, and (subject to yrs skipping empty txns for the step itself) the operator gets
   "unsaved changes" over a document that did not change — exactly the "success over a write that
   never landed" class T-779 closed for the Arsenal in this same wave. With zero connections left,
   the guard returns false and the Delete keypress silently does nothing (the slot branch is never
   reached because the match already consumed `Some`).

**Impact.** No operator-authored data is destroyed in mode 2; mode 1 deletes a real, live edge the
operator did not intend to delete (recoverable by one Ctrl+Z, but it is a wrong destructive action
taken over a highlighted selection that said otherwise). Both contradict the shipped ticket's own
stated invariant.

**Disposition / fix shape (in-wave, small):**
- `mission_editor.rs:3525` — resolve before firing: take the id only if it still names a live edge,
  else fall through to `delete_selection()`. The Effect and the arm already share
  `live_connection_segments`; a `segs.iter().any(|s| s.id == id)` (or a
  `connection_rows_json` contains-check via a tiny `editor_ops` helper) is one expression:
  `selected_connection.try_get_untracked().flatten().filter(|id| edge_is_live(id))`.
- `editor_ops.rs:3624` — fix the verb's own lie: gate `delete_connection` on the **id being
  present** (the rows JSON, or give `remove_connection` a `bool` like `add_connection` has) instead
  of `connection_count() == 0`. That closes mode 2 at the single shared vocabulary, for the panel
  path too.
- Optional hardening for mode 1: clear `selected_connection` inside the `doc_tick` Effect when the
  selected id no longer resolves (untracked set, guarded on change), and/or clear it on the
  slot-selection routes. The arm-side `filter` alone makes Delete correct; the Effect-side
  reconcile also stops a later re-armed id (the `:3522` comment's own "silently re-armed" case,
  which today is reachable via panel-delete → Ctrl+Z, not only via the map arm).

### F-2 · MINOR · `editor_ops.rs:3624-3628` — `delete_connection` reports `true` for an id the document does not hold

Called out separately from F-1 because it is the *verb's* defect and predates T-780 (T-672 shipped
it): the guard is a count, the ack is invented, and `remove_connection` returns unit — the same
discarded-answer shape T-779's audit hunted in the same file (its audit scope was
`core.<method>(` **return types**, and `remove_connection` returns `()`, so it legitimately fell
outside that scan — this is the gap in the scan's shape, not a miscount). Until T-780 no caller
could realistically pass a dead id (the panel's row buttons re-render from `doc_tick`); the map
selection now can (F-1 mode 2). Fix shape above.

### F-3 · NIT · `ruler_tool.rs:1483-1507` — the RENDER_CTX Class-R pin is substring-hollow to a decoy, though it does catch the honest regression

Attacked both ways (perturb → run → restore, tree clean after each):
- **Honest regression** — `register_render_ctx` rewritten to the pre-T-778 direct write
  (`RENDER_CTX.with(|c| *c.borrow_mut() = Some((engine, host)))`):
  `the_render_ctx_seam_is_installed` **FAILED**. The pin is not hollow against the defect it was
  written for.
- **Adversarial decoy** — body `let install_seam_later = (); … c.replace(Some((engine, host)))`:
  test **ok** (1 passed). The `install_seam` needle is a substring match and `RefCell::replace`
  dodges the `borrow_mut` negative, so an adversarial (or unlucky-refactor) rewrite greens the pin
  while restoring the un-unregisterable registration.

Calibration: Class-R pins guard regressions, not adversaries, and every Class-R pin in this
codebase has this ceiling. Cheap tightening if wanted: needle `"install_seam(&RENDER_CTX"` (the
call survives `live_code` — it is code, not a string) and a negative on `".replace("` alongside
`borrow_mut`. FILE-grade, not in-wave.

### F-4 · NIT · `mission_editor.rs:3290-3306` — connection lane vs. drag preview

During a slot/vehicle drag the drag-preview lanes re-pack per pointermove but the connection lane
re-binds only on commit (`doc_tick`), so an edge stays pinned to the *committed* endpoint until
pointer-up. This is exactly `SquadLinks`' behaviour (`upload_squad_links` also runs only in
`after_doc_change`/rebind), so it is consistent, cosmetic, and disclosed here rather than filed.

---

## THE THIRTEEN ATTACKS, ITEM BY ITEM

**1. T-778's refusal to guard `select_tool::register_editor_selection` — CORRECT.** Read the full
body (`select_tool.rs:737-832`) and every closure it forgets: `count`/`ids` read the leaked
`Rc<RefCell<Vec<String>>>` selection; `selfcheck`/`marquee_selfcheck` read the doc handle
(`Rc<RefCell<Option<…>>>`) and answer honest booleans; `probe`/`probe_marquee`/`probe_move`
(`:587-676`) mutate only the **camera** via the engine handle and never write the doc. The defect's
precondition — a write onto a DISPOSED reactive signal reported as success — is absent: there is no
reactive state anywhere in the closure set, every handle is a leaked `Rc` that cannot die, and the
bridge is read-only w.r.t. selection/doc, so it has no success to misreport. `Closure::forget()` is
irreversible (the only conceivable "unregister" is `Reflect::delete` of the window property, which
would break the S5 smoke that reads `__editorSelection` across the page lifetime by design — the
same lifetime `__missionDoc` takes). The defect is NOT reachable there; the refusal stands.

**2. The three lifecycle cases + the `&&` tuple — VERIFIED BY PERTURBATION.**
- Removed the `is_same_registration` guard from `ruler_tool::unregister_seam` (unconditional
  `take()`): `an_older_owners_cleanup_does_not_clobber_a_newer_registration` **FAILED
  ALONE** (shapes 1, 2 and the Class-R pin all stayed green) — exactly the differential the ticket
  claims, across all four native seams in the table.
- Removed the `on_cleanup` from `install_seam`: shape 2
  (`a_seam_is_unregistered_when_its_owner_is_cleaned_up`) **FAILED** (shape 3's final
  own-cleanup assert fails with it, which is correct — with no cleanup at all nothing ever clears).
- Both restored; tree clean; full suite back to 1041/1041.
- `RENDER_CTX`'s `&&`-on-both-`Rc`s is the right direction and **cannot strand a seam**: a
  cleanup's `mine` is the exact pair it installed, so against its *own* live entry both `ptr_eq`s
  hold and it clears; against any *other* pair (including a partial overlap, e.g. a remount reusing
  one handle) at least one leg fails and it leaves the LIVE registration alone. The only residue of
  a partial match is a deferred unregister of an entry that has already been overwritten — the
  documented worst case, and the safe one. `||` would let a half-matching stale cleanup kill a live
  remount; `&&` is correct.

**3. The copied `install_seam`/`unregister_seam` — cannot drift silently in the defect
direction.** The identity vocabulary (`SeamRegistration`, `pub(crate)`) is genuinely shared — one
definition, `validation_panel.rs:393`, imported at `ruler_tool.rs:53`. The two mechanism copies are
line-identical today (diffed both `unregister_seam` bodies: byte-same logic). What catches drift:
**each copy has its own 3-shape behavioural table over its own seams**
(`validation_panel::f5_seam_lifecycle` for its four; `t778_seam_lifecycle` for these four), and my
perturbations proved the ruler copy's table reddens without touching validation_panel's — so a
defect-shaped drift in EITHER copy reds that copy's table on its own. Only a behaviour-preserving
divergence can go unnoticed, which is cosmetic by definition. The in-source FOLLOW-UP to lift the
pair beside the trait and delete the copy is the right end state; until then the tables hold it.

**4. Is the RENDER_CTX pin hollow?** No for the regression it targets, yes to an adversarial decoy
— **F-3**, with both runs' verdicts quoted there.

**5. The `refused` persistence state — reasoning and implementation both hold.**
- The reasoning is verified against the code: with the tail correctly gated, a refused pick no
  longer dirties (`commit_one_write` fires no tail → `dirty` unset), so without the third state the
  line would render `PERSIST_CLEAN` green over a pick that never landed. Real defect, real fix.
- **Stuck-on impossible:** `persist_refused.set(!took)` runs on EVERY persist
  (`arsenal.rs:1308-1314`), not just refusals, so the first pick the document takes clears it; a
  reopen is a fresh component with a fresh `false` signal.
- **Green-over-refusal impossible:** `refused` is checked FIRST in the verdict closure
  (`arsenal.rs:2082-2093`) and `editor_ops::set_loadout` has exactly ONE production caller — the
  `persist` closure (`grep`: arsenal.rs:1308 is the only non-test, non-string site) — so no write
  can bypass the flag. Cargo edits route through `persist_cargo` → same closure; the T-686 import
  applies through the same single persist. The `PERSIST_ALWAYS` "every pick is written the moment
  you make it" lead is dropped on the refused branch — the contradiction was noticed and handled.
- Boundary case, accepted: the mount-time `seed_slot_cargo` (`arsenal.rs:1266`) does not set
  `persist_refused` on a refusal — but a seed is not an operator pick, its tail is now correctly
  gated (item 7), and the first real pick will surface the refusal, so the line never lies about
  an authored action.

**6. The 159 call sites — INDEPENDENTLY RE-DERIVED, EXACT.** My own scrubber (comments stripped,
string/char literals blanked, `#[cfg(test)]` modules removed by brace matching — written fresh, not
the slice's) counts **159** `core.<method>(` occurrences in live `editor_ops.rs`. Cross-referenced
every method against its `doc/store.rs` signature: the non-unit-returning mutators called there are
`update_slot_loadout` ×4 (three now consumed via `commit_one_write`/`.then_some`/direct return, the
fourth counted by `commit_writes` — the exact 3 fixed + 1 already-counted the ticket claims),
`duplicate_comment` (checked, `if !… { return }`), `add_connection` (returned), `place_composition`
(`written.is_empty()` gate at `editor_ops.rs:5386-5390`), `force_to_formation` /
`clear_all_editor_hidden` / `rotate_entities` ×2 / `update_entity_transforms` ×2 (all returned or
compared). Remaining deliberate discards, neither a surviving instance of the defect:
`let _ = core.remove_connections_touching(id)` (`:492`, inside `delete_selection` — the tail is
owned by the `remove_slots` that follows, and the selection is pruned to live ids on every
`after_doc_change`, so a phantom-tail path through it is not reachable) and the three
`seed_cargo_in_core(…)` statement-callers (`:4327/:4558/:5422` — each sits inside a place commit
whose tail is owed to the placement itself, and the function's T-779 comment says the discard is
known). The `remove_connection` unit-return gap is F-2 — a shape the return-type scan structurally
cannot see, not a miscount.

**7. `seed_slot_cargo` — fix right, caller behaves.** The tail Option now carries the sink's
answer (`.then_some(json)`, `editor_ops.rs:2000-2001`), the `after_local_edit` tail is gated on
that same Option (`:2003-2005`), and the sole caller (`arsenal.rs:1266`) treats `None` with
`.or(loadout_json)` — a refused seed falls back to the stored loadout instead of minting a step.
`seed_cargo_in_core` likewise returns the sink's bool.

**8. THE CHAIN — attacked hard, not broken.** Every route by which the document changes or is
swapped was traced to `doc_tick`:
- **Committed edits** (all 56 `editor_ops` sites + hydrate/commands/editor's 3): `after_local_edit`
  → `after_doc_change` (`mission_history.rs:271-278`) → `refresh_signals` (`:423`) →
  `editor_ops::refresh_docks` (`:528`) → `ctx.doc_tick.set(get_untracked()+1)`
  (`editor_ops.rs:2691-2692`) — the very signal handed to `set_ctx` at `mission_editor.rs:3232`
  and read by the lane Effect at `:3291`. **Both connection mutators**
  (`complete_connect`/`delete_connection`) and the delete cascade tail through it.
- **Undo / redo**: both call `after_doc_change` (`:245`, `:263`). Same chain.
- **IDB restore, success**: doc swap at `mission_editor.rs:3746` → `refresh_hud()` (`:3753`) →
  `refresh_signals` → bump. **Failed/aborted restore** (`apply_update` err or empty blob): the doc
  is never swapped, so there is nothing stale to show — no bump needed, none owed.
- **Server hydrate**: `mission_hydrate.rs:1023` → `rebind_engine_from_doc` → `refresh_signals`
  (`:335`) → bump; the adopt path additionally goes through `after_local_edit`
  (`mission_hydrate.rs:496`).
- **Engine-mount handshake, both orders**: restore settles first → engine task's
  `if restore_settled.get() { rebind_engine_from_doc() }` (`mission_editor.rs:3931-3934`) bumps
  after the engine exists; engine mounts first → the restore task's `if engine_mounted.get()`
  (`:3815-3817`) rebinds. Before either, the Effect's initial run at creation (inside `on_load`,
  AFTER `set_ctx`, BEFORE `refresh_hud()` at `:3309` which bumps again) reads the live doc
  directly, so the seed bind happens even with zero bumps.
- **Mission switch**: a route remount — new page, fresh `doc_tick`, fresh Effect, fresh (empty)
  `selected_connection`.
- **Effect legality**: `Effect::new` inside the `on_load` body has four pre-existing siblings
  (`:2869/:2919/:3646` etc.) and `on_cleanup` is used there, so the closure runs under the reactive
  owner — the Effect is real, not a silently-never-registered closure.
- The one window where the lane can transiently show nothing: engine mounted while restore is
  still in flight — identical to the slot glyphs' seed-bind window, resolved by whichever of the
  two handshake arms fires, and the boot overlay is up for the pathological "hydrate never
  returns" case. Not a stale lane; an unbound-yet lane.

**I could not find a mutation or swap path that misses `doc_tick`.** What I DID find on this
surface is F-1: the lane is always right; the *selection over it* is what goes stale.

**9. Do the pins actually constrain, absent a browser run?** Partially, and honestly labelled.
What they constrain for real: every LINK of the chain is a live-code call-relation assert
(`every_history_path_reaches_the_doc_tick_the_lane_binds_on` — sever any of
undo/redo→`after_doc_change`, the three heads→`refresh_signals`,
`refresh_signals`→`refresh_docks`, `refresh_docks`→`doc_tick.set` and it reds), the feed pin
requires `connections_bind` + `live_connection_segments` + `connection_lane_verts` + a tracked
`doc_tick.get()` inside the MOUNT body specifically, and the self-canary
(`connection_pins_are_load_bearing`) proves needle-deletion fires the asserts. What they cannot
see: that the Effect actually re-runs in a browser, that `unproject_xy` yields a sane tolerance,
that the lane is visually where order 26 says. Those stayed unobserved, the slice says so, and
F-1 — found by reading, provable only in exactly the undo-then-Delete session nobody ran — is a
live demonstration of what the no-browser gap costs. The pins would stay green under an
owner/scheduling regression of the Effect itself; they'd catch every source-shaped severing.
Calibration: same evidence standard T-760/T-748 shipped on; adequate for the lane, and the F-1 fix
should get the browser pass the wave never had.

**10. `draw_order.rs` renumbering — SAFE, verified at every consumer.** `lane_order` is consumed
in exactly two places outside its own module (`engine.rs:960` — `> WorldTrees` threshold;
`engine.rs:2418` — sorted-insert position), both RELATIVE comparisons; the u8 is never persisted,
never serialized, never crosses a wire. The wire vocabulary (`role_id::SEA..MISSION_ZONES`,
`MAX = 10`) is character-for-character untouched in the wave diff, so shifting orders 26-34 breaks
nothing that stored the old numbers — nothing stores them. `ALL_LANES` 34→35 is pinned exhaustively
(tag-set == `0..35`, plus `lane_order_is_all_lanes_order`). The `None` arm of `lane_role_to_u32`
for `MissionConnections` is right for a typed-bind lane and is pinned in BOTH directions
(`mission_connections_has_no_wire_upload_id`: `to_u32` is `None` AND no id in `0..=MAX` resolves to
it), so the generic upload path cannot reach the lane by number even by accident.

**11. Delete — genuinely one deletion path; T-662 not weakened.** SPA-wide grep: exactly ONE
`core.remove_connection(` (inside `editor_ops::delete_connection`) and exactly TWO callers of
`delete_connection` — the panel row (`mission_editor.rs:1905`) and the map arm (`:3528`). The
rewritten `backspace_hides_chrome_and_does_not_delete` still forbids everything the old one
forbade: the combined `"Delete" | "Backspace"` alias (same assert), Backspace-must-not-delete (the
window scan from the Backspace arm to the catch-all — which now spans the KeyE/KeyR/G arms too, so
it is STRICTER than before), and Delete-still-deletes (`delete_selection()` required between the
two arms). What it deliberately stopped asserting is that the Delete arm is a single expression —
which is the T-780 change itself, and the new `map_delete_calls_the_panels_delete_connection` pin
adds the ordering fact (selection read BEFORE the verb, `delete_selection` surviving as the other
branch) the old pin never had. The keymap census still sees one `"Delete"` arm with one `!modk`
guard. No weakening — but note the arm's new body is where F-1 lives.

**12. `upload_hairline_segments` refactor — byte-equivalent.** Extracted the pre-wave body from
`3d2aad32:engine.rs` and diffed it against `upload_hairline_lane` + the new 4-line
`upload_hairline_segments`: the moved body is verbatim (STRIDE, the empty/misaligned
`remove_lane` + `set_vector_stat(0)` early-out, ANCHOR subtraction, buffer init, `upsert_lane`) —
the only change is the `lane_role_from_u32` resolution hoisted into the public wrapper, which is
the identical control flow for every `role_id` caller. `Contours`/`ForestOutline`/`SquadLinks`/
`MissionZones` behaviour unchanged by construction; the 67/67 render suite (which includes the
hairline lane pins) agrees.

**13. No affordance on the line — the reasoning is sound.** The wave-129 invariant polices a
PROMISE (cursor/affordance/probe saying "clicking works") that precedes capability; T-780 paints no
promise at all — the tint is applied after the pick, by the lane Effect, as a consequence. That is
exactly the slot pick's contract (slots have no hover affordance either), and an edge pick IS a
selection gesture: it routes to no inspector, asks `route_target` nothing, and the pin
(`no_second_delete_path_and_no_hardcoded_kind_list`) keeps a kind-vocabulary from creeping in.
Inventing a hover affordance would have re-created the two-answers defect. Absent-but-would-work is
the operator's FILE category by standing rule; here nothing is even absent-but-promised. Sound.

## CROSS-CUTTING

- **Wave-127 z-rule**: the wave diff contains ZERO new `update_slot_position` /
  `move_entities_and_vehicles` callers (grepped the full `3d2aad32..HEAD` diff; the only hits are
  prose and the pre-existing drag-commit site, which passes real `zs` via `keep_z_rows`). T-780's
  claim of writing no position is true: its block is read-only over `SlotSoa.xy` +
  `vehicle_points()`, on their way to an f32 vertex buffer and a distance test, never to a
  `position` write.
- **00c5d01b re-derivation — the comment is RIGHT this time.** Independent scrub-count:
  `after_local_edit(` in live SPA code = **56 in `editor_ops.rs`** + 1 each in
  `mission_hydrate.rs`, `mission_commands.rs`, `mission_editor.rs` = **59 call sites SPA-wide**
  (60 total occurrences minus the definition in `mission_history.rs`). All three previously-wrong
  parts now match measurement, including the file list `mission_commands.rs` was missing. The
  reworded paragraph does NOT contain the `arsenal_production_src()` split token (it names "such
  attribute" without spelling `#[cfg(test)]`; verified against the splitter at
  `arsenal.rs:6277`), and `t739::arsenal_cites_live_set_loadout_lines` is green in the full run —
  the pin still sees the cites below the comment.
- **No `.py` file and no scratch/`tmp_*` file** anywhere in `3d2aad32..HEAD` (name-only diff and
  added-files log both empty of matches).
- **Registry/tickets**: not audited beyond the diff scan — out of verify scope, nothing anomalous
  seen in passing.

## IS MAIN SAFE — **YES.**

1041/1041 + 67/67 green in a private target dir, list and run totals agree, wasm32 check clean,
the peaks test passes on main, the tree is clean, and no finding is a BLOCKER: nothing is broken at
rest, no gate reported success over unexamined code, and no data is at risk by default. F-1 is a
MAJOR against T-780's claims — a stale/coexisting edge selection makes map Delete either delete a
live edge the operator did not target (one Ctrl+Z recovers it) or dirty an unchanged mission — and
under the no-deferral regime it should be fixed before the wave closes (fix shape in F-1/F-2:
~3 lines in the Delete arm plus an id-presence gate in `delete_connection`). It requires a
deliberate cross-route selection sequence or an undo over a live selection; it cannot fire on its
own.

## VERIFIED-CLEAN REGISTER — what I attacked and FAILED to break

- **The doc_tick chain (the T-069/T-672 hunt, the wave's stated priority):** traced every
  document-mutating and document-swapping path in the SPA — all 59 `after_local_edit` sites (56
  editor_ops + 3), undo, redo, IDB restore success, IDB restore FAILURE (empty blob and
  `apply_update` err), server hydrate (adopt and trusted-local), the engine-mount handshake in
  both settle orders, the pre-engine seed bind, mission switch remount, and the connection
  mutators specifically — **no path misses `doc_tick`**. The lane cannot go stale; only the
  selection over it can (F-1, which is not a lane defect).
- **T-778 shape-2/shape-3 differential:** two perturbations (guard deleted; cleanup deleted) —
  reddened exactly as claimed, shape 3 ALONE for the guard, across all four native seams; restored
  green.
- **RENDER_CTX honest regression:** direct-write rewrite → pin FAILED as designed (the decoy
  ceiling is F-3, a NIT).
- **`&&` tuple identity stranding:** exhausted the match matrix (own pair / foreign pair / partial
  overlap) — no strand exists; a cleanup always fully matches its own registration.
- **The select_tool refusal:** audited all seven forgotten closures for any reactive read/write or
  misreportable success — none; the refusal is correct on the evidence, not just on the argument.
- **Copy-drift between the two seam mechanisms:** diffed the bodies (identical) and proved
  per-copy behavioural coverage by perturbing one copy and watching only its table redden.
- **T-779 stuck-on / green-over-refusal:** single production caller of `set_loadout`, flag set on
  every persist, refused-checked-first — both failure directions closed; boundary (mount-time
  seed) reasoned safe.
- **The 159-site audit:** re-derived with an independent scrubber — 159 exact; every non-unit
  return either consumed or deliberately-and-safely discarded; the only structural gap is the
  unit-returning `remove_connection` (F-2, MINOR).
- **The 56/59 comment (00c5d01b):** re-measured independently — exact; split-token absence
  verified against the real splitter.
- **draw_order renumbering:** checked every `lane_order` consumer, the wire `role_id` table, and
  persisted-state surfaces — nothing depends on the old numbers.
- **Hairline refactor:** old body vs new body diffed — verbatim; role-id path control flow
  identical.
- **T-662:** old prohibitions mapped onto the new assertions one-for-one — nothing the old pin
  forbade is now allowed.
- **z-rule:** full wave diff grepped for new position writers — zero.
- **Hygiene:** no `.py`, no `tmp_*`/scratch survivors, tree clean at every checkpoint.
- **Harness honesty:** list vs run cross-checked on both suites (1041=1041, 67=67); the peaks
  test's feature gate identified and the test *actually executed* (11.66 s, PASS) rather than
  trusting a filtered-out "ok".

*Private target dir `/home/Samuel/.cache/tbd-target-verify142` deleted after the final wasm32
check — stated in the close-out below the report.*
