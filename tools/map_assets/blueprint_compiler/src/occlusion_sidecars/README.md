# Occlusion sidecars and prefab placement

The blueprint compiler's 3D side: it turns game models into `.bvh` occlusion sidecars (a
triangle mesh with its bounding volume hierarchy, or BVH), walks a building prefab's children out
of the [Enfusion](/documentation/glossary/a_to_f.md#enfusion) game paks into an instances file, and
checks the result against the engine: line-of-sight parity with the
[Workbench](/documentation/glossary/n_to_z.md#workbench) oracle, socket placements against a recon
dump, and the Euler composition of prefab angles.

## Contents

```text
tools/map_assets/blueprint_compiler/src/occlusion_sidecars/
├── batch_processing/         the `bvh-batch --prefab` entry and the helpers every pak walk shares
├── batch_processing.rs       `Asset`, `LayerPolicy`, `AssetCache`, `ChildEntityWalker`, `SceneSpecification`: the pak walk
├── construction.rs           `bvh-parity` and `bvh-emit`, and `load_compound` for the compound lane
├── instance_verification/    the matching and the `instances-verify` command
├── instance_verification.rs  the recon dump model, the groups, the tolerances and the report
├── prefab_templates/         the `.et` entity-template tokenizer and parser, and one file's own facts
├── prefab_templates.rs       `Block`, `ResolvedPrefab`, `PrefabResolver`: prefab inheritance resolved
├── rotation_validation.rs    `rotation-pin`: the 48 Euler hypotheses scored against a recon sample
├── tests/                    the unit tests of the walk, the sidecars, the placements and the prefab resolver
└── world_instances.rs        `instances-verify --world-row`: sockets placed through a chunk row
```

## How it works

The files are modules of `occlusion_sidecars`, declared in `tools/map_assets/blueprint_compiler/src/occlusion_sidecars.rs`; the unit tests are in `tests/`,
one file per module.

`batch_processing.rs` walks a prefab: `PrefabResolver` resolves its `.et` chain to a mesh, door
parameters, socket pivot, slot bones and children; `AssetCache` decodes each model once into an
`Asset` with its COLL mesh, node table, per-triangle surface kinds and sidecar bytes; and `ChildEntityWalker`
recurses through the children to depth 8, placing each collision-bearing child in the building's
frame, from its parent model's socket (`xobSocket`) or else from its prefab `coords`, `angles` and
`scale` (`prefabCoords`). The single-building run writes the shell sidecar, the shared meshes, the
instances file and an optional scene file; the whole-catalogue run
(`bvh-batch --all-prefabs`) lives in
`tools/map_assets/blueprint_compiler/src/archive_emission/`.

```text
Prefabs/…/X.et ──PrefabResolver──▶ ResolvedPrefab ──Walker──▶ InstanceRecord per child
      │                                                 │
      └── model .xob ──AssetCache::load──▶ Asset ───────┴──▶ blas/<stem>.bvh, buildings/<slug>.bvh
```

`construction.rs` runs the parity checks. `bvh-parity` replays a Workbench parity file of
observer and target pairs through a BVH any-hit raycast over either a model's COLL trimesh
(`--mesh`) or an emitted sidecar (`--sidecar`), which must print the same numbers; with
`--instances` it replays through the compound building, shell plus every placed mesh, with glass
and foliage never blocking and doors in the `--doors closed|open` state. It prints the agreement
and always exits 0. `bvh-emit` writes one model's COLL mesh as `<slug>.bvh`, every triangle
opaque, into `--out` or `assets/terrains/everon/prefabs/buildings/`, and parses the bytes back
before writing.

`instance_verification.rs`, `world_instances.rs` and `rotation_validation.rs` pin the placement
maths to the engine. `instances-verify` matches every socket-placed instance to a Workbench recon
dump within 2 cm and 1°, and `--world-row` places the same children through the committed chunk
row and the object catalogue and compares them with the recon's world positions. `rotation-pin`
scores the six axis orders and eight sign patterns of `angles` against a recon sample of a tilted
parent with a rotated child, and exits 1 unless `Rigid::from_enfusion`,
`R_y(yaw) · R_x(-pitch) · R_z(-roll)`, ranks first; it prints the margin to the runner-up.

## Public surface

- `blueprint_compiler::run_occlusion_sidecar_batch`, `run_occlusion_sidecar_emission`, `run_occlusion_sidecar_parity`,
  `run_instance_verification` and `run_rotation_validation`, re-exported by the blueprint root and run by the
  `cargo xtask map` commands of the same names.
- The walk, the resolver and the verification types stay inside the blueprint compiler.

## Boundaries

- Depends on: `tools/map_assets/blueprint_compiler/src/mesh_decoding/` (models, node tables),
  `tools/map_assets/blueprint_compiler/src/architectural_analysis/surface_classification.rs`, and
  `AnalysisParameters` and `SolidInterval` from `tools/map_assets/blueprint_compiler/src/voxel_processing/`;
  the pak reader in `tools/enfusion/enfusion_pak/src/`; the `repository_layout` crate;
  `spatial_indexes::bounding_volume_hierarchy` (the sidecar codec, the BVH and `SurfaceKind`),
  `building_interiors::compound` (instances, doors, `CompoundBuilding`),
  `geometry_primitives::rigid_transform::Rigid` and `interior_line_of_sight::compound_walk`; the
  contract
  `contracts/definitions/building-instances.schema.json`.
- Used by: the blueprint root's re-exports and `cargo xtask map` (`bvh-batch`, `bvh-emit`,
  `bvh-parity`, `instances-verify`, `rotation-pin`); the prefab library and archive writer in
  `tools/map_assets/blueprint_compiler/src/archive_emission/`; `xob-inspect` in
  `tools/map_assets/blueprint_compiler/src/mesh_decoding/`.
- Rules:
  - the compound lane's door parity is pinned (`farmhouse_compound_door_parity_is_pinned` in
    `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/tests/construction_compound_tests.rs`);
  - `Rigid::from_enfusion` stays the winning hypothesis
    (`rigid_from_enfusion_is_the_y_x_z_hypothesis` in
    `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/tests/rotation_validation_tests.rs`).

## Related documentation

- [Triangle mesh bounding volume hierarchy](/crates/geometry/spatial_indexes/src/bounding_volume_hierarchy/README.md) —
  the `TBVH` sidecar format these commands write.
- [Building interiors](/crates/world_objects/building_interiors/README.md) — the
  compound building and instance model the instances files feed.
- [Everon building models](/assets/terrains/everon/prefabs/buildings/README.md) — the committed
  shell sidecar, instances and scene files.
