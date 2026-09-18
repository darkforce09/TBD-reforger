# Blueprint compilation

This subsystem decodes model meshes and voxel dumps, extracts building structure, builds bounding volume hierarchies (BVHs), and emits blueprint libraries and binary archives. Command adapters supply the active repository path. Shared PAK parsing and archive access belong to `../enfusion_pak/`; blueprint and spatial contracts come from website-map-engine.

## Directory ownership

- `mesh_decoding/`: `.xob` mesh and collision decoding, scene-node records, and archive inspection.
- `voxel_processing/`: mesh voxelization, voxel-dump parsing, voxel types, analysis parameters, and synthetic test fixtures.
- `architectural_analysis/`: vertical slabs, wall extraction, floor plates, roof profiles, polygon rings, contour tracing, convex hulls, and surface classification.
- `bvh/`: BVH construction and sidecar emission, batch processing, prefab catalogs, world instances, instance-pair verification, and rotation validation.
- `archive_emission/`: blueprint assembly, library reading, archive writing, and archive command handling.
- `tests/`: separate Rust test modules covering geometry, decoding, compilation, archives, and parity.

`mod.rs` exposes the command entrypoints and coordinates voxel interpretation and per-floor assembly. Its `#[path]` declarations map the internal modules to their responsibility directories. `ingest.rs` discovers profile exports, validates building blueprints, and copies them into repository assets. `parity_report.rs` compares recorded Workbench trace results with blueprint line-of-sight results.

## Command entrypoints

The public entrypoints support voxel interpretation, mesh voxelization, BVH emission and batch compilation, BVH parity, instance verification, rotation checks, and PAK/model inspection. `ingest::run` and `parity_report::run` provide ingestion and parity reporting.

`cargo xtask map blueprint-from-voxels` interprets voxel dumps and writes validated building JSON. Its `archive` subcommand assembles the blueprint library into `building_blueprints.rkyv`. `cargo xtask map ingest-blueprints` imports exported building JSON; `cargo xtask map parity-report` reports agreement with Workbench trace results.
