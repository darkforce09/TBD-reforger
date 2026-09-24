# T-939.1 — Plan

## Context
outliner_tree.rs:1110 `begin_layer_slot_drag(one String)` (peers :1022, :664, :1101): a multi-selection drag moves
only the pressed row. Priority 2 UX.

## Approach
1. Verify on main: test dragging with two selected rows moves one; paste the red.
2. `panels/outliner_drag.rs` (new, in panels/mod.rs): `DragSet {anchor, ids}`, drop planning (order kept, self-drop rejected).
3. Thread `DragSet` through the four call sites; apply the drop in one doc-store transaction; ghost shows the count.
4. Perturbation: drop the ids tail → two-row test red; restore, touch, green.

## Risks
- Dropping a set into one of its own rows: plan rejects; unit-tested.
- panels/mod.rs also owned by T-936.1 — separate waves.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.1`
