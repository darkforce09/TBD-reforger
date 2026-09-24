# T-143 — Plan

## Context
T-935.9 ships the water mask/vectors and `world/water.rs`; T-305 fixes pak.rs offsets. This ticket uses both:
a placement guard in the editor and an exact water source (Eden water entities via the pak reader).

## Approach
1. New `state/operations/placement_guard.rs` (register in `state/operations.rs`): `check(x, z) -> Ok | Warn(kind)
   | Refuse(kind)`; place/move ops call it; toast on refuse; override flag. wasm tests on a synthetic mask.
2. `tools/tbd-tools/src/world/pak.rs`: entity-layer reader for water entities (lake/river/pond + surface_y) with a
   fixture pak; output feeds the T-935.9 `map water` emit.
3. `crates/map-engine-core/src/world/water.rs`: `body_kind(x, z)` (ocean/lake/river/none) from mask + vectors.
4. Perturbation: guard treats ocean as none → refuse test red; restore, `touch`, green.
## Risks
- Eden water entities may not be in the shipped paks (the T-090.1.2.5.2 spike found codec failures); fallback is the
  Workbench vector export T-935.9 already consumes — report which source was used.

## Verification
- `cargo test -p map-engine-core --all-features world::water` · `cargo test -p tbd-tools --lib world::pak`
- `cargo xtask mk ci-local-leptos` · `cargo xtask platform wave gate --slice T-143`
