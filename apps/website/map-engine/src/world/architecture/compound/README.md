# Building compounds

A building as its collision shell plus every entity placed in it: door leaves and frames, window
frames, glass panes, furniture, props and trees, each its own mesh under a rigid transform, with
door leaves that open and close. The line-of-sight walk, the visibility wash and the section
drawings run over it.

## Contents

```text
apps/website/map-engine/src/world/architecture/compound/
├── assembly.rs   `CompoundBuilding`: a shell and its instances, assembled atomically and flattened
├── doors.rs      door records and states, the hinge or slide motion, and the door lookups
├── instances.rs  the `<slug>.instances.json` model: instance records and kinds, and live instances
├── mod.rs        the module tree
├── scene/        the compound items under one path, and the instances JSON round-trip test
├── tests/        unit tests for rigid transforms: inverses, quaternions, nested precision
└── transform.rs  `Rigid`: rotation, translation and uniform scale, and Enfusion angle conversion
```

## How it works

The instances file `prefabs/buildings/<slug>.instances.json` sits beside the blueprint JSON in a
terrain's asset folder
(`assets_v2/terrains/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.instances.json`) and reads
into `InstancesFile`, at `INSTANCES_SCHEMA_VERSION` (1.0.0). It names the shell's sidecar
(`shellBvh`, relative to the file) and lists one `InstanceRecord` per placed entity: an id (the
prefab child's `ID` chain joined by `/`), an `InstanceKind`, the child's prefab, its BLAS sidecar
path relative to the prefabs root (`blas/<asset>.bvh`), a `LocalTransform` (position, unit
quaternion `[x, y, z, w]`, uniform scale, 1 when absent), a `DoorRecord` for a door leaf, a
`CoverTier` (`full`, `low` or `none`), a `PlacementSource` and the parent it hangs under.
A hand-placed `<slug>.scene.json` adds trees the same way.

`CompoundBuilding::assemble(shell, records, blas_by_path)` resolves every record's `blas` string
through the caller's map. It is atomic: when a path is missing it returns
`CompoundError::MissingBlas` with each missing path once, in first-use order, and assembles
nothing; `append` adds more records under the same rule. Each `Instance` keeps its record, the
shared BLAS, its placement at rest (the leaf's closed pose for a door), the BLAS's root bounds and
a `DoorState`.

A door is a `DoorLeaf` with a `DoorRecord`. It starts open by the fraction
`(initialAngleDeg − closedAngleDeg) / angleRangeDeg`, over a sweep of 1 for a slider, and closed
when that is not above zero. `hinge()` turns a hinged leaf about its local Y to
`closedAngleDeg + fraction · angleRangeDeg`, or slides a sliding leaf `fraction · openedDistance`
along its local X, and `placement()` applies the hinge before the rest placement.
`DoorState::fraction` clamps to 0–1 and reads a non-finite fraction as closed; `toggled` flips
between closed and fully open; `set_door` and `door_state` refuse an id that is no door.

`flatten()` bakes every instance at its current state into one mesh after the shell and rebuilds a
BVH over the union; `FlatMesh::owner` gives each triangle's owner, 0 for the shell and `i + 1` for
instance `i`. The section drawings and height fields read that mesh.

`Rigid` maps a point as `m · (scale · p) + t`. `compose` applies its argument first, `inverse`
transposes the rotation and takes the reciprocal scale, and `from_enfusion` turns Enfusion's
`coords`, `angles` (pitch, yaw and roll, in degrees) and `scale` into
`rot_y(yaw) ∘ rot_x(−pitch) ∘ rot_z(−roll)` at that position. `to_quat` returns the quaternion
with `w ≥ 0`, `yaw_deg` the plan heading in (−180°, 180°], and `aabb_of` the bounds of a
transformed box.

## Public surface

- `assembly`: `CompoundBuilding` (`assemble`, `append`, `placement`, `flatten`, `doors`,
  `set_door`, `door_state`, `instance_index`), `CompoundError`, `FlatMesh`, `CoverTier`,
  `PlacementSource` and `INSTANCES_SCHEMA_VERSION`.
- `instances`: `InstancesFile`, `InstanceRecord`, `InstanceKind`, `LocalTransform`, `Instance` and
  `instances_from_records`.
- `doors`: `DoorRecord` and `DoorState`.
- `transform`: `Rigid`.

## Boundaries

- Depends on: `crate::spatial::bvh` (`BvhSidecar`, `Bvh` and `SurfaceKind`); `serde` for the JSON.
- Used by:
  - `crate::spatial::los::interior`, whose walker traces through a `CompoundBuilding` and whose
    wash rasters it;
  - `crate::spatial::los::world`: the occluder's residency builds instances with
    `instances_from_records`, its prefab descriptors carry `InstanceRecord`s, its placements are
    `Rigid`s, and its coverage reads `Instance` and `InstanceKind`;
  - the debug building viewer and interior bench in `apps/website/frontend/src/v2/apps/debug/`,
    which assemble a compound from `<slug>.instances.json` and open and close its doors;
  - the blueprint tooling in `tools_v2/developer-tools/src/blueprint/`, where
    `cargo xtask map bvh-batch` writes `<slug>.instances.json`, and the world line-of-sight checks
    in `tools_v2/developer-tools/src/map_verification/tests/world_line_of_sight.rs`.
- Rules: the folder compiles only with the `io` feature; assembly is all or nothing and `append`
  keeps the instances already placed (`assemble_is_atomic_and_append_adds_scene_trees` in
  `apps/website/map-engine/src/spatial/los/interior/tests/walker.rs`); a door fraction clamps to
  0–1 and the initial angle sets the starting state
  (`door_state_fraction_toggle_and_initial_angle`); a sliding leaf moves along its local X
  (`sliding_leaf_translates_along_its_local_x`); a flattened mesh tags every triangle with its
  owner (`flatten_bakes_instances_with_owners_and_owned_cuts_tag_the_leaf`); a transform
  composed with its inverse is the identity, a quaternion round-trips, and a prop
  nested in a building at world coordinates maps back within a micrometre
  (`rot_y_turns_x_toward_z_and_inverse_undoes`, `quaternion_round_trips_and_matches_euler_axes`,
  `nested_composition_keeps_sub_micrometre_precision` in `tests/transform_tests.rs`).

## Related documentation

- [Building instances schema](/contracts_v2/definitions/building-instances.schema.json) — the
  instances JSON.
