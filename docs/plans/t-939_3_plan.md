# T-939.3 — Plan

## Context
overlays.rs:172-198 draws Translate X/Y only; :200-210 a flat rotate ring; the gizmo enum is total. No vertical drag.

## Approach
1. Verify on main: test that hit-testing above the origin returns None and the enum has no Z variant; paste the red.
2. `canvas/gizmo_z.rs` (new, in canvas/mod.rs): arm geometry, hit test, dy→metres at camera scale, snap, readout.
3. overlays.rs draws the arm + readout; gestures.rs routes a Z-arm press to the drag; z written via the translate op.
4. Shift suspends snapping.
5. Perturbation: invert the dy sign → unit test red; restore, touch, green.

## Risks
- Camera scale changes mid-drag: sample scale at press, not per move.
- overlays.rs/canvas/mod.rs also owned by T-939.6 → it packs later.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.3`
