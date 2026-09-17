# 3D Blueprint Compiler (`developer-tools/src/blueprint`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

3D geometric mesh decoder, voxel raymarching engine, architectural component extractor, and Bounding Volume Hierarchy (BVH) compiler.

Relocated from `xtask/src/map_blueprint/` (13,409 LOC, 32 files) into `developer-tools` where heavy graphics dependencies belong.

---

## 1. Submodules

- **`mesh_reader.rs`** (<450 LOC): Decodes Bohemia's proprietary `.xob` 3D model meshes (`LODS`, `COLL` colliders).
- **`mesh_nodes.rs`** (<400 LOC): Traverses the scene node hierarchy and transforms vertices into object-space coordinates.
- **`voxel_raymarcher.rs`** (<450 LOC): Raymarches triangle meshes against a 3D bounding volume to emit voxel occupancy grids.
- **`architectural_analysis/`**: Geometric algorithms extracting structural components from voxel data:
  - `slabs.rs`: Floor slab separation.
  - `walls.rs`: Wall detection, window/door openings, and rectilinear simplification.
  - `plates.rs`: Horizontal floor plate extraction.
  - `roofs.rs`: Heightfield roof surface generation.
  - `rings.rs`: 2D polygon boundary tracing.
- **`bvh/`**: 3D Bounding Volume Hierarchy acceleration structures for fast line-of-sight and raycast occlusion:
  - `builder.rs`: Builds Bottom-Level Acceleration Structures (BLAS).
  - `binary_emit.rs`: Emits zero-copy `.bvh` sidecar binary files.
  - `batch.rs`: Batch generator iterating across all world prefabs.
- **`archive_compiler.rs`** (<450 LOC): Folds extracted blueprints and BLAS structures into `prefabs/building_blueprints.rkyv`.
