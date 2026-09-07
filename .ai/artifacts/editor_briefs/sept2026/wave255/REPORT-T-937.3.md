# REPORT — T-937.3 (Materialize side-key cache and slot_exists fast path)

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-937.3
slice/T-937.3
```

Branch point `c73d39668`. Tree clean at report time.

## defect_verified

Both defects were pinned RED **before** any fix, in commit `dbf374c0c`, which adds only the
measurement (`side_key_resolutions` counter) and the probes. Verbatim from
`hcargo xtask platform wave test --slice T-937.3 -p map-engine-core --all-features`:

```
---- doc::store::tests::materialize_resolves_each_distinct_side_once_over_500_slots stdout ----

thread 'doc::store::tests::materialize_resolves_each_distinct_side_once_over_500_slots' (571236) panicked at crates/map-engine-core/src/doc/store.rs:14971:9:
T-937.3: 500 slots over 2 distinct sides took 500 side-key resolutions; materialize must resolve each distinct side once per call

---- doc::store::tests::slot_attrs_exists_reads_the_raw_map_not_the_materialized_view stdout ----

thread 'doc::store::tests::slot_attrs_exists_reads_the_raw_map_not_the_materialized_view' (571281) panicked at crates/map-engine-core/src/doc/store.rs:15012:9:
T-937.3: slot_attrs_exists must answer existence off the raw slot map (`MissionDocCore::slot_exists`), so a hidden slot counts as existing; body:{
    core.materialize().ids.iter().any(|s| s == id)
}
```

**Resolution count before:** 500 walks of `resolve_slot_side_key` for 500 slots over 2 distinct
sides — one per row, memoizing nothing. **After:** 2.

A third test, `materialize_drops_hidden_slots_the_document_still_holds`, is green on both sides and
characterises *why* the second probe matters: a slot on a hidden layer (T-665) and a slot carrying
its own `editorHidden` flag (T-701) are both absent from `materialize().ids` while `slots_json`
still carries them verbatim. The concrete user-visible consequence of the old `slot_attrs_exists`:
place a composition while the active layer is hidden and every id `place_composition` wrote is
filtered out of the post-place selection, so the stamp appears to have done nothing.

### Why the entity.rs half is a source probe, not a behavioural test

`apps/website/frontend/src/editor/state/operations/entity.rs` is reached only through
`state/operations.rs`, which carries `#![cfg(target_arch = "wasm32")]` on its whole subtree, and
`website-frontend` links `map-engine-core`'s `doc` feature **only** in its
`cfg(target_arch = "wasm32")` dependency table. There is no native build of that file to call into,
which is why every existing pin on it in this repo is an `include_str!` probe. Mine lives in
`store.rs` (one of my two owns) following the T-491/T-574 precedent already in that same test
module, and it is read over `strip_rust_lexical_noise` and scoped to the brace-matched body, so
neither the doc comment describing the rule nor a `materialize(` elsewhere in the 4.9k-line file can
decide it. The gate's own `wasm32 (frontend)` step is what proves the edited body compiles.

## changes

**`crates/map-engine-core/src/doc/store.rs`**

1. `struct SideKeyMemo` (new, production, above `MissionDocCore`) — `squad id -> side key`, a
   `RefCell<(version, HashMap)>` plus an `Arc<AtomicU64>` version bumped by a
   `Doc::observe_after_transaction` subscription the memo owns.
2. `MissionDocCore` gains `side_key_memo: Option<SideKeyMemo>` and `side_key_resolutions: Cell<u64>`.
   `from_doc_with_clock` installs the subscription **first**, before any `get_or_insert_map` opens a
   transaction (`observe_*` needs exclusive store access).
3. `materialize()` reads through the memo. **Signature untouched** — still
   `pub fn materialize(&self) -> SlotSoa`, interior mutability only, per the wave pin on
   `persist.rs::slots_digest` (T-190).
4. `pub fn slot_exists(&self, id: &str) -> bool` — one `contains_key` on the raw `slots` root map
   under a read txn. Hidden slots count as existing.
5. `pub fn side_key_resolution_count(&self) -> u64` — the memo's honesty check.
6. Tests (all inside the existing `mod tests`, at EOF, below every production item).

**`apps/website/frontend/src/editor/state/operations/entity.rs`**

7. `slot_attrs_exists` -> `core.slot_exists(id)` (was `core.materialize().ids.iter().any(...)`).
8. The two warning comments at `live_slot_ids` and `mint_ids` now name `slot_attrs_exists` as the
   single-id form of the same rule, so both readers are documented as seeing one universe.

### Invalidation — the design decision the brief asked me to justify

There is **no change observer anywhere in the repo** (confirmed: zero hits for `.observe(`,
`observe_deep`, `observe_update`, `yrs::Subscription` before this commit). The brief's two options
were "add `RefCell`/`Cell` state plus explicit invalidation" or "introduce the observer yourself".

**I introduced the observer.** Explicit invalidation cannot be made safe here:

* the mutators that can move a side are not a closed set a reader can eyeball — `add_squad`,
  `move_slot_to_squad`, `add_faction` (which *overwrites*, so it is also the side-rename path),
  `place_character_under_side`'s inline faction mint, `hydrate`, the remint paths,
  `apply_faction_library`; and
* three whole classes of write never touch a mutator at all: **undo/redo**, **`apply_update`** (a
  peer's bytes or an IndexedDB restore), and **hydrate**. A hand-placed `invalidate()` cannot cover
  those, and a memo that misses one is silently and permanently stale — a row rendering under the
  wrong side's colour, forever. Two of those three are pinned by tests below.

`observe_after_transaction`, **not** `observe_update_v1`: v1 encodes the entire update to lib0
bytes before calling back (`store.rs:585` in yrs 0.27.2), i.e. a full encode on every drag frame to
compute a number we discard. `after_transaction` (`store.rs:604`) fires unconditionally on every
committed `TransactionMut` and costs one closure call. The callback does one relaxed `fetch_add`
and *deliberately nothing else* — a callback that touched the `RefCell` could re-enter it mid-borrow
while `materialize` holds it across its loop.

It over-invalidates (a `transact_mut` that changed nothing bumps too). That is the correct direction
to be wrong in: the per-call memo still holds inside `materialize`, which is where the acceptance
bound lives.

Fail-safe: if the subscription cannot be installed, `side_key_memo` is `None` and `materialize`
falls back to a per-call scratch map — the cross-call memo is **disabled**, never left
uninvalidated. A cache with no invalidation signal is a staleness bug wearing a speedup's clothes.

## perturbation

Perturbation applied to the invalidation itself — `SideKeyMemo::entries`, `if e.0 != live` ->
`if false && e.0 != live` — then `touch crates/map-engine-core/src/doc/store.rs`. Three tests went
RED, verbatim:

```
---- doc::store::tests::side_key_memo_sees_a_faction_minted_after_the_first_materialize stdout ----

thread 'doc::store::tests::side_key_memo_sees_a_faction_minted_after_the_first_materialize' (608399) panicked at crates/map-engine-core/src/doc/store.rs:15287:9:
assertion `left == right` failed: T-937.3: the side-key memo served a stale side after the faction was minted — the memo is not being invalidated by the document's own change signal
  left: ["BLUFOR"]
 right: ["OPFOR"]

---- doc::store::tests::side_key_memo_sees_a_remote_update stdout ----

thread 'doc::store::tests::side_key_memo_sees_a_remote_update' (608400) panicked at crates/map-engine-core/src/doc/store.rs:15356:9:
assertion `left == right` failed: T-937.3: a remote update moved the side and the memo did not notice
  left: ["BLUFOR"]
 right: ["OPFOR"]

---- doc::store::tests::side_key_memo_sees_an_undo stdout ----

thread 'doc::store::tests::side_key_memo_sees_an_undo' (608401) panicked at crates/map-engine-core/src/doc/store.rs:15326:9:
assertion `left == right` failed: T-937.3: Ctrl+Z removed the faction and the memo kept serving its side key
  left: ["OPFOR"]
 right: ["BLUFOR"]
```

```
test result: FAILED. 1065 passed; 4 failed; 2 ignored; 0 measured; 0 filtered out; finished in 4.06s
```

(The fourth is the pre-existing LFS red — see *found_not_fixed*.)

Restored by hand-editing the line back, `git diff` verified **empty** against the committed fix,
then `touch`ed the file (a restore alone does not reliably re-trigger cargo — the T-244/T-420
class). Re-run rebuilt from source and returned to green:

```
   Compiling map-engine-core v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-937.3/crates/map-engine-core)
     Running unittests src/lib.rs (/var/home/Samuel/.cache/tbd-target-T-937.3/debug/deps/map_engine_core-cc72c982ac1eb5cf)
```

The `Compiling` line names **this worktree's** path and the run used the **per-slice private**
target dir `tbd-target-T-937.3` — not a replayed cached verdict from a sibling (T-596).

Two tests deliberately stay **green** under this perturbation, and that is the design, not a gap:

* `side_key_memo_survives_a_slot_moving_sides` — the memo is keyed by squad, so a re-parented slot
  looks its side up under a different key and self-heals with no invalidation at all;
* `a_warm_memo_materializes_exactly_what_a_cold_one_does` — no document change happens between its
  materialize calls, so there is nothing to invalidate.

## gate_verdict_tail

`hcargo xtask platform wave gate --slice T-937.3`, run detached from the worktree. Two independent
runs, both PASS at HEAD `c971780365eb`:

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

  gate verdict PASS @ c971780365eb recorded: .ai/artifacts/verdicts/T-937.3.json
SLICE GATE: PASS
```

Recorded verdict at final HEAD: `{"sha": "2d87edf9b51e46cbaa42c0143aa1bf405c14c91a", "verdict": "PASS", "at": "2026-09-07T16:37:48Z"}` (re-gated after the report commit; an earlier identical PASS was recorded at `c971780365eb`, the last code commit).

Direct suite runs, tails pasted:

```
# hcargo xtask platform wave test --slice T-937.3 -p map-engine-core --all-features
test result: FAILED. 1068 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 4.00s
#   the 1 = dem::peaks::everon_peaks_max_above_350, an unfetched LFS pointer (see found_not_fixed)

# hcargo xtask platform wave test --slice T-937.3 -p website-frontend
test result: ok. 1344 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.33s
```

`--all-features` was used throughout; the crate's own `feature_gate_tripwire` refuses a bare
`cargo test -p map-engine-core` and caught my first attempt (171 tests vs 1069). No test printed
`skip:`.

## files_outside_owns

`[]` — exactly the two owned files:

```
apps/website/frontend/src/editor/state/operations/entity.rs |  26 +-
crates/map-engine-core/src/doc/store.rs                     | 649 ++++++++++++++++++++-
```

No sibling file touched: not `persist.rs` / `mission_editor.rs` / `canvas/render_sync.rs` (T-190),
not `state/history.rs` (T-936.7 — `after_local_edit`'s signature did not move under me), not
`soa.rs`, not `doc/mod.rs`, not `docs/`, not `.ai/tickets/`.

Both owned files are already on the SIZE-3 allowlist (`.coding-standards-allowlist.yaml:257` and
`:353`, expiring 2026-11-13), so nothing was added to that allowlist. New production code would
normally go in a new file per the brief's rule 7, but a new `doc/*.rs` module requires a `mod`
declaration in `crates/map-engine-core/src/doc/mod.rs`, which is **not in my owns** — see
*deviations*.

## found_not_fixed

1. **`dem::peaks::tests::everon_peaks_max_above_350` fails in this worktree** —
   `decode: Decode("Invalid PNG signature.")`. Cause verified: `packages/map-assets/everon/dem/
   everon-dem-16bit.png` is an unfetched **git-lfs pointer file** here (`version https://git-lfs...`,
   `size 719115`). Environmental, present before my first edit, unrelated to this slice, and the
   known-red class the platform notes already cover. The slice gate is green regardless.
2. **`resolve_slot_side_key`'s second caller, `squad_link_inputs` (`store.rs:~966`), is already
   optimal** — it iterates *squads*, not slots, so it resolves once per squad by construction. No
   memo needed and none added; noted so a later reader does not "fix" it. It deliberately does
   **not** bump `side_key_resolutions`, which is documented as materialize-scoped — that is what
   lets `slot_exists_answers_true_for_slots_materialize_drops` use the counter as a
   "did-not-materialize" assertion.
3. **A latent flake class in this test module, one instance found and fixed (mine).** My first
   version of `side_key_memo_survives_a_slot_moving_sides` asserted `side_keys == ["BLUFOR",
   "OPFOR"]`, i.e. it asserted `yrs` map *iteration order*. It passed, then failed a later run with
   `left: ["OPFOR", "BLUFOR"]` — caught only because I re-ran. Fixed in `c97178036` by reading the
   side back through the module's existing `row_of` helper; six consecutive clean runs of the 165
   `doc::store` tests plus three full-suite runs since. I swept the rest of `store.rs` for the same
   shape: the only other positional read is `soa.ids[0]` at `:13085`, which is guarded by an
   `ids.len() == 1` assertion immediately above it and is therefore safe. No action needed
   elsewhere.
4. **A `yrs` map-merge property worth knowing, not a bug.** My first draft tested undo *and* a
   remote update on the same key in one test: undo the local `faction-OPFOR` mint, then apply a
   peer's concurrent `faction-OPFOR` mint. The key stayed resolved-to-nothing (`BLUFOR` fallback).
   That is CRDT map conflict resolution — the locally-deleted item can still win the map-entry
   pointer against a concurrent remote insert of the same key — not memo staleness. Split into two
   independent tests, which is the better shape anyway. Flagged because a future collab slice
   (T-295) may meet it as "undo + peer edit loses the peer's write".

## deviations

1. **The memo is keyed by SQUAD id, not by slot id.** The ticket says "a side-key cache keyed by
   slot id". A slot's side is a pure function of its `squadId`, and `squadId` is read fresh from the
   document on every row of every call, so keying by squad is strictly better in every dimension:
   it is `O(squads)` instead of `O(slots)` in memory, and it removes an entire invalidation trigger
   — a slot re-parented to another squad self-heals with no invalidation at all, where a slot-keyed
   cache would need a third "and also when a slot changes squad" rule that someone has to remember.
   Both acceptance clauses are met unchanged, and `side_key_memo_survives_a_slot_moving_sides`
   pins the re-parent case explicitly.
2. **New production code went into `store.rs` rather than a new file** (brief rule 7). A new
   `crates/map-engine-core/src/doc/side_key_memo.rs` needs a `mod` line in `doc/mod.rs`, which is
   outside my owns — the brief says to stop and report rather than edit such a file, so I did not.
   No allowlist was extended: `store.rs` and `entity.rs` both already carry dated SIZE-3 entries.
   If the intent was a separate module, it needs `doc/mod.rs` added to a future slice's owns.
3. **`cargo xtask mk ci-local-leptos` (in the ticket's `verify` list) was not run** — brief rule 6
   forbids it explicitly ("No `ci-local`, no `leptos-gates`"). Its substance is covered by the slice
   gate's `fmt`, `clippy (changed crates)`, `wasm32 (frontend)` and `test (frontend, changed)`
   steps, all PASS. This is a brief-directed substitution, not a self-authored deferral.
4. **`side_key_resolution_count()` is public and unconditional, not `#[cfg(test)]`.** A test-only
   counter exists only in the configuration that never ships and is invisible to any probe reading
   the shipped build. Cost is one `Cell` increment per cache *miss*.
5. **`slot_attrs_exists` was kept as a named wrapper** rather than inlining `core.slot_exists(id)`
   at its single call site: it keeps the documented rule attached to a named function, and it is the
   anchor the source probe (and the two updated warning comments) point at.

Nothing in the ticket's scope was deferred. No follow-up ticket was invented.

## commits

| sha | subject |
|---|---|
| `dbf374c0c` | `T-937.3: RED — instrument the side-key walk and pin both defects` |
| `1ded2b150` | `T-937.3: side-key memo behind a doc observer, and slot_exists` |
| `c97178036` | `T-937.3: read the side back by id, not by row position` |

All on `slice/T-937.3`, off `c73d39668`. Every `git add` named explicit paths; no `git add -A`, no
`git add <dir>`, no `git stash`. Not shipped, not merged, not pushed. Branch committed and clean.

## manual_checklist

None. The spec's MANUAL section for this slice reads "None", and nothing here needs a browser, a
Workbench, a database or an operator: the whole acceptance is covered by native tests that the
slice gate runs. `:3000` and `:8080` were left alone (never started, never stopped).
