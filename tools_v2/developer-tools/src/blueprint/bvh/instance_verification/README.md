# Socket instance verification

The matching and the command behind `instance_verification.rs` in
`tools_v2/developer-tools/src/blueprint/bvh/`: `cargo xtask map instances-verify` checks the
instances file that `bvh-batch` placed from a model's sockets against a
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) recon dump of the same building's live entity
tree.

## Contents

```text
tools_v2/developer-tools/src/blueprint/bvh/instance_verification/
└── instance_group.rs  grouping, matching, the door and pivot checks, and `run_instances_verify`
```

## How it works

`run_instances_verify` loads `--instances <slug>.instances.json` and `--recon <slug>_children.json`.
`architectural` keeps the instances placed from an XOB socket and skips those that descend from a
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
exits 0 when every instance matches within the parent's `POS_TOL_M` (2 cm) and `YAW_TOL_DEG` (1°)
and the world-row check passes, and 1 otherwise.

## Boundaries

- Depends on: the parent's `ReconFile`, `Group`, `Match` and `Report`; the world-row check in
  `tools_v2/developer-tools/src/blueprint/bvh/world_instances.rs`;
  `website_map_engine::world::architecture::compound` (`InstancesFile`, `InstanceRecord`,
  `InstanceKind`, `PlacementSource`, `Rigid`).
- Used by: `instance_verification.rs`, which re-exports `run_instances_verify`, `load`, `verify`
  and `wrap_deg`; `cargo xtask map instances-verify`, through
  `developer_tools::blueprint::run_instances_verify`; the tests in
  `tools_v2/developer-tools/src/blueprint/tests/verify/tests.rs` and
  `tools_v2/developer-tools/src/blueprint/tests/world_row/tests.rs`, which call `load` and `verify`.
- Rules: the handedness is measured, never assumed; the committed
  `assets_v2/terrains/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.instances.json` matches all
  88 children of the recon fixture
  `tools_v2/developer-tools/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_children.json`
  (`farmhouse_sockets_match_the_workbench_recon`), and furniture descendants are skipped
  (`matches_through_the_building_yaw_and_skips_furniture_descendants`), both in
  `tools_v2/developer-tools/src/blueprint/tests/verify/tests.rs`.
