# T-822 — Plan

## Context

The left dock is a DOM descendant of the map's dblclick container (`mission_editor.rs:1208`), so an outliner-row dblclick opens Attributes AND the empty-ground asset picker under it. The pointerdown chrome guard already stops pointerdown, not dblclick.

## Approach

1. Verify on main: container-dblclick at a dock row → `attrH2 = 1` and `placeAsset = 1` (red).
2. `mission_editor.rs`: in the dblclick handler, ignore targets whose `composedPath`/`closest` hits a dock subtree — mirror the pointerdown guard.
3. Map-ground dblclick still opens the picker; test both.

## Risks

- `mission_editor.rs` is SIZE-3 allowlisted; keep the change to the handler.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-822`
