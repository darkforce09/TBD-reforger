# T-704 — Plan

## Context
3den's command palette (3DEN-TOOL-013/014) is the highest-ceiling chrome fix for the T-668 navigation complaint.
Commands already exist as enums (top_strip.rs Cmd :105, toolbelt); the palette only indexes and dispatches them.

## Approach
1. New `panels/command_palette.rs` (register in `panels/mod.rs`): `CommandEntry {id, label, hotkey, run}` built
   from the existing enums; fuzzy scorer (subsequence + prefix bonus); usage counts in localStorage (try/catch).
2. Overlay under modal_stack; Alt+Space toggle wired in `mission_editor.rs` (call site only, allowlisted SIZE-3).
3. wasm tests: ranking, toggle, coverage (entry count == enum variant count).
4. Perturbation: drop one entry → coverage test red; restore, `touch`, green.

## Risks
- Alt+Space is a window-manager shortcut on some desktops; also bind Ctrl+K and document both.
- Duplicating handlers would drift; the palette must call the same `run` closures the buttons use.

## Verification
- `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-704`
