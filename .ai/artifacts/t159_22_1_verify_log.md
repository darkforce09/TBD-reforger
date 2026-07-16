# T-159.22.1 — Undo granularity: verify log

**Slice:** T-159.22.1 · **Spec:** [`docs/platform/t159_22_1_undo_granularity.md`](../../docs/platform/t159_22_1_undo_granularity.md)
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159` · **Baseline:** `351c668b` (post T-159.22 merge; code baseline `0154b4e9`)

## Headline

**There was no undo-granularity defect in the product.** The invariant T-159.22 reported broken —
one LOCAL transaction = one undo step — holds, and always held: on native debug, native release, and
wasm. The reported symptom was an artifact of the **gate driver**: `smoke_undo_editor.mjs`'s
`keyChord()` dispatched `rawKeyDown` **and** `keyDown` for one chord, which Chrome delivers as **two
`keydown` events**, so every "Ctrl+Z" the gate pressed was really **two Ctrl+Z presses**.

`map-engine-core` is **unchanged apart from tests + one read-only accessor**. The fix is one line in
the driver.

## Root cause

### What was claimed

`store.rs:75-78` and `mission_history.rs:5` state that `capture_timeout_millis: 0` + `ZeroClock` make
every transaction its own undo step. T-159.22 observed two drags collapsing into one undo step and
concluded the core violated its own documented invariant, noting the mechanism was "unresolved vs
yrs 0.27.2 static reading".

### What is actually true

The static reading was right and the observation was mismeasured. Evidence, in the order it landed:

**1. Capture is granular — the docs are correct.** yrs `undo.rs:264-270`:

```rust
let extend = !undoing && !redoing && same_doc
    && inner.last_change > 0
    && now - inner.last_change < inner.options.capture_timeout_millis;
```

Two independent guards each force `extend = false` here: `u64 < 0` is never true
(`capture_timeout_millis = 0`), and `ZeroClock` pins `last_change` to `0` so `last_change > 0` is
never true either. Every captured transaction pushes a fresh `StackItem`. T-159.22's reading of this
code was correct; its conclusion that observation disagreed was the error.

**2. New in-crate tests were GREEN on baseline — no red phase.** `two_local_moves_are_two_undo_steps`
and `two_local_places_are_two_undo_steps` (mirroring the browser sequence: INIT `seed_random(8)` →
two LOCAL gestures → undo) passed on the untouched baseline, in **debug and release**:

```
test doc::store::tests::two_local_moves_are_two_undo_steps ... ok
test doc::store::tests::two_local_places_are_two_undo_steps ... ok
```

Spec U2 asked for a red-first test. It could not go red, because the core was not broken. Reported as
found rather than manufactured.

**3. Replaying the host's per-gesture tail natively also passed.** A throwaway probe interleaving
`materialize` / `small_maps_json` / `encode_state` / `slot_count` / `can_undo` (everything
`after_doc_change` does) between the two moves: `depth after move1 = 1`, `depth after move2 = 2`,
`undo1 == d1? true | == d0? false`. Not the host tail.

**4. A pure-doc probe inside wasm passed too** — same code the browser runs, no UI, no drag:

```
PURE-DOC PROBE: {"depth1":1,"depth2":2,"did":true,"depthAfter":1,
                 "u_eq_d1":true,"u_eq_d0":false,"d1_ne_d0":true,"d2_ne_d1":true}
```

So the core is granular on the wasm target too. That left only the browser input path.

**5. The driver double-fires.** A window `keydown` counter, driven by the exact CDP sequence
`keyChord()` sent:

```
keydown events seen by window for ONE keyChord() call: 2      <- rawKeyDown + keyDown + keyUp
keydown events for rawKeyDown+keyUp only:             1
```

CDP `rawKeyDown` is a keydown *without* the char event; `keyDown` is a keydown *with* one. Sending
both = two keydowns = the window handler runs `undo()` twice.

### Why it stayed hidden for two slices, then "appeared" in T-159.22

T-159.21's gate made exactly **one** mutation. Two undos on a one-item stack = one real undo plus a
no-op on an empty stack, so `d2 === d0` and `can_undo === false` both held and the gate went green
**for the wrong reason**. T-159.22 was the first thing to make *two* mutations, and the phantom second
undo instantly looked like "one Ctrl+Z reverted both". Every number in the T-159.22 §Pre-existing
defect section is explained by it, with nothing left over:

| T-159.22 observation | Explanation |
|---|---|
| `undo1 -> == d0? true`, `canUndo=false` (2 drags) | stack `[m1, m2]`, chord = 2 undos → both popped |
| places `9 → 10 → 8`, layer removed by a "second undo" | stack `[layer, s1, s2]`; chord 1 pops `s2`+`s1` → 8; chord 2 pops `layer` |
| "items appear to group by root map" | coincidence of the above; items never grouped |
| "proven pre-existing on untouched `54c8a4bd`" | true, and expected — the *driver* was the constant |

### Falsification (the check that makes this conclusive)

Re-introducing the double-fire into the fixed gate reproduces T-159.22's symptom exactly, and only
that:

```
keydownEvents: 4 | undoDepth: [0, 1, 2, 0, 0, 1]
failed checks: ['a2_undoLandsOnD1', 'a2_undoDidNotLandOnD0', 'a2_depthOne',
                'a2_stillUndoable', 'a7_oneKeydownPerChord']         exit=1
```

`undoDepth` falls **2 → 0** across one chord. Removing the extra dispatch: `keydownEvents: 2`,
`undoDepth: [0, 1, 2, 1, 0, 1]`, exit 0. The defect lives in the driver and nowhere else.

**Superseded:** [`t159_22_verify_log.md`](t159_22_verify_log.md) §Pre-existing defect. Its
observations were real and its analysis honest about being unresolved; its conclusion (a
`map-engine-core` defect) is wrong. Its recommendation — a dedicated slice — was right, and this is
it.

## What changed

| File | Change |
|---|---|
| `.ai/artifacts/t159_gates/driver/smoke_undo_editor.mjs` | **The fix:** `keyChord()` sends one `rawKeyDown` + `keyUp` (was `rawKeyDown` + `keyDown` + `keyUp`). Gate re-based onto **two** drags so it exercises a step boundary; `undo_depth()` + A7 keydown-count assertions added; header/`expectedCount` updated (12 → 21 checks). |
| `crates/map-engine-core/src/doc/store.rs` | `undo_depth()` — read-only accessor (`undo_stack().len()`). Two new unit tests pinning the boundary. Corrected the `extend` comment to cite **both** guards (`last_change > 0` was omitted) and pointed it at the tests. **No behaviour change.** |
| `apps/website-leptos/src/mission_history.rs` | `__editorHistory.undo_depth()` on the read-only gate bridge. **No behaviour change.** |

`undo_depth` is the surface that makes this class of bug visible: `can_undo` only reports "≥ 1", which
is precisely why a double-undo could masquerade as a merge. The gate now distinguishes a capture bug
(two gestures → one item) from a pop bug (one press → two items).

**U5 (React parity):** no contract change. `map-engine-core` behaviour is untouched, so the
`map-engine-wasm` consumer and React's `captureTimeout: 0` contract are unaffected.

## Verify

| Gate | Result |
|---|---|
| `cargo test -p map-engine-core --features doc` | **100 passed** (98 + 2 new), 0 failed; +5 +5 integration |
| `cargo test --release -p map-engine-core --features doc two_local` | 2 passed (rules out an opt-level artifact) |
| `cargo check -p map-engine-core --target wasm32-unknown-unknown --features doc` | clean |
| `trunk build --release` | ✅ success |
| `cargo fmt -p map-engine-core --check` | clean |
| `cargo clippy -p map-engine-core --features doc --all-targets` | **0** warnings |
| `cargo clippy -p website-leptos --target wasm32-unknown-unknown` | 11 warnings — **stash-diff identical to baseline: zero new lints** |
| `smoke_undo_editor` (re-based) | **pass: true** — 21/21 checks, `undoDepth: [0,1,2,1,0,1]`, `keydownEvents: 2` |
| Other 10 editor smokes | all `pass: true`, exit 0 (`smoke_editor`, `smoke_doc_editor`, `smoke_select_editor`, `smoke_marquee_drag_editor`, `smoke_pan_editor`, `smoke_cur_editor`, `smoke_persist_editor`, `smoke_save_export_editor`, `smoke_outliner_palette_editor`, `selfcheck_editor`) |
| Negative control | double-fire restored → gate exits **1** on the 4 boundary checks + A7 |

Pre-existing and left alone (out of scope): `cargo fmt -p website-leptos` drift across ~8 files,
including `mission_history.rs:227` (T-159.21's `onkeydown` closure) — same location on baseline.

## Notes / follow-ups

- **No other gate is affected.** `smoke_undo_editor` was the only driver using `dispatchKeyEvent`;
  `cdp.mjs`'s `dispatchKey` is a thin per-type wrapper with no double-fire, and no other smoke sends
  keys. A7 now guards the regression in the one place it can occur.
- **Operator note:** nothing to re-check by hand in the app — a real key press has always fired one
  `keydown`. The user-facing behaviour never differed from the docs. The T-159.22 line "a user's
  Ctrl+Z throws away more work than they made" was inferred from the gate, not observed by a human.
- **Worth internalising:** the gate was green for two slices while pressing a key twice. Assertions
  that only test a single mutation cannot see step boundaries, and `can_undo` is not a substitute for
  a depth. When an observation contradicts a careful static reading, suspect the measurement — the
  instrument is code too.
