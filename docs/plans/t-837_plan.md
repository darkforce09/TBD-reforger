# T-837 — Plan

## Context

Wave-205 eye-pass (XFORM-DEL-001): a placed vehicle cannot be deleted while slots can. `delete_selection` (`operations/entity.rs:64`) partitions comments and slots only (absorbed T-844); the outliner PLACED VEHICLES rows (`outliner_tree.rs:1129-1139`) have no delete affordance; vehicle ids live in `ctx.selection` (wave-145).

## Approach

1. Audit: which delete paths (Del, context menu, outliner row) skip vehicles vs are absent — record it.
2. `operations/entity.rs`: vehicle partition in `delete_selection` releasing crew assignments in one undo step; `context_menu.rs` Delete and `outliner_tree.rs` row delete call it; `mission_editor.rs` Del key path.
3. Test: place → select → Del → gone from map, outliner and compiled export; undo restores with crew.

## Risks

- T-819 derived-hide of crewed slots must flip back on delete; assert it.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-837`
