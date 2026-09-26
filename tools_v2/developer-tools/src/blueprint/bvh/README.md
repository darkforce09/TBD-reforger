# Occlusion sidecars and prefab placement

The blueprint compiler's 3D side: it turns game models into `.bvh` occlusion sidecars (a
triangle mesh with its bounding volume hierarchy, or BVH), walks a building prefab's children out
of the [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) game paks into an instances file, and
checks the result against the engine: line-of-sight parity with the
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) oracle, socket placements against a recon
dump, and the Euler composition of prefab angles.

## Contents

```text
tools_v2/developer-tools/src/blueprint/bvh/
├── batch_processing/         the `bvh-batch --prefab` entry and the helpers every pak walk shares
├── batch_processing.rs       `Asset`, `LayerPolicy`, `AssetCache`, `Walker`, `SceneSpec`: the pak walk
├── construction.rs           `bvh-parity` and `bvh-emit`, and `load_compound` for the compound lane
├── instance_pairs.rs         one-sided collision face pairing into solid intervals, for the walls
├── instance_verification/    the matching and the `instances-verify` command
├── instance_verification.rs  the recon dump model, the groups, the tolerances and the report
├── prefab_catalog/           the `.et` tokenizer and one file's own facts
├── prefab_catalog.rs         `Block`, `ResolvedPrefab`, `PrefabResolver`: prefab inheritance resolved
├── rotation_validation.rs    `rotation-pin`: the 48 Euler hypotheses scored against a recon sample
└── world_instances.rs        `instances-verify --world-row`: sockets placed through a chunk row
```

## How it works

The files are modules of the blueprint root, declared in
`tools_v2/developer-tools/src/blueprint/mod.rs` by `#[path]` as `batch`, `bvh`, `pair`, `verify`,
`prefab`, `rotation_pin` and `world_row`.

`batch_processing.rs` walks a prefab: `PrefabResolver` resolves its `.et` chain to a mesh, door
parameters, socket pivot, slot bones and children; `AssetCache` decodes each model once into an
`Asset` with its COLL mesh, node table, per-triangle surface kinds and sidecar bytes; and `Walker`
recurses through the children to depth 8, placing each collision-bearing child in the building's
frame, from its parent model's socket (`xobSocket`) or else from its prefab `coords`, `angles` and
`scale` (`prefabCoords`). The single-building run writes the shell sidecar, the shared meshes, the
instances file and an optional scene file; the whole-catalogue run
(`bvh-batch --all-prefabs`) lives in
`tools_v2/developer-tools/src/blueprint/archive_emission/`.

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
opaque, into `--out` or `assets_v2/terrains/everon/prefabs/buildings/`, and parses the bytes back
before writing.

`instance_verification.rs`, `world_instances.rs` and `rotation_validation.rs` pin the placement
maths to the engine. `instances-verify` matches every socket-placed instance to a Workbench recon
dump within 2 cm and 1°, and `--world-row` places the same children through the committed chunk
row and the object catalogue and compares them with the recon's world positions. `rotation-pin`
scores the six axis orders and eight sign patterns of `angles` against a recon sample of a tilted
parent with a rotated child, and exits 1 unless `Rigid::from_enfusion`,
`R_y(yaw) · R_x(-pitch) · R_z(-roll)`, ranks first; it prints the margin to the runner-up.

`instance_pairs.rs` belongs to the voxel side: Enfusion collision faces register only when marched
into from the front, so it pairs each forward entry face with the nearest closing face to rebuild
solid intervals for wall extraction.

## Public surface

- `developer_tools::blueprint::run_bvh_batch`, `run_bvh_emit`, `run_bvh_parity`,
  `run_instances_verify` and `run_rotation_pin`, re-exported by the blueprint root and run by the
  `cargo xtask map` commands of the same names.
- The walk, the resolver and the verification types stay inside the blueprint compiler.

## Boundaries

- Depends on: `tools_v2/developer-tools/src/blueprint/mesh_decoding/` (models, node tables),
  `tools_v2/developer-tools/src/blueprint/architectural_analysis/surface_classification.rs`, and
  `Params` and `SolidInterval` from `tools_v2/developer-tools/src/blueprint/voxel_processing/`;
  the pak reader in `tools_v2/developer-tools/src/enfusion_pak/`; `crate::repository_layout`;
  `website_map_engine::spatial::bvh` (the sidecar codec, the BVH and `SurfaceKind`),
  `website_map_engine::world::architecture::compound` (instances, doors, `CompoundBuilding`,
  `Rigid`) and `website_map_engine::spatial::los::interior::walker`; the contract
  `contracts_v2/definitions/building-instances.schema.json`.
- Used by: the blueprint root's re-exports and `cargo xtask map` (`bvh-batch`, `bvh-emit`,
  `bvh-parity`, `instances-verify`, `rotation-pin`); the prefab library and archive writer in
  `tools_v2/developer-tools/src/blueprint/archive_emission/`; `xob-inspect` in
  `tools_v2/developer-tools/src/blueprint/mesh_decoding/`; wall extraction in
  `tools_v2/developer-tools/src/blueprint/architectural_analysis/`.
- Rules:
  - sidecars are deterministic: the committed
    `assets_v2/terrains/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.bvh` is byte-identical to
    `tools_v2/developer-tools/test_fixtures/blueprint/FarmHouse_E_1L01_Wood.bvh.golden`, and 400 of
    400 oracle pairs agree over it (`farmhouse_bvh_sidecar_parity_is_pinned` in
    `tools_v2/developer-tools/src/blueprint/tests/bvh/tests.rs`);
  - the compound lane's door parity is pinned (`farmhouse_compound_door_parity_is_pinned` in
    `tools_v2/developer-tools/src/blueprint/tests/bvh/compound_tests.rs`);
  - `Rigid::from_enfusion` stays the winning hypothesis
    (`rigid_from_enfusion_is_the_y_x_z_hypothesis` in
    `tools_v2/developer-tools/src/blueprint/tests/rotation_pin/tests.rs`).

## Related documentation

- [Triangle mesh bounding volume hierarchy](/apps/website/map-engine/src/spatial/bvh/README.md) —
  the `TBVH` sidecar format these commands write.
- [Building architecture](/apps/website/map-engine/src/world/architecture/README.md) — the
  compound building and instance model the instances files feed.
- [Everon building models](/assets_v2/terrains/everon/prefabs/buildings/README.md) — the committed
  shell sidecar, instances and scene files.
