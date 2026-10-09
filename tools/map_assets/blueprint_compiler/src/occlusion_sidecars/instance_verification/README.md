# Socket instance verification

The matching and the command behind `instance_verification.rs` in
`tools/map_assets/blueprint_compiler/src/occlusion_sidecars/`: `cargo xtask map instances-verify` checks the
instances file that `bvh-batch` placed from a model's sockets against a
[Workbench](/documentation/glossary/n_to_z.md#workbench) recon dump of the same building's live entity
tree.

## Contents

```text
tools/map_assets/blueprint_compiler/src/occlusion_sidecars/instance_verification/
└── instance_matching.rs  grouping, matching, the door and pivot checks, and `run_instance_verification`
```

## How it works

`run_instance_verification` loads `--instances <slug>.instances.json` and `--recon <slug>_children.json`.
`architectural_instances` keeps the instances placed from an XOB socket and skips those that descend from a
furniture instance, since the world places the furniture composition beside the building, where
the recon cannot see it. Each instance joins a group by its kind, and each recon child by its class
and components; within a group, instances pair with children by nearest position in the building's
local frame. `verify` runs the match under both yaw signs and keeps the one with the smaller total
position error. When the recon carries them, `enrichment_checks` also compares each door leaf's
hinge with its `DoorRecord`, each child's `pivotId` with the instance id, and each child's origin
in its parent's frame.

The command prints each group's worst position and yaw error, every enrichment mismatch and every
unmatched instance. `--world-row --chunk <cx_cy.json.gz> --prefabs <prefabs.json.gz>` then places
every matched child through the committed chunk row (`world_instances.rs` in the parent folder). It
exits 0 when every instance matches within the parent's `POSITION_TOLERANCE_METERS` (2 cm) and `YAW_TOLERANCE_DEGREES` (1°)
and the world-row check passes, and 1 otherwise.

## Boundaries

- Depends on: the parent's `ReconstructionFile`, `ArchitecturalGroup`, `InstanceMatch` and `InstanceVerificationReport`; the world-row check in
  `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/world_instances.rs`;
  `building_interiors::compound` (`InstancesFile`, `InstanceRecord`, `InstanceKind`,
  `PlacementSource`) and `geometry_primitives::rigid_transform::Rigid`.
- Used by: `instance_verification.rs`, which re-exports `run_instance_verification`, `load`, `verify`
  and `wrap_degrees`; `cargo xtask map instances-verify`, through
  `blueprint_compiler::run_instance_verification`; the tests in
  `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/tests/instance_verification_tests.rs` and
  `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/tests/world_instances_tests.rs`, which call `load` and `verify`.
- Rules: the handedness is measured, never assumed; the committed
  `assets/terrains/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.instances.json` matches all
  88 children of the recon fixture
  `tools/map_assets/blueprint_compiler/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_children.json`
  (`farmhouse_sockets_match_the_workbench_recon`), and furniture descendants are skipped
  (`matches_through_the_building_yaw_and_skips_furniture_descendants`), both in
  `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/tests/instance_verification_tests.rs`.
