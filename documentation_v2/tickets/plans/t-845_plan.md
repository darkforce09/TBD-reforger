# T-845 — Plan

## Context

Wave-207 eye-pass: a selected vehicle looks identical to an unselected one. Slots got the T-808 ringed-twin treatment (`slots_gpu.rs:127/150`) and comments the same wave; the vehicle lane never did.

## Approach

1. Verify on main: render test selecting one of three vehicles shows identical crops (red).
2. `slots_gpu.rs`: ringed-twin atlas cell per vehicle silhouette kind plus selection tint — extra cells, not extra instances; `engine.rs`: patch the vehicle lane on `set_selection` (O(delta), no re-pack), like the comment refresh exits.
3. Test: exactly one of three changes; deselect returns neutral; heading/kind survive; no re-pack counter increment.

## Risks

- Atlas cell budget; count cells before adding.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo test -p map-engine-render` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-845`
