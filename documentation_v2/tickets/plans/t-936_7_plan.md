# T-936.7 — Plan

## Context
$defs/marker (mission.schema.json:1171) is a closed single-point icon; T-673 adds color/fill/rotation/shape.
No polyline, boundary or arrow graphic exists in schema, canvas or compiled document.

## Approach
1. Schema `tacticalGraphics[]` {id, kind, points[][2] minItems 2, label?, sideKey?, style?}; golden updated.
2. `mission/tactical_graphics.rs` (new, in mission/mod.rs): model + validator; register in extensions.rs.
3. `canvas/tactical_graphics.rs` (new, in canvas/mod.rs): draw per kind, Catmull-Rom arrows, vertex drag.
4. Perturbation: accept a one-point phase line → validator test red; restore, touch, green.

## Risks
- Style vocabulary must reuse T-673's color/fill — hence depends_on T-673.
- Hit-testing polylines on the canvas — reuse the overlay layer's existing picking.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask ci schema-validate`; `cargo xtask mk ci-local-leptos`
- `cargo xtask mk leptos-gates`; `cargo xtask platform wave gate --slice T-936.7`
