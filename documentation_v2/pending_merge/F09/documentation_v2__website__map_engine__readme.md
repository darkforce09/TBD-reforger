# Map Engine & Spatial Computation (`website/map_engine/`)

The map engine manages world spatial computation, terrain elevation models, 512m chunk residency, and the scenario CRDT document model.

## Subsystems
- `world/`: Digital Elevation Models (DEM), 512m terrain chunking, inland water mask classification, building voxel meshes.
- `spatial/`: 3D Bounding Volume Hierarchies (BVH), raymarching, hardware picking, line-of-sight (LOS) computations.
- `camera/`: 2D/3D camera projections, pan/zoom physics, viewport frustum culling, metric <-> MGRS coordinate unproject.
- `data/`: Yjs/yrs CRDT scenario store, binary rkyv serialization, scenario AST compilation and schema validation.
- `symbology/`: NATO MIL-STD-2525 tactical graphics and symbol rasterization.
- `frame/`: Translates world state and camera viewport into pure `graphics_engine::frame` render packets.

## Code Mapping
- Source: `apps/website/map-engine/`
