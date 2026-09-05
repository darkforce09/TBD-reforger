# T-828 — Plan

## Context

Wave-204 MINOR: `MarkerComposite` (`engine.rs:316`) recomputes icon screen offsets per frame (T-790) but freezes caption positions at bind-time zoom; gaps of 43/52 px appear at 1.30/1.06 m/px until a doc change forces a rebind.

## Approach

1. Verify on main: a render test zooming 5.6 → 1.06 m/px with no doc change shows caption drift > 40 px (red).
2. `engine.rs`: compute caption offsets in the same per-frame screen-space pass as icons from one shared transform.
3. Perturbation proof; sweep test stays green.

## Risks

- Text layout per frame must reuse cached glyph runs or frame time regresses.

## Verification

- `cargo test -p map-engine-render` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-828`
