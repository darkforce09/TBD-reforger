# REPORT — T-946.86 · Wire the wave 255 dead code into production

Wave 256. Branch `slice/T-946.86`. Base `dc073f7c2`. Head `a4481558f`.

## pwd_branch

```
$ pwd && git branch --show-current
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-946.86
slice/T-946.86
```

The brief was NOT in this worktree — the worktree branched at `dc073f7c2` and the wave-256 briefs
landed on `main` two commits later (`17a6c9902`). Read via
`git show main:.ai/artifacts/editor_briefs/sept2026/wave256/T-946.86.md`. No merge taken.

## defect_verified

Pasted BEFORE any edit, at `HEAD = dc073f7c2`:

```
### HEAD: dc073f7c2
----- set_z_drag_readout -----
apps/website/frontend/src/editor/canvas/overlays.rs:32:pub(crate) fn set_z_drag_readout(readout: Option<String>) {
----- snap_elevation -----
apps/website/frontend/src/editor/canvas/gizmo_z.rs:15:pub fn snap_elevation(z: f64, step: f64) -> f64 {
----- format_height_readout -----
apps/website/frontend/src/editor/canvas/gizmo_z.rs:23:pub fn format_height_readout(z: f64) -> String {
----- dy_to_elevation -----
apps/website/frontend/src/editor/canvas/gizmo_z.rs:11:pub fn dy_to_elevation(dy: f64, scale: f64) -> f64 {
apps/website/frontend/src/editor/canvas/gizmo_z.rs:43:        let elev = dy_to_elevation(-10.0, 2.0);
----- plan_drop -----
apps/website/frontend/src/editor/panels/outliner_drag.rs:9:pub fn plan_drop(
apps/website/frontend/src/editor/panels/outliner_drag.rs:45:            plan_drop(&drag, "valid", descendants),
apps/website/frontend/src/editor/panels/outliner_drag.rs:50:        assert_eq!(plan_drop(&drag, "a", descendants), None);
apps/website/frontend/src/editor/panels/outliner_drag.rs:53:        assert_eq!(plan_drop(&drag, "c", descendants), None);
----- begin_tactical_draw -----
apps/website/frontend/src/editor/mission_editor_tests/t936_7_tactical_lane_bind.rs:9  (prose)
apps/website/frontend/src/editor/mission_editor_tests/t936_7_tactical_lane_bind.rs:51 (assert-message string)
apps/website/frontend/src/editor/state/history.rs:386 (comment)
apps/website/frontend/src/editor/state/operations/tactical_graphics.rs:44 (doc link)
apps/website/frontend/src/editor/state/operations/tactical_graphics.rs:188:pub fn begin_tactical_draw(kind: &str) -> bool {
apps/website/frontend/src/editor/state/operations.rs:83 (comment naming it as uncalled)
----- duplicate_slot_ids -----
apps/website/frontend/src/editor/state/operations/slot_ids.rs:4:pub fn duplicate_slot_ids(doc: &MissionDocCore) -> Vec<(String, String)> {
apps/website/frontend/src/editor/state/operations/slot_ids.rs:55:        let dups = duplicate_slot_ids(&doc);
```

A string match is not proof, so residual hits were classified against each file's `#[cfg(test)]`
boundary — the same cut `class_r_scrub::live_source` makes:

```
editor/canvas/gizmo_z.rs            (46 lines)   cfg(test) at 27  -> dy_to_elevation:43 INSIDE tests
editor/panels/outliner_drag.rs      (87 lines)   cfg(test) at 26  -> plan_drop:45,50,53 INSIDE tests
editor/state/operations/slot_ids.rs (59 lines)   cfg(test) at 44  -> duplicate_slot_ids:55 INSIDE tests
editor/canvas/overlays.rs          (1025 lines)  cfg(test): none  -> set_z_drag_readout:32 sole hit
```

**All four confirmed: zero production call sites.** Every `begin_tactical_draw` hit outside its
definition is prose — a doc block, two comments, one string in an assert message.

Two things the grep alone did not show, read out of the files:

1. `outliner_drag.rs` had **production code BELOW its `#[cfg(test)]`** (`LayerDrag`, `PENDING_DRAG`,
   all four `begin_*` at lines 57-87, under the test module at 26). `live_source` cuts from the
   first `#[cfg(test)]` to EOF, so half the file was invisible to every source-scrubbing pin.
2. `read_z_drag_readout` was wired but **not reactive** — called at `overlays.rs:220` as a bare
   `{ … }` expression inside `view!`, not a `move ||` closure, so it evaluated once at build time.
   Even with a writer, the chip would stay empty: the enclosing `{move || projected()…}` subscribes
   to the CAMERA, and a Z drag moves no camera.

## changes

Six files, all within owns. `1315 insertions(+), 56 deletions(-)`.

### .82 — the Z gizmo arm
* `gestures.rs` `onpointermove`: reads the arm (`z_drag.borrow().clone()`), computes the snapped
  delta, publishes `set_z_drag_readout(Some(format_height_readout(base + delta)))`, returns. After
  the pan branch, before the T-936.7 vertex preview (same mutual-exclusion reasoning).
* `gestures.rs` `onpointerup`, FIRST branch: takes the arm, **releases pointer capture
  unconditionally** (before the commit can decide it has nothing to write), clears the readout,
  commits every dragged slot's elevation in ONE `core.move_entities(slot_ids, 0.0, 0.0, zs)` txn
  => one Ctrl+Z. Zero travel writes nothing.
* `overlays.rs`: `set_z_drag_readout` bumps an `ArcRwSignal<u32>` generation; `read_z_drag_readout`
  reads it first and unconditionally; the render site became `{move || …}`. Pattern copied from
  `mission_editor.rs`'s `TOOLBAR_DISPATCH_GEN` (owner-independent, survives unmount/remount).
* `overlays.rs`: `z_drag_elevation_delta` + `z_drag_snap_step` — the gesture's one arithmetic,
  shared by preview and commit so they cannot show one number and store another. Snaps against the
  TRANSLATE ladder; guards a degenerate camera scale so an infinite elevation cannot reach the doc.
* `gizmo_z.rs` untouched, as instructed.

NOT `move_entities_and_vehicles`: the first draft used it and reddened
`t796_comment_drag::the_move_commit_partitions_comments_to_their_own_mutator`, which anchors on the
FIRST `move_entities_and_vehicles(` in the file and walks back to `LG::Move`'s `if dx != 0.0` guard.
`move_entities` is also semantically correct — `move_vehicles_in_txn` has no z column, which is why
the arm only ever resolved `initial_zs` for the slot half.

### .83 — the outliner multi-select drop
* `outliner_drag.rs`: `complete_multi_drop_onto_folder(dest, folder_descendants)` — takes
  `PENDING_DRAG`, clears the single-id latch, plans through `plan_drop`, applies every id in one
  `with_batch("outliner-multi-drop")` group. A refused plan still returns `true` (armed and
  consumed) so the drop is never handed back to the anchor-only path.
* `outliner_drag.rs`: `complete_multi_refile_onto_squad(dest)` — the ORBAT peer, refiling every
  dragged slot through `ops::refile_slot` in one batch. No `plan_drop`: a squad is not an ancestor
  of its slots, so there is no cycle to guard.
* `outliner_tree.rs`: `drag_set_for(anchor, selection, nodes)` — one shared set builder, lifted out
  of the folder row's inline `fn walk`. Takes the selection only when the pressed row is IN it;
  orders by the rendered tree, not selection arrival order, because the drop applies ids in order.
* `outliner_tree.rs`: `node_descendant_ids(nodes, id)` — the subtree answer `plan_drop` asks for.
* All FOUR arms now build a set (folder row, slot row's ORBAT branch, slot row's layer branch,
  comment row); both drops consume one, falling back to the single-id completion only when
  `complete_multi_*` returns `false`. `comment_row` takes the whole `RowAuthoring` (it is `Copy`).
* The Class-R string `complete_layer_drop_onto_folder` is still present AND still a live call
  (`outliner_tree.rs:1156`) — the pin at `outliner_tree.rs:1905` is satisfied honestly.

### .84 — the tactical draw trigger
* `zones_panel.rs`: a "Tactical graphics" section — count, kind `<select>` seeded from
  `map_engine_core::mission::tactical_graphics::KINDS`, a `data-testid="tactical-draw-arm"`
  **`<button>`** whose `arm_tactical` closure calls `ops::begin_tactical_draw(&kind)`, and a live
  draft block (Finish / Undo vertex / Cancel) copying the zone draw block's shape.
* A BUTTON, not a keybinding. `canvas/commands.rs` and `help_modal.rs` untouched.

### .85 — the duplicate slot-id guard
* `commands_hotkeys.rs`: `save_now` calls `live_duplicate_slot_ids()` **before** `compile_payload`
  and before the POST, and `return`s on a non-empty result — a refusal, not an annotation on a save
  already in flight.
* `duplicate_slot_id_report(dups) -> (headline, rows)` at file scope (NOT inside the wasm-only
  `mod imp`, so the native harness can exercise it). One row per duplicate, each naming callsign and
  id: `Squad 1-1: slot id "s1" is used more than once in this squad`.
* `live_duplicate_slot_ids()` reads the live `MissionDocCore` through
  `operations::duplicate_slot_ids` — the shared operation, not a third private scan.
* Twin divergence documented in code at `commands_hotkeys.rs:1014-1030`.

## perturbation

Four loops. Sever -> RED -> restore -> `touch` -> green. Every run showed exactly one
`Compiling website-frontend` line, so no verdict is a replayed cache (T-596).

### Loop 1 — .82, sever the pointermove borrow
`let z_arm = z_drag.borrow().clone();` -> `let z_arm: Option<(…)> = None;`

```
---- editor::canvas::overlays::t946_86_z_arm::the_z_arm_is_borrowed_by_both_pointer_closures stdout ----

thread 'editor::canvas::overlays::t946_86_z_arm::the_z_arm_is_borrowed_by_both_pointer_closures' (2255280) panicked at apps/website/frontend/src/editor/canvas/overlays.rs:1158:9:
T-946.86 (.82): onpointermove must BORROW the armed z_drag — writing it at pointerdown and never reading it is the wave-255 defect this repairs


failures:
    editor::canvas::overlays::t946_86_z_arm::the_z_arm_is_borrowed_by_both_pointer_closures

test result: FAILED. 1411 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.38s
```

Restore + `touch` -> `test result: ok. 1412 passed; 0 failed` (Compiling: 1).
**This loop failed to go red on its FIRST attempt — see deviations item 1.**

### Loop 2 — .83, sever the multi-drop call site
```
---- editor::panels::outliner_tree::t946_86_multi_drop::the_folder_drop_consumes_the_pending_drag_set stdout ----

thread 'editor::panels::outliner_tree::t946_86_multi_drop::the_folder_drop_consumes_the_pending_drag_set' (2274130) panicked at apps/website/frontend/src/editor/panels/outliner_tree.rs:2461:9:
T-946.86 (.83): the folder-row drop must consume the multi-select DragSet — completing through the single-id latch alone moves only the anchor

---- editor::panels::outliner_tree::t946_86_multi_drop::the_single_id_completion_is_the_fallback_not_a_second_commit stdout ----

thread 'editor::panels::outliner_tree::t946_86_multi_drop::the_single_id_completion_is_the_fallback_not_a_second_commit' (2274161) panicked at apps/website/frontend/src/editor/panels/outliner_tree.rs:2480:14:
checked by the pin above


failures:
    editor::panels::outliner_tree::t946_86_multi_drop::the_folder_drop_consumes_the_pending_drag_set
    editor::panels::outliner_tree::t946_86_multi_drop::the_single_id_completion_is_the_fallback_not_a_second_commit

test result: FAILED. 1410 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.75s
```

### Loop 3 — .84, sever the tactical arm
```
---- editor::panels::zones_panel::t946_86_tactical_trigger::the_panel_arms_the_tactical_draw stdout ----

thread 'editor::panels::zones_panel::t946_86_tactical_trigger::the_panel_arms_the_tactical_draw' (2285217) panicked at apps/website/frontend/src/editor/panels/zones_panel.rs:2081:9:
T-946.86 (.84): a production control must call begin_tactical_draw — the wave-255 state was a complete draw tool with no way to start it


failures:
    editor::panels::zones_panel::t946_86_tactical_trigger::the_panel_arms_the_tactical_draw

test result: FAILED. 1411 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.69s
```

### Loop 4 — .85, sever the duplicate check
```
---- editor::state::commands_hotkeys::t946_86_duplicate_guard::save_now_checks_duplicates_before_it_compiles_or_posts stdout ----

thread 'editor::state::commands_hotkeys::t946_86_duplicate_guard::save_now_checks_duplicates_before_it_compiles_or_posts' (2291726) panicked at apps/website/frontend/src/editor/state/commands_hotkeys.rs:2589:14:
T-946.86 (.85): save_now must ask for the duplicate slot ids


failures:
    editor::state::commands_hotkeys::t946_86_duplicate_guard::save_now_checks_duplicates_before_it_compiles_or_posts

test result: FAILED. 1411 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.14s
```

All four restored + `touch` -> `test result: ok. 1412 passed; 0 failed` (Compiling: 1),
`git status --short` empty.

### Incidental RED (design correction, not a loop)
```
thread 'editor::mission_editor::t796_comment_drag::the_move_commit_partitions_comments_to_their_own_mutator' (2189146) panicked at apps/website/frontend/src/editor/mission_editor_tests/t796_comment_drag.rs:139:10:
T-796: the drag-commit delta guard must survive
```

## acceptance_instrument — production call sites

Classified with each file's `#[cfg(test)]` boundary applied and comment-only lines excluded.

| Symbol | Prod call sites | Where |
|---|---|---|
| `set_z_drag_readout` | 2 | `canvas/gestures.rs:488`, `canvas/gestures.rs:932` |
| `snap_elevation` | 1 | `canvas/overlays.rs:119` |
| `format_height_readout` | 1 | `canvas/gestures.rs:489` |
| `dy_to_elevation` | 1 | `canvas/overlays.rs:115` |
| `z_drag_elevation_delta` | 2 | `canvas/gestures.rs:477`, `canvas/gestures.rs:935` |
| `plan_drop` | 1 | `panels/outliner_drag.rs:111` |
| `DragSet` | 9 | incl. `panels/outliner_tree.rs:1088` |
| `PENDING_DRAG` | 7 | incl. consuming take at `panels/outliner_drag.rs:101` |
| `complete_multi_drop_onto_folder` | 1 | `panels/outliner_tree.rs:1115` |
| `complete_multi_refile_onto_squad` | 1 | `panels/outliner_tree.rs:876` |
| `node_descendant_ids` | 1 | `panels/outliner_tree.rs:1117` |
| `drag_set_for` | 4 | `outliner_tree.rs:756, 1138, 1257, 1272` |
| `outliner_drag::begin_layer_slot_drag` | 1 | `panels/outliner_tree.rs:1277` |
| `outliner_drag::begin_layer_comment_drag` | 1 | `panels/outliner_tree.rs:761` |
| `outliner_drag::begin_refile` | 1 | `panels/outliner_tree.rs:1262` |
| `refile_slot` | 1 (new) | `panels/outliner_drag.rs:177` |
| `begin_tactical_draw` | 1 | `panels/zones_panel.rs:63` |
| `duplicate_slot_ids` | 1 | `state/commands_hotkeys.rs:758` |

Caveat stated rather than hidden: a naive grep reports two "production" hits for
`begin_tactical_draw`; the second (`mission_editor_tests/t936_7_tactical_lane_bind.rs:51`) is a
string inside an assert message, not a call. The only call site is `zones_panel.rs:63`.

Pins added: 19. Suite: 1392 -> 1415.

## gate_verdict_tail

`hcargo xtask mk ci-local-leptos` — all four steps, run twice:
```
cargo fmt -p website-frontend --check                                            (silent — pass)
cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets   (0 errors)
cargo test -p website-frontend
    test result: ok. 1415 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.02s
trunk build --release
    Finished `release` profile [optimized] target(s) in 1m 06s
    INFO applying new distribution
    INFO ✅ success
```

`hcargo xtask platform wave gate --slice T-946.86`, detached, polled on `^SLICE GATE:` only:
```
═══ slice gate T-946.86 ═══
touch_workspace: invalidated 678 workspace .rs file(s) and 134 include_str!/include_bytes! input(s) across 9 member(s)
  cargo check              PASS
  wasm32 (frontend)        PASS
  fmt (changed)            PASS
  clippy (changed crates)  PASS
  test (frontend, changed) PASS
  schema                   PASS
  T-278 catalogue drift    PASS
  db_migrate claim body    PASS
  db_migrate persist       PASS
  T-439 objects aliases    PASS
  T-444 wiki seed          PASS
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ a4481558f10c recorded: .ai/artifacts/verdicts/T-946.86.json
SLICE GATE: PASS
```

## files_outside_owns

**None.** `git diff --stat dc073f7c2..HEAD` lists exactly the six owned files and nothing else.

Sibling files confirmed untouched: `state/operations.rs`, `state/operations/reassign.rs`,
`crates/map-engine-core/src/doc/store.rs` (T-939.2); `panels/{context_menu,top_strip,help_modal}.rs`,
`editor/mission_editor.rs` (T-939.4); `canvas/gizmo_z.rs`; `canvas/commands.rs`. `with_batch` and
`refile_slot` were reached through existing re-exports; `mission_editor.rs`'s `ArcRwSignal` pattern
was read, never edited.

## found_not_fixed

1. **A multi-FOLDER selection is not reachable, so the folder-row drop can only ever carry one id.**
   `mirror_selection` (`state/operations/context.rs:675`) publishes `ctx.selection` — the canvas
   ENTITY selection — into the signal the outliner trees receive, and a folder click runs
   `select_layer_children`, which selects the folder's *slots*. A folder id therefore never enters
   `selected`, so on a folder row `sel.contains(&id_down)` is always false. The acceptance line
   "dropping five selected rows onto a folder moves five" cannot be exercised via folder rows.
   Mitigated, not deferred: the slot and comment rows now arm sets too, and slot ids ARE what that
   mirror publishes. Giving folders their own multi-selection is a new feature, not a wiring fix.

2. **The duplicate-guard twins still disagree; `mission_library.rs` is outside owns.** Verified:
   `editor/library/mission_library.rs:1519` defines `check_duplicate_slot_ids_in_payload`, called
   once at `:1565`. It reads `editor.get("squads").as_array()` and the `slotIds` within, and has
   **no `slot_exists` call at all**, whereas `state/operations/slot_ids.rs:32` gates each id on
   `doc.slot_exists(id_str)`. A payload with a dangling duplicate id is refused on upload and passes
   the new doc-side guard. They also read different shapes (`squads[]` array vs `squadsById`
   object). Documented in code at `commands_hotkeys.rs:1014-1030`.

3. **Four pieces of prose now falsely say `begin_tactical_draw` has no caller.** All outside owns:
   `state/operations.rs:83` (T-939.2's file this wave), `state/history.rs:386`,
   `state/operations/tactical_graphics.rs:172-186` (its own doc block, which names the remedy this
   slice performed), `mission_editor_tests/t936_7_tactical_lane_bind.rs:9,51`. None load-bearing —
   the suite is green — but each is stale and would mislead the next reader.

4. **`gestures.rs` cannot host a test.** `canvas/mod.rs` declares `pub mod gestures` under
   `#[cfg(target_arch = "wasm32")]`, so any `#[cfg(test)]` inside it is never compiled by
   `cargo test -p website-frontend`. Same for `canvas/commands.rs`. Any future slice told to "add a
   pin at the bottom of gestures.rs" will write a test that silently never runs. Worked around here;
   the underlying trap belongs in the brief template.

5. **Pre-existing `unused variable: nodes`** in `folder_row_actions` (`outliner_tree.rs`, was `:514`,
   now `:559`). Present in the baseline `ci-local-leptos` log before any edit; not introduced.

## deviations

1. **The `.82` pins were relocated after the first perturbation loop refused to go red — the
   headline finding.** Seven pins were first written at the bottom of `gestures.rs`, per the brief's
   "tests go at the BOTTOM of each file". They compiled nowhere: `gestures` is wasm-gated. Severing
   the pointermove borrow left the suite **green at 1412**, and the suite ran exactly 1405 tests
   both with and without those seven pins. Without the loop, this slice would have shipped seven
   tests that never execute — the identical defect it was filed to repair, one level down, in the
   repair itself. `z_drag_elevation_delta`, `z_drag_snap_step` and all seven pins therefore live in
   `canvas/overlays.rs`: ungated, in owns, same directory (so `include_str!("gestures.rs")` still
   scrubs the live gesture source), and already the owner of the Z readout they feed. Commit
   `c5bce8f01`.
2. **Scope widened within owns** to the three other dead `outliner_drag` arms and a second consumer
   (`complete_multi_refile_onto_squad`). The .83 finding names those three; the acceptance
   instrument covers every function the findings name; and without them the .83 fix would be
   structurally correct and never exercised (found_not_fixed 1). No file outside owns touched.
3. **The Z commit uses `core.move_entities`, not `move_entities_and_vehicles`** — forced by
   `t796_comment_drag`'s pin (RED above) and independently correct (vehicles have no z column).
4. **The brief was read from `main`** — it did not exist at the worktree's base commit. No merge.
5. **No manual browser verification.** The four acceptance items need a live editor and a pointer;
   they are listed under manual_checklist rather than claimed. `:3000` (`trunk serve --release`,
   pid 1264262) and the API were left running and untouched.

## commits

| SHA | Subject |
|---|---|
| `77066eed6` | T-946.86: wire the four wave-255 dead features into production |
| `8fd235f18` | T-946.86: Class-R pins for all four new call sites |
| `c5bce8f01` | T-946.86: move the .82 helper + pins out of the wasm-only gesture file |
| `a4481558f` | T-946.86 (.83): the other three dead drag arms, and the lane where multi actually works |

Nothing pushed, nothing merged, no ship. Working tree clean at `a4481558f`.

## manual_checklist

1. **Z gizmo.** Select a slot, Translate widget variant, drag the vertical arm. Expect: the height
   chip updates live; releasing writes the elevation; ONE Ctrl+Z reverts the whole drag; later
   clicks behave normally (capture released). Press `G` and step the translate rung — the drag
   should quantise to 1/5/10 m.
2. **Multi drop — slot rows (the reachable lane).** Marquee five slots on the canvas, drag one of
   the five in the Editor-Layers tree onto a different folder. Expect five to move and one Ctrl+Z to
   restore all five. Repeat in the ORBAT tree dropping onto a squad.
3. **Multi drop — the negative.** Press an UNSELECTED row while others are selected and drag it.
   Expect that row alone to move.
4. **Tactical draw.** Zones panel -> Tactical graphics -> `phase_line` -> Draw -> click two points
   -> Finish (or right-click). Expect the graphic to appear, pick and delete; Esc mid-draw abandons;
   Undo vertex drops the last point.
5. **Duplicate slot ids.** Author two slots sharing an id in one squad, then Save. Expect the save
   to be REFUSED before any request, headline naming the count and a findings line naming the
   callsign and the id. Confirm via devtools that no POST to `/missions/:id/versions` is made.
6. **Non-regression.** Drag a folder onto the header root dropzone — `complete_layer_drop_onto_root`
   reads the single-id latch, which is still armed beside the set.
