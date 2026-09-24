# T-816 — Plan

## Context

Wave-201 NIT-1: with a composition armed and the Controls Hint open, one Esc clears both. The hint (`top_strip.rs:117`, T-692 toggle) is not a `modal_stack` participant, so the editor's `any_open()`-gated arm-cancel (`mission_editor.rs:202` `cancel_pending`) fires in the same keydown as the strip's hint close; the T-814 consume-aware guard covers stack dialogs only.

## Approach

1. Verify on main with a keydown test: arm + hint open → one Esc → both gone (red).
2. `core/ui.rs`: let the hint close mark the event consumed (or register the hint as a stack participant); `top_strip.rs`: use it.
3. Keep the T-814 ladder test green; perturbation proof.

## Risks

- T-797/T-798 reworked the strip; anchors may have moved — re-verify before editing.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-816`
