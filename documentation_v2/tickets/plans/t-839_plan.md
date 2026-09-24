# T-839 — Plan

## Context

Operator decision 1 (review §8) retired the floating Select/Ruler/LoS pill; T-797/T-798 neither carried it. The T-636 pill mounts live in `toolbelt.rs` (header comment) and `mission_editor.rs:2749-2757`; Ruler/LoS arming (T-642/T-643) is unchanged — only the buttons move to the row-2 toolbar (`top_strip.rs`).

## Approach

1. `top_strip.rs`: Ruler and LoS-ray buttons in the row-2 tools group with BTN_ICON chord tooltips; no separate Select button (T-835 arrow glyph = No-Widget).
2. `toolbelt.rs` + `mission_editor.rs`: delete the pill component and its mount; shrink the Backspace hide-set; update any pill pins honestly.
3. Test: no pill in any mode; arming from row-2 works; Backspace hides all chrome.

## Risks

- Shares `top_strip.rs`/`mission_editor.rs` with T-834 — later wave.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-839`
