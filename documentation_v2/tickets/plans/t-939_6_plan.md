# T-939.6 — Plan

## Context
overlays.rs:654-655 "no map glyph" for connections; ConnectionsPanelOverlay :679 is panel-only; validation findings
never reach the canvas. Packs after T-939.3 (overlays.rs, canvas/mod.rs).

## Approach
1. Verify on main: test that a slot with a finding renders no badge; paste the red.
2. `canvas/diagnostics_overlay.rs` (new, in canvas/mod.rs): badge placement, wire geometry, tooltip text.
3. overlays.rs mounts it above the gizmo layer; hidden layers filter badges and wires.
4. Badge click selects + opens the validation entry; dangling connection ends draw a badge.
5. Perturbation: skip hidden-layer filtering → test red; restore, touch, green.

## Risks
- Wire count on large missions: draw only resident endpoints; batch into one path.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.6`
