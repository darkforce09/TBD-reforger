# T-841 — Plan

## Context

Wave-206 eye-pass: the T-810 Type picker popover (`attributes_modal.rs:1579` `type_picker`) sits on a translucent plate, so option text blurs over the map. T-827's lesson: measure live composite, not the token sheet.

## Approach

1. `attributes_modal.rs`: give the popover (and its search input plate) the solid modal surface token — no `/nn` alpha, no backdrop-blur.
2. Measure option-text contrast on the live composite ≥ 4.5:1; screenshot before/after.

## Risks

- None; visual only. `attributes_modal.rs` is SIZE-3 allowlisted — minimal diff.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-841`
