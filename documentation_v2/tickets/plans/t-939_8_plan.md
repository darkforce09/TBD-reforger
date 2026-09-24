# T-939.8 — Plan

## Context
dock_left.rs:134-144 search_document (T-697) is keyboard-unreachable; no f arm in the mission_editor.rs keydown.
Packs after T-939.4 (mission_editor.rs, help_modal.rs).

## Approach
1. Verify on main: test dispatching Ctrl+F leaves the search input unfocused; paste the red.
2. dock_left.rs: `focus_search` (expand, focus, select text); Escape returns focus without clearing.
3. mission_editor.rs: Ctrl+F / Cmd+F arm with preventDefault; help_modal.rs row.
4. Perturbation: drop preventDefault → consumed-event test red; restore, touch, green.

## Risks
- Focus while a modal is open: the arm is inert when a modal owns focus.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.8`
