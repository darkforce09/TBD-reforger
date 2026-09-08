# REPORT — T-939.2 · Attributes: batch faction and squad reassign

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-939.2
slice/T-939.2
```

Branch point `dc073f7c2`. (`main` has since moved to `1725e53e7`; the five wave-256 brief `.md`s
and `docs/platform/FACTORY_RUN_2026-09.md` show in `git diff main --name-only` only because main
*added* them after my branch point — they are not slice changes. My brief was read from the main
checkout, since it is not on this branch.)

## defect_verified

The Identity tab rendered the squad as an inert read-only `div` (`attributes_modal.rs:1864-1872`
building the string, `:1924-1929` painting it) and offered no faction control at all — every
faction hit in the file belongs to `type_picker`, which edits `assetId`. Opened and read, not
string-matched.

A pin scoped to `identity_tab`'s own body (so `type_picker`'s faction mentions cannot green it),
run on the unmodified tree:

```
running 1 test
test editor::panels::attributes_modal::t939_2_batch_reassign::the_identity_tab_offers_a_faction_control_and_an_editable_squad_control ... FAILED
thread '...the_identity_tab_offers_a_faction_control_and_an_editable_squad_control' (2115912) panicked at apps/website/frontend/src/editor/panels/attributes_modal.rs:3511:9:
T-939.2: the Identity tab must render the faction/squad reassign controls; body was:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1392 filtered out; finished in 0.19s
```

Committed as the defect record in `7314afcc2` before any production code was written.

## changes

Four files, all in owns. No file outside owns was touched.

### `crates/map-engine-core/src/doc/store.rs` (+ additive core primitive, + 4 native tests)

* `move_slot_to_squad` and the new **`move_slot_to_squad_keep_source`** now both delegate to one
  private `move_slot_between_squads(slot_id, dest, keep_source)`. The two paths differ in exactly
  one branch — the emptied-source arm — so the `slotIds` splice, the dense `index` rewrite and the
  dest-side leader invariant cannot drift into two definitions of a move.
* `keep_source` skips `garbage_collect_squad_in_txn` and instead removes only the now-dangling
  `leaderSlotId` (the leader left with the last slot; an empty squad with no leader key is exactly
  what `add_squad` mints). The row, its place in `faction.squadIds` and its attached vehicles all
  survive.
* **The existing default is unchanged** and a test proves it
  (`the_default_move_slot_to_squad_still_garbage_collects_an_emptied_source`), because the Outliner
  drag-refile depends on the GC.
* Doc-fixture tests at the bottom of the file: emptied squad survives with vehicles + position;
  the default still GCs; the derived side key follows a cross-faction move with the `SideKeyMemo`
  warmed under the old sides first; the shared body still promotes the next leader and keeps
  indices dense.

### `apps/website/frontend/src/editor/state/operations/reassign.rs` (new)

`reassign_slots(ids, &ReassignTarget) -> Result<usize, String>` — moves every id into one
destination inside **one** `with_batch("reassign-slots", …)` group, through
`move_slot_to_squad_keep_source`. The destination is resolved *before* the group opens, so a
refusal never leaves an empty undo group behind. `reassign_rows()` exposes the live faction/squad
rows in one doc read. One `materialize()` per batch, not per id.

### `apps/website/frontend/src/editor/panels/attributes_modal.rs`

* **`plan_reassign` / `faction_label`** — pure, native, *outside* the wasm block (the
  `axis_chip_class` / `nudge_step` precedent). This is where the refusal sentences live, so
  `cargo test` calls the real function and reads the real copy rather than scraping source for a
  string nobody proved the modal can produce.
* **`reassign_picker`** replaces the inert div: a Faction `<select>` and a Squad `<select>`, both
  committing over `targets` (the `attrs_multi_ids` set). Picking a faction commits immediately to
  that faction's first squad — the acceptance's "choosing another faction moves all five". Neither
  control keeps a local pick signal: both read out of the document and the commit's `doc_tick` bump
  re-renders them (the T-649 `MultiOpts` lesson — a latch minted inside a render closure un-ticks
  itself the instant its own commit lands). A mixed selection reads `Mixed`, never slot[0]'s squad.
  The refusal renders in-modal with `role="alert"`.
* The selection's current squads are inverted once out of `SquadRow::slot_ids` rather than asked
  per id via `read_attrs` (which materializes the whole SoA per call — 50 full scans per render on
  a 50-slot selection).

### `apps/website/frontend/src/editor/state/operations.rs`

`pub mod reassign;` + `pub use reassign::*;` alongside the existing registrations. (rustfmt sorted
the glob re-export into alphabetical position after the `entity` block.) Two hunks, nothing else.

### On the two ticket corrections

* **Side keys are derived** — confirmed against `resolve_slot_side_key` (`store.rs:6941-6966`) and
  `SideKeyMemo` (`:98-147`). Nothing writes a side key, so there was nothing to skip; rewriting
  `squadId` *is* the side change. The memo is keyed by **squad** id and cleared by the
  `observe_after_transaction` version bump every committed transaction fires, so a moved slot
  self-heals on the next `materialize()`. `keep_source_move_carries_the_derived_side_key_across_factions`
  warms the memo under the old sides *before* the move so a cold memo cannot pass it by accident.
* **`store.rs` in owns** — used exactly as directed: one additive entry point, one shared private
  body, default untouched. Total core production diff is ~30 lines, kept small for T-932.

## perturbation

Two perturbations, because the requirement has two halves — the core primitive and the batch's
choice of path.

### A — route the keep-source path through the GC-ing move (the brief's step 3)

`move_slot_to_squad_keep_source` delegated with `false` instead of `true`. RED verbatim:

```
running 1 test
test doc::store::tests::move_slot_to_squad_keep_source_keeps_the_emptied_squad_its_vehicles_and_its_position ... FAILED

failures:

---- doc::store::tests::move_slot_to_squad_keep_source_keeps_the_emptied_squad_its_vehicles_and_its_position stdout ----

thread 'doc::store::tests::move_slot_to_squad_keep_source_keeps_the_emptied_squad_its_vehicles_and_its_position' (2222833) panicked at crates/map-engine-core/src/doc/store.rs:15584:9:
T-939.2: the keep-source move lost the squad row, the attached vehicle v1, the squad->vehicle attachment, its place in faction.squadIds; squads were {"sq-opf":{"slotIds":[],"id":"sq-opf","factionId":"faction-OPFOR","name":"Krasnyi","vehicleIds":[]},"sq-a":{"id":"sq-a","vehicleIds":[],"leaderSlotId":"solo","name":"Alpha","slotIds":["solo"],"factionId":"faction-BLUFOR"},"sq-c":{"factionId":"faction-BLUFOR","name":"Charlie","vehicleIds":[],"id":"sq-c","slotIds":[]}}, vehicles were {}
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    doc::store::tests::move_slot_to_squad_keep_source_keeps_the_emptied_squad_its_vehicles_and_its_position

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1102 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p map-engine-core --lib`
```

**The first run of this perturbation did not name the vehicles.** A chain of `assert!`s
short-circuited on the squad row and reported only that, so a perturbation that also deleted the
vehicles read as one that only deleted the row. The test now collects the losses and names all
four — the RED above is that version, and the fix is committed (`767a26bbf`). This is exactly why
the brief asks for the RED verbatim: the first one hid two-thirds of the damage.

### B — route the batch itself through the GC-ing call

`reassign.rs` changed to call `core.move_slot_to_squad(id, &dest)`. RED verbatim:

```
running 1 test
test editor::panels::attributes_modal::t939_2_batch_reassign::the_batch_uses_the_keep_source_core_path_not_the_garbage_collecting_one ... FAILED

failures:

---- editor::panels::attributes_modal::t939_2_batch_reassign::the_batch_uses_the_keep_source_core_path_not_the_garbage_collecting_one stdout ----

thread 'editor::panels::attributes_modal::t939_2_batch_reassign::the_batch_uses_the_keep_source_core_path_not_the_garbage_collecting_one' (2224486) panicked at apps/website/frontend/src/editor/panels/attributes_modal.rs:4001:9:
T-939.2: the batch must move through move_slot_to_squad_keep_source
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    editor::panels::attributes_modal::t939_2_batch_reassign::the_batch_uses_the_keep_source_core_path_not_the_garbage_collecting_one

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1402 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p website-frontend --bin website-frontend`
```

### restore + touch + green

Both restored, all three files `touch`ed, no `PERTURBATION` marker left in either crate:

```
running 173 tests   (doc::store::tests)
test result: ok. 173 passed; 0 failed; 0 ignored; 0 measured; 930 filtered out

running 1403 tests  (website-frontend)
test result: ok. 1403 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## gate_verdict_tail

`cargo xtask mk ci-local-leptos` — all four steps ran and passed (`cargo fmt --check`,
`clippy --target wasm32-unknown-unknown --all-targets`, `cargo test -p website-frontend`
1403 passed, `trunk build --release` ending `✅ success`).

`hcargo xtask platform wave gate --slice T-939.2`, detached, polled on `^SLICE GATE:`:

```
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

  gate verdict PASS @ e9d9d9aa89bb recorded: .ai/artifacts/verdicts/T-939.2.json
SLICE GATE: PASS
```

`e9d9d9aa89bb` is the final HEAD. (The run waited ~120s on the gate lock held by slice T-212 —
serialisation, reported as such by the driver.)

## files_outside_owns

`[]` — final `git diff main --stat` over source is exactly the four owned files.

One near-miss worth recording: `hrustfmt --edition 2024` run on `operations.rs` **recursively
reformatted every submodule it declares** (`attrs.rs`, `context.rs`, `entity.rs`,
`tactical_graphics.rs`) — and in the wrong style edition, since `website-frontend` is edition
**2021** while `map-engine-core` is 2024. Caught on `git status` before those files reached a
commit; the four were reverted with explicit paths and the three owned files re-formatted with the
gate's own `cargo fmt -p website-frontend`. `entity.rs` and `tactical_graphics.rs` belong to
sibling slices this wave, so this would have been a cross-slice collision. **Do not run `hrustfmt`
on a module file — run `hcargo fmt -p <pkg>`.**

## found_not_fixed

* **`map-engine-core` has one pre-existing red in this worktree**, unrelated to the slice:
  `dem::peaks::tests::everon_peaks_max_above_350` panics `decode: Decode("Invalid PNG signature.")`
  because `packages/map-assets/everon/dem/everon-dem-16bit.png` is an unfetched 133-byte LFS
  pointer (`oid sha256:585e1432ddf24dfb963f81510b4b570a41c68ec8ea85f56e755c3c5f95f4517b`,
  `size 719115`). All 1100 other `map-engine-core` tests pass, including all 173 in
  `doc::store::tests`. This is the known worktree-LFS condition the brief itself flags.
* The `unused import` warnings on `operations.rs`'s hand-listed `pub use entity::{…}` block are
  pre-existing and are the exact cost T-936.7 documented when it chose a glob for
  `tactical_graphics`. `pub use reassign::*;` is a glob for the same reason. Not widened.
* `SlotAttrs.squad` is a raw **squad id**, not a name — which is what the old read-only field was
  showing operators. The picker now shows names, but `SlotAttrs.squad` is still an id and lives in
  `attrs.rs` (outside owns), so it was left alone.

## deviations

1. **Where the pure decision lives.** `plan_reassign` / `faction_label` sit in
   `attributes_modal.rs`, not in `reassign.rs`. Reason: `operations.rs` carries a file-level
   `#![cfg(target_arch = "wasm32")]`, which gates its whole submodule tree, so **anything in
   `reassign.rs` is invisible to `cargo test`** (verified — `slot_ids.rs`'s test module never runs
   natively). Putting the refusal strings there would have left requirement 4 provable only by
   source-scraping. `attributes_modal.rs` compiles natively and already hosts pure helpers outside
   the wasm block for exactly this purpose (`axis_chip_class`, `nudge_step`,
   `attrs_multi_subtitle`). `reassign_slots` itself is in `reassign.rs` and registered in
   `operations.rs` as specified.
2. **Doc-fixture behaviour tests are in `store.rs`, not `reassign.rs`** — same cause, and it is
   also where the primitive they exercise lives and where the file's own comment says behaviour
   belongs.
3. The emptied-squad test was restructured mid-perturbation to collect losses rather than
   short-circuit (see §perturbation). This strengthened the pin; it did not weaken it.
4. Nothing was deferred.

## commits

| sha | subject |
|-----|---------|
| `7314afcc2` | RED — the Attributes Identity tab offers no faction or squad control |
| `475d6e79c` | additive keep-source move in the doc core |
| `39fc33219` | batch faction and squad reassign in the Attributes modal |
| `8f9a58b47` | restore edition-2021 rustfmt on the frontend files |
| `767a26bbf` | the emptied-squad test reports every loss, not just the first |
| `e9d9d9aa8` | read the selection's squads off the rows in hand, not read_attrs |

No push, no merge, no ship. Branch committed and clean.

## manual_checklist

Automated coverage stops at the `view!` boundary — `cargo test` cannot instantiate a Leptos tree
over `web_sys` nodes, so these need a browser. `:3000` and `:8080` were left up.

1. Open the editor, place five slots under BLUFOR, select all five, open Attributes → Identity.
   The **Faction** select shows the common faction; the **Squad** select lists that faction's
   squads with slot counts.
2. Choose another faction. All five move; the Outliner shows them under the new faction, in that
   faction's first squad. **One Ctrl+Z restores every one of them** (the unit tests pin the
   `with_batch` bracket, not the browser's undo).
3. Empty a squad that has a vehicle attached by moving its last slot out. The squad stays in the
   Outliner, in its original position under the faction, **with its vehicle badge intact**.
4. Select slots from two different squads. Both selects read `Mixed …`, not the first slot's
   squad. Picking a faction moves the whole mixed selection.
5. Pick a faction with no squads. The modal shows the named refusal
   ("… has no squads yet — add one in the ORBAT dock, then reassign.") and nothing moves.
6. The cross-faction refusal (requirement 4) is a race guard — the squad list only ever offers the
   chosen faction's squads — so reproducing it by hand needs a second peer or an undo landing
   between render and commit. `plan_reassign` covers it natively; flagged here as the one
   requirement whose refusal is deliberately hard to reach through the UI.
