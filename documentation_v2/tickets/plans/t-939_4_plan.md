# T-939.4 — Plan

## Context
top_strip.rs:249-252 Arrange menu (T-645), gated at :378-380; context_menu.rs has no entries; no chords; no help rows.
Keymap = mission_editor.rs wasm keydown closure (allowlisted SIZE-3 file).

## Approach
1. Verify on main: test that the context menu lacks Arrange and the keydown ignores the align chord; paste the red.
2. top_strip.rs: one shared entry list (label, chord, invoker).
3. context_menu.rs: Arrange submenu when selection ≥ 2.
4. mission_editor.rs: chord arms as one-line callers; help_modal.rs: rows.
5. Perturbation: drop one chord arm → keydown test red; restore, touch, green.

## Risks
- mission_editor.rs file-length: arms must be thin; the list lives in top_strip.rs.
- Chord clashes with existing bindings: read the help modal table first.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.4`
