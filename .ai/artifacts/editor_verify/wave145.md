# Wave 145 adversarial verification — T-784 (merge 6bd25431, merged as 6422f2ff)

Verifier: Claude (Fable), 2026-08-09. Verified on MERGED MAIN at 2b8634ee.
Scope: T-784 only ("a comment can be selected by clicking it — outliner row and map glyph").
Wave 144 was verified separately (`wave144.md`); the disavow/renumber bookkeeping was checked here
for code disturbance only.

## Harness

Private `CARGO_TARGET_DIR=/home/Samuel/.cache/w145_verify_target` (host filesystem, **not /tmp**),
cargo routed through `distrobox-host-exec`. **The target dir was deleted at the end of the run.**

| Suite | `--list` total | Run total | Result |
|---|---|---|---|
| website-frontend | **1065** | **1065** | 1065 passed, 0 failed |
| map-engine-core `--lib --all-features --no-fail-fast` | **640** | **640** | 639 passed, 0 failed, 1 ignored¹ |
| map-engine-render | **67** | **67** | 67 passed, 0 failed |

¹ The one ignored is the standing `mission::flatten::tests::regen_compiler_shaped_fixture`
("manual golden regeneration") — not new, not T-784.

`dem::peaks::tests::everon_peaks_max_above_350` **RAN and PASSED on main** (LFS blob real here).
Feature set used for core: `--all-features` (the 640-test universe, as expected; the 636 variant was
not used).

Perturbation discipline: three deliberate breakages were applied to main, each proven RED, each
reverted; `git status` at exit shows the tree exactly as found (sole pre-existing entry: staged
`.ai/artifacts/editor_verify/wave144.md`). Nothing committed.

## FINDINGS

### F-1 | CONFIRMED (pre-existing, disclosed as `found_not_fixed`) | mission_history.rs:316-321 and 371-376
**What:** `rebind_engine_from_doc` and `after_doc_change` both prune the selection with
`retain(|id| live.contains(id))` where `live` is `materialize()`'s `soa.ids` — the slot SoA, which
holds no vehicle, object, or comment ids.
**Proof:** read directly at both sites; `materialize()` (store.rs:740+) builds only from the slots
map (probe C below confirms a comment id never appears in `soa.ids`). `after_doc_change` runs on
every drag commit, undo, redo, and every `after_local_edit` (all comment mutators, layer toggles,
loadout writes); `rebind_engine_from_doc` runs on the IDB restore / hydrate swap. In every one of
those, all non-slot ids are silently dropped from `ctx.selection`, then `refresh_signals →
refresh_docks → mirror_selection` propagates the pruned set, so the UI un-highlights consistently.
**Answer to the operator's question: YES — click a comment, then drag a slot (or undo, or edit
anything), and the comment falls out of the selection.** A T-781 compose (Ctrl+click comment +
slot) followed by any commit loses the comment half. Vehicles and placed objects are equally
affected, and were before T-784.
**Impact:** operator selection work lost on every doc change; no document data at risk. It is ALSO
the mechanism that currently guarantees no stale comment id ever survives into Delete (see
register) — a fix must preserve that property.
**Disposition:** pre-existing, honestly disclosed by the ticket's `found_not_fixed`; NOT a T-784
regression, so not an in-wave fix item — recorded for the operator. **Fix shape when taken:** at
both sites build `live` as the union of `soa.ids` + the key sets of `vehiclesById` /
`entitiesById` (off `small_maps_json`) + `commentsById` (off `comments_json`) of the live doc. The
union is read from the post-change document, so deleted ids still fall out (staleness protection
retained). Decide deliberately whether hidden-layer slots should keep being deselected (today they
are, because the SoA drops them — T-665).

### F-2 | NIT | editor_ops.rs:515-517, mission_editor.rs:12446-12447
**What:** the guard rationale is false. Both the shipped comment and the pin's assert message claim
`remove_slots(vec![])` "would still open a transaction — an empty undo step".
**Proof:** empirical probe against `MissionDocCore` (native, scratchpad crate, since deleted):
`remove_slots(Vec::new())` on a fresh doc leaves `can_undo() == false`; `remove_comment("ghost")`
likewise. yrs skips empty transactions; no undo step is minted.
**Impact:** none on behavior — the guard is correct hygiene either way. But the codebase's own
standard (T-739 et al.) is that shipped prose must not state falsehoods, and this one is load-borne
into a pin message.
**Fix shape:** reword editor_ops.rs:515-517 and the mission_editor.rs:12447 message to "avoids a
pointless empty transaction" — no code change.

### F-3 | NIT | mission_editor.rs:12365 vs map-engine-render/src/draw_order.rs:690-698
**What:** the ordering pin's justification misstates the draw order. The assert message says the
comment pick must precede the edge pick because "a drawn object wins over a line **under** it" —
but `draw_order` pins `MissionConnections` **above** `MissionComments` ("the ORBAT hairlines win
the overprint"). At an exact overlap the edge is the topmost pixel, yet the click selects the
comment.
**Proof:** draw_order.rs:691 (`MissionConnections > MissionComments`) against the pick order at
mission_editor.rs:4943 (comment fold) → :4988 (edge pick, miss-only).
**Impact:** the pick priority itself is deliberate, pinned, and defensible (a point glyph would be
unclickable if a line crossing it could shadow it; the line is clickable along its whole length) —
this is the intended winner, verified not accidental. Only the stated reason is wrong.
**Fix shape:** correct the message/comment at mission_editor.rs:12365 (and the matching prose in
the T-780/T-784 block) to the real rationale.

### F-4 | NIT (pre-existing class, widened) | editor_ops.rs:463, store.rs:1901-1905
**What:** `delete_selection`'s headline still says "in one undoable step", but each
`remove_comment` opens its own transaction, so a multi-comment or mixed delete is one undo step per
comment + one per T-672 edge cascade + one for the slots. The T-672 "KNOWN AND ACCEPTED" paragraph
covers the connection cascade only; the new comment loop was not added to it.
**Impact:** undo restores everything (probe B), but a mixed delete takes several Ctrl+Z presses.
Same accepted class as T-672; single-comment delete is genuinely one step.
**Fix shape:** extend the KNOWN-AND-ACCEPTED note to name the comment loop, or batch the removals
in one txn core-side.

No BLOCKER. No MAJOR. `delete_selection` returning `true` (and scheduling a persist) for the
partition is honest in every reachable state — the ghost-id route that would make it a T-779 lie is
closed by the F-1 prune (see register).

## ATTACK LOG — what was verified per brief item

**1. Selection reconcile — the "no new reconcile code" claim HOLDS.** The comment id lands in
`ctx.selection` itself (map: folded into `hit` → `apply_click`, mission_editor.rs:4943/5000;
outliner/search: router writes `*selection.borrow_mut() = vec![id]`, mission_editor.rs:3367).
`reconcile_connection_selection` (editor_ops.rs:3722) keys on `!selection.is_empty()` — kind-blind,
so a comment id drops the edge with no code of its own — and lives inside `mirror_selection`
(editor_ops.rs:2841), the ONE writer of `selected_ids` (grep: exactly one `.set`). The diff adds
zero lines to either. Attacks:
- *comment selected, then undone away:* undo → `after_doc_change` → prune drops the id → Delete
  no-ops (`ids.is_empty` → false). No stale id.
- *comment selected, then deleted from the panel:* `delete_comment` → `edit_comment` →
  `after_local_edit` → same prune + `refresh_docks` re-mirrors. No stale id, no stale highlight.
- *comment id surviving a document swap:* `rebind_engine_from_doc` prunes BEFORE the engine-mounted
  check (mission_history.rs:316-321), so even a pre-mount swap cleans it. Selection is not
  persisted to IDB at all — there is no restore route that reintroduces ids.
- *comment + edge coexisting (the wave-142 case):* every route was walked. Map click on a comment:
  line 4981 clears `selected_connection` directly AND the reconcile re-runs via
  `refresh_selection`. Outliner/search comment click: reconcile inside `mirror_selection` clears
  the edge. Edge pick runs only on a full miss AND non-additive, and `apply_click(None, false)`
  clears the entity selection in the same breath; Ctrl+click can never set an edge (line 4983
  guard). Delete's arm re-resolves the armed edge against the document (`connection_exists`,
  mission_editor.rs:3796-3804) and falls through to `delete_selection` on staleness. No coexisting
  state found.

**2. Delete semantics.** Partition is by membership in `comment_details(core)` — a set built from
`comments_json` — never a prefix (editor_ops.rs:493-496). Probe B (native, against
`MissionDocCore`): a comment with id `"note_alpha 7"` (no `cmt-`, contains a space) adds, removes
by key, and undo restores it byte-for-byte — every consuming path (`comment_points`,
`route_target`'s `commentsById` lookup, `row_router_subject`, the partition) is key-based, so
hydrated ids work end to end. Comment-only Delete: comment txns only, `remove_slots` not called
(guard at :518), edge-cascade loop runs over the empty slot half. Mixed: both halves reach their
mutators. Perturbation: replacing the membership test with `id.starts_with("cmt-")` turned
`delete_partitions_the_selection_by_what_the_document_says` RED. Reverted.

**3. Affordance invariant.** `row_routes` (eden_tree.rs) is
`row_router_subject(...).is_some_and(validation_panel::subject_id_routes)`; `subject_id_routes`
(validation_panel.rs:588) is the registered probe, and mission_editor registers probe and click
seams as `Rc::clone`s of ONE `available` closure (mission_editor.rs:3335-3380) — the same
resolution the click runs. `route_availability` narrows only `Zone`; comments pass through. The
correspondence pin's corpus is every `NodeKind` × {resolving id, refused id}, so the non-vacuity
asserts genuinely see both a selectable row (Comment × "row-yes") and inert rows.
Perturbation: hardwiring `row_routes` → `true` turned BOTH
`the_affordance_and_the_click_cannot_disagree_over_any_row_kind` and
`no_probe_means_no_affordance_and_no_fallback` RED. Reverted. No row kind found where affordance
and click disagree: Slot/Folder rows are always-clickable via their own non-router verbs
(`select_slot` cannot fail; folder click is a drop-target/child-select), which
`row_router_subject` correctly declares out of the router's domain.

**4. One document read.** `comment_points` (mission_editor.rs:2246) is the sole parse;
`comment_lane_xy` packs it; `pick_comment` hit-tests it; `mission_history::comment_lane_xy`
(mission_history.rs:430-432) is a one-line delegation and its former private parse is deleted —
grep for `serde_json` in that function: none. The map-engine-render T-748 feed pin still holds both
bind sites to `comments_bind(&comment_lane_xy(doc))` (draw_order.rs:766+, ran green in the 67).
Tolerance: `MissionDocCore::PICK_RADIUS_PX` is `4.0` at store.rs:4100, the needle
`"PICK_RADIUS_PX: f64 = "` occurs exactly once in that file, and the pin parses the literal back —
perturbing `COMMENT_PICK_PX` to `5.0` turned `comment_pick_px_is_the_slot_pick_radius` RED.
Reverted.

**5. Pointerup ordering.** Verified in live code (mission_editor.rs:4895-5013): entity pick →
connect arm (entity hits only, so an armed connection CANNOT take a note as an endpoint — clicking
a comment while armed keeps the arm and selects the note, same as any miss) → comment fold (entity
miss only, so a note over a unit never steals the unit's click; matches draw order, slots above
comments) → edge pick (full miss + non-additive only). Comment-over-line overlap: comment wins
deliberately (pinned `at_pick < at_edge`) — see F-3 for the one wrong sentence about why. The
ordering is structurally pinned, not accidental.

**6. Out-of-owns files.**
- `mission_history.rs` (49 lines): entirely the lane-feed delegation + its doc comment. REQUIRED by
  the one-read mandate (the private parse was the second reader) — necessary and confined.
- `arsenal.rs` (2 lines): cite bumps `editor_ops.rs:2142→2168` and `:2158→2184`. Verified against
  live code: `pub fn set_loadout` IS at editor_ops.rs:2168 and its `after_local_edit` tail at
  :2184 (T-784 added 26 lines above them). The T-739 pin
  (`arsenal_cites_live_set_loadout_lines`, arsenal.rs:6287+) computes the live line numbers from
  `include_str!` and compares — the touch was pin-forced, and the pin still constrains (a future
  drift goes RED). Necessary and confined.

**7. found_not_fixed.** VERIFIED TRUE — see F-1, including the operator's specific question.

## Cross-cutting

- **Wave-127 z-rule:** zero `update_slot_position` / `move_entities_and_vehicles` in the T-784
  diff.
- **Hidden slots (T-665/T-701):** comments are enumerated from `comments_json`, never
  `materialize()`; probe C confirms a comment id never enters the slot SoA. Note (not a finding):
  comments filed into a HIDDEN layer still draw, still pick, and still list — unchanged from T-748
  behavior, and consistent across all three surfaces (lane, pick, outliner), which is the
  consistency T-784 promises. Whether hidden layers should hide their comments is a pre-existing
  design question.
- **No `.py`, no scratch/`tmp_*` tests** anywhere in the wave diff (5 .rs files only).
- **Bookkeeping:** the renumber (47117d19) touched only `wave_plan.tsv` + two run-log docs; cutting
  the wave column, the remaining 574 rows hash **identical** before and after (md5 verified) — only
  the wave number moved. The disavow (2b8634ee) touched registry/docs/artifacts only, carries the
  load-bearing `This reverts commit` trailer, and disturbed no code.
- Minor register note: comment ids ride into `RenderEngine::set_selection` (map path filters only
  vehicle ids; router path filters nothing) — a no-op in the slot tint lane, same as any unknown
  id; harmless today.

## Is `main` safe?

**YES.** All three suites green with list totals cross-checked against run totals; every T-784
claim verified against live code; three perturbations proved the load-bearing pins fire; the only
confirmed defect (F-1) is pre-existing, disclosed by the ticket itself, and loses selection state,
not document data. F-2/F-3/F-4 are prose-accuracy NITs with zero behavioral impact.

## VERIFIED-CLEAN REGISTER — attacked and FAILED to break

1. **The stale-selection hunt (primary target — two prior MAJORs):** tried comment-selected →
   undo-away, panel delete, document swap (IDB restore, pre-mount and post-mount), any-edit prune,
   and Delete-over-ghost-id. Every route that removes a comment funnels through
   `after_doc_change`/`rebind_engine_from_doc`, whose prune (F-1) drops every comment id
   unconditionally before Delete can see it. **Found no reachable state in which Delete acts on a
   stale or coexisting id.** Caveat recorded: this safety currently RIDES ON the F-1 bug — a future
   F-1 fix must keep `live` sourced from the post-change document so deleted ids still fall out.
2. **Comment + edge coexistence (the wave-142 F-1 shape):** all six write routes to the two
   selections walked (map hit, map miss, Ctrl variants, outliner row, search hit, Delete arm) — no
   ordering leaves both non-empty past a mirror.
3. **Armed connect over a comment:** the fold is after `complete_connect` and the connect arm sees
   only entity hits — no edge to a comment can be minted; pinned structurally.
4. **Prefix dependence:** hunted for any `cmt-`/`starts_with` classification in the selection,
   pick, route, and delete paths — none; probe B proved the full lifecycle on a non-minted id.
5. **Vacuous pins:** perturbed `COMMENT_PICK_PX` (RED), `row_routes` (RED ×2), and the delete
   partition (RED) — all restored; `t784` pins execute and bite. The correspondence pin's
   non-vacuity asserts are genuinely exercised in both directions.
6. **Route regression on existing arms:** the `Comment` arm is appended last in `route_target`; an
   id resolving as slot/vehicle/entity/zone before T-784 resolves identically after (arm order
   unchanged, verified in diff and live source; the t784 route test re-asserts all prior arms).
7. **Vacuous-suite hazard (T-747):** core ran with `--all-features` — 640 listed, 640 ran;
   `everon_peaks_max_above_350` passed on main. Frontend 1065 listed = 1065 ran; render 67 = 67.
8. **Bookkeeping contamination:** disavow + renumber commits grepped for code paths — none; plan
   ticket set byte-identical (md5 over wave-column-stripped rows).
9. **One-read claim:** searched `mission_history` and the pointerup for any second `commentsById`
   parse — only `comment_points` remains; the lane, the pick, the router arm, the outliner rows,
   and Delete's membership set all read `comments_json`.

Attempts 1-9 produced no finding beyond F-1 (which the ticket itself disclosed) and the three
prose NITs above.

## Verification of the orphaned fix commit ace5a6d6

Verified post-mortem (the fix agent died before reporting; everything below re-measured, not
trusted). Commit is COMPLETE and CORRECT; one prose NIT found.

- MINOR | apps/website/frontend/src/mission_history.rs:315 | The `prune_selection` doc comment
  names the Class-R pin as `t784_comment_glyph::the_selection_prune_runs_over_the_whole_
  selectable_universe`, but the shipped pin lives in `mission_editor::w145_selection_prune`
  (mission_editor.rs:12569). Proved by grep: the named module holds no such test, so
  `cargo test t784_comment_glyph::the_selection_prune…` filters to zero — a reader sent to the
  named path finds nothing. Prose-only: the pin itself exists, executes, and was perturbed RED
  in five directions (SoA restored to the prune body; needles moved into a comment; a second
  `retain` added; `commentsById` dropped from the universe; `editorHidden` filtering added;
  `zonesById` admitted — each reddened its dedicated assert, tree restored byte-identical after).
