# T-938.4 — Plan

## Context
building_section.rs:292 section_at_owned iterates every triangle; occl.bvh (bvh_sidecar.rs:219) unused here.
HeightField (:46 MAX_PLAN_DIM 2048, :81 vec![None; cols*rows] of Option<f64>) allocates up to 67 MB per level.

## Approach
1. Verify on main: triangles visited per cut and bytes per HeightField on a golden building → paste.
2. `building_section_index.rs` (new, in lib.rs): y-interval BVH query → candidate triangles.
3. section_at_owned uses the index; equality test on the six golden buildings.
4. HeightField: f32 + NaN sentinel in lazily allocated tiles behind the existing accessors.
5. Perturbation: zero-height y-interval → golden test red; restore, touch, green.

## Risks
- BVH bounds are for occlusion; confirm they enclose the section geometry before trusting the query.
- f32 vs f64 heights — tolerance in the equality test, documented.

## Verification
- `cargo test -p map-engine-core --all-features`
- `cargo xtask platform wave gate --slice T-938.4`
