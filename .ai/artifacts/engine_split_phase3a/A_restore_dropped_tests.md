# A — restore three tests the first 3A agent deleted

**Run this first. It is a regression fix, and it is small.**

Test accounting across the first agent's eight commits: the frontend lost **117** `#[test]` fns
and map-engine gained **116**. Three tests left the frontend and never arrived. All three were
real, substantive tests. Two of them are still *cited by comments that claim they exist* — those
comments currently lie, which is worse than the missing coverage.

Recover the originals with `git show a4b86912d:<path>`.

## A1 · `no_los_doc_writes`
Was in `apps/website/frontend/src/editor/tools/los_tool.rs`. A source scrub proving the LOS tool
never writes the document:

```rust
let code = crate::v2::core::test_support::class_r_scrub::live_code(include_str!("los_tool.rs"));
for banned in ["MissionDocCore","move_entities","add_slot","store.rs","hydrate",
               "after_local_edit","editor_ops"] { assert!(...) }
```

`apps/website/map-engine/src/editing/tools/line_of_sight/tests/capture.rs:152` and `:158`
already say *"Covered by `no_los_doc_writes` above (whole-file scrub)"*. **There is no such test
above.** Restore it against the engine-side LOS sources.

## A2 · `no_ruler_doc_writes`
Was in `apps/website/frontend/src/editor/tools/ruler_tool.rs`. Same scrub, same banned list, over
`ruler_tool.rs`. `apps/website/frontend/src/editor/panels/toolbelt.rs:1369` says *"the real
no-doc-writes proof is `ruler_tool`'s `no_ruler_doc_writes`"* — also now false. Restore it against
the engine-side ruler sources.

## A3 · `the_exporter_grid_ref_is_the_map_furnitures_own_label_text`
Was in `apps/website/frontend/src/editor/state/commands_hotkeys.rs`. Not a scrub — a behavioural
test. It builds an `OrthoCamera`, calls `toolbelt::{edge_eastings, edge_northings}`, and asserts
the exporter's grid reference equals the map furniture's own edge label text.
`apps/website/map-engine/src/editing/commands/selection_digest.rs:24` still cites it. It now spans
the crate boundary (exporter engine-side, toolbelt frontend-side) — put it wherever it can still
assert that equality, and give one line saying why it lives there.

## Why this matters more than three test names

A1 and A2 are the proof that Phase 3A is honest: that the tools compute and never mutate the
document. They are `include_str!` source scrubs, so they were bound to the frontend file paths —
when the tools moved, the scrub target moved and they were dropped rather than repointed. Losing
the invariant to the very refactor it polices is not acceptable.

The same failure mode is already known to be waiting in 3B: `render_sync.rs` is `include_str!`d
by `t802_hover_cursor.rs:21`, `t808_symbology_feed.rs:23` and `t784_comment_glyph.rs:33`.

## Done when

- The three tests exist, run, and pass.
- No comment anywhere cites a test that does not exist. Sweep for it.
- You show the arithmetic: frontend-lost equals map-engine-gained.

```
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```
