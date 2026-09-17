# BVH Collision Trees (`developer-tools/src/blueprint/bvh`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Compiles and verifies 3D Bounding Volume Hierarchy (BVH) trees for building collision and raycast occlusion queries.

---

## Submodules

- **`builder.rs`** (<450 LOC): Surface Area Heuristic (SAH) BVH tree builder for building geometry.
- **`binary_emit.rs`** (<400 LOC): Serializes BVH nodes and leaf primitive references into `.bvh` golden sidecars.
- **`parity.rs`** (<350 LOC): Verifies that raycasting against emitted BVH trees matches original `.xob` triangles.
- **`batch.rs`** (<450 LOC): Multi-threaded batch walker processing all Everon prefabs.
