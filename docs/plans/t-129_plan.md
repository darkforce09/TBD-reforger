# T-129 — Plan

## Context
Multi-floor buildings have no floor concept in the editor; descriptors carry no storey field, but the BVH gives
per-building height bands. T-935.8 rewrites occluder_host.rs first; this ticket builds on its loader.

## Approach
1. `world_assets/occluder_host.rs`: `floor_bands(building_id) -> Vec<(z_lo, z_hi)>` from BVH node heights
   (storey height 3 m default, descriptor override when present); wasm test on a synthetic 3-band building.
2. New `editor/tools/floor_selector.rs` (register in `tools/mod.rs`): active floor signal per selection,
   PgUp/PgDn, clip uniform pushed to the render; placement uses the band's z.
3. Perturbation: floor_bands returns one band → test red; restore, `touch`, green.

## Risks
- Sloped roofs produce spurious top bands; clamp bands to >= 2.4 m height.
- Render clipping is a scene uniform in map-engine-render (not owned): if a new uniform is required, report
  found_not_fixed and ship the tool with outline-only clipping.

## Verification
- `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-129`
