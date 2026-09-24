# T-149 — Plan

## Context
Forest hulls (build.rs:593 forest-regions emit) are raw marching-squares rings over the 8 m TBDD density grid
(density.rs:24) — blocky at every edge. A Chaikin pass is cheap and format-neutral.

## Approach
1. New `tools/tbd-tools/src/world/forest_smooth.rs` (register in `world/mod.rs`): `chaikin(ring, iters, keep_corners)`;
   tests: square rounds, area drift < 3%, 3-point ring valid, ring stays closed.
2. `world/build.rs` forest-regions emit: smooth each ring; log per-region vertex delta and area drift.
3. `density.rs`: expose the grid sampler the smoother uses for corner-preservation (no format change).
4. Local everon run for the numbers (rebuilt artifact not committed unless the command center asks).
5. Perturbation: iters = 0 → rounding test red; restore, `touch`, green. Edition-2024 rustfmt.
## Risks
- Tiny rings collapse under smoothing; skip rings under 6 vertices.
- map-object-golden may pin the current hulls; if so, regenerate the golden in the same slice and report it.

## Verification
- `cargo test -p tbd-tools --lib world::forest_smooth` · `cargo xtask ci verify map-object-golden`
- `cargo xtask platform wave gate --slice T-149`
