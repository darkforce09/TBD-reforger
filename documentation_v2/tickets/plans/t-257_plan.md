# T-257 — Plan

## Context
store.rs:372-375 undo-scopes slots, squads, factions, editor_layers (vehicles joined under T-180.2). hydrate
clears loadouts, items, objectives and markers too, but they are outside the UndoManager — a trap for T-936.2 tasks
and the marker work, which will mutate them.

## Approach
1. `crates/map-engine-core/src/doc/store.rs` tests: for each of the four roots, insert a row under LOCAL origin,
   undo, assert the root is empty again, redo, assert it is back — red on main; paste it.
2. Add `undo_mgr.expand_scope(&doc, &<root>)` for the four roots next to :372-375.
3. Perturbation: remove the markers line → its test red; restore, `touch`, green. Run with `--all-features`.

## Risks
- hydrate's clear must keep its non-tracked origin or the first undo after load would wipe the document; the
  existing hydrate test guards that — keep it green.

## Verification
- `cargo test -p map-engine-core --all-features doc::store`
- `cargo xtask platform wave gate --slice T-257`
