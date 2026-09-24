# T-824 — Plan

## Context

Wave-203 eye-pass: a placed zone is not visible at rest. Drawing works (zone count moves), so the render lane is gated selection-only or missing. Lane recipe from T-760/T-790: feed `after_doc_change` (`mission_editor.rs:1977`) plus the draw_order rebind tail in `engine.rs`; atlas shape at `scene.rs:195`.

## Approach

1. Measure first in a browser at rest and selected at 2–3 zooms; record which case renders.
2. If gated: fix the visibility condition in `zones_panel.rs`/`scene.rs`; if the lane is missing: add the zone lane after the T-760 template in `engine.rs` and bind it from `after_doc_change`.
3. Test: circle and polygon visible at rest at swept zooms; toggles round-trip; count chip matches.

## Risks

- Adding a lane touches the wave-130/141 rebind pin chain; follow the template exactly.

## Verification

- `cargo xtask mk leptos-gates` · `cargo test -p map-engine-render` · `cargo xtask platform wave gate --slice T-824`
