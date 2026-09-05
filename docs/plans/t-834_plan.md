# T-834 — Plan

## Context

Wave-205 NIT batch: two doc comments still describe the pre-renumber digit map; `EditorToolbarDispatch.widget_is_rotate` (`mission_editor.rs:799`) is dead (`widget_digit` is the read); the "Placing a Attack marker" hint is at `dock_right.rs:3888`. Absorbs T-835 (No-Widget glyph) and T-840 (draft chip says "saved just now" on boot).

## Approach

1. Fix the two comments to the 1/2/3 map (lines per wave205.md); remove `widget_is_rotate` and its registration/call sites.
2. `dock_right.rs`: article grammar ("an Attack marker" or reworded).
3. T-835: No-Widget button wears the arrow glyph; T-840: draft chip shows no "saved just now" before any edit.

## Risks

- None; mechanical. `mission_editor.rs`/`top_strip.rs` are shared with T-839 — later wave.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-834`
