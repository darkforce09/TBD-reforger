# T-838 — Plan

## Context

Operator rule (wave-205): markers cannot be selected by map click; the right dock only places. Markers render as pickable targets since T-790; the comment glyph pick lane (`pick_comment`, `mission_editor.rs:53`) is the template; selection ops live in `operations/entity.rs` (`set_selection_ids :4772`). T-831 (per-side) is separate.

## Approach

1. `mission_editor.rs`: marker map-click pick into `ctx.selection` after the comment lane; dblclick opens marker Attributes (T-763 lineage).
2. `outliner_tree.rs`: placed markers listed like the T-809 placed-vehicles interim pattern; row dblclick opens Attributes.
3. `operations/entity.rs`: Del removes a marker in one undo; prune selection by doc presence (invisible ≠ gone). Dock marker tab keeps only the picker.

## Risks

- F-07 / T-790 guard bars new dock editing lanes — do not add any.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-838`
