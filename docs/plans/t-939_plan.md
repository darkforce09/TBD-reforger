# T-939 — Plan

## Context
Master audit S4 (2026-09-04) verified against main: the editor lacks multi-select drag, batch reassign, a Z gizmo,
Arrange shortcuts, squad templates, canvas diagnostics, vehicle virtualization and Ctrl+F. Eight slices under
`apps/website/frontend/src/editor/`; keymap = the wasm keydown closure in `mission_editor.rs`.

## Approach
1. Wave A (disjoint owns): T-939.1 outliner drag, T-939.3 Z gizmo, T-939.5 squad templates, T-939.7 virtualization.
2. Wave B: T-939.2 (after T-937.2 batch undo), T-939.4 Arrange (after .1), T-939.6 diagnostics (after .3).
3. Wave C: T-939.8 Ctrl+F (after .4; shares mission_editor.rs and help_modal.rs).
4. Each slice: red test on main, new file for new logic, perturbation with touch, `leptos-gates`.

## Risks
- mission_editor.rs is an allowlisted SIZE-3 file: keydown arms stay one-line callers or the file-length gate fails.
- Multi-entity edits must be one transaction or undo fragments; T-939.2 leans on T-937.2's `with_batch`.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.N` per child
