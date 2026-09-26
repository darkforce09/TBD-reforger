# Blueprint door prefab fixtures

Synthetic [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) door prefabs: a door frame with a
child leaf, and three leaf variants that each exercise one way the prefab resolver reads a door.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/Doors/
├── Door_Base.et      the rotating-door base: `DoorComponent` with `WholeEntity` animation, kinematic body
├── Door_Leaf.et      a leaf that sets `AngleRange -120`, `ClosedAngle 0` and its own mesh over the base
├── Door_Plain.et     a leaf that sets only its mesh, so its door comes from the base chain
├── Door_Sliding.et   a barn door: `DoorComponent Enabled 0` and a `SlidingDoorComponent` opening 2.05 m
├── DoorSet.et        the door frame, whose child `Door_Leaf.et` hangs on the `socket_door_LEFT` pivot
└── DoorSet_Base.et   the frame base: placeholder mesh, static rigid body, hierarchy
```

## How it works

`Prefabs/Houses/House_Base.et` maps the bone prefix `socket_door_left` to `DoorSet.et` and places
one door set on the pivot `socket_door_left_01`. `DoorSet.et` inherits `DoorSet_Base.et`, sets the
frame mesh `Assets/Doors/DoorFrame.xob` and adds one `Door_Leaf.et` child on the frame's
`socket_door_LEFT` pivot. The three leaves inherit `Door_Base.et`, which carries the
`DoorComponent` without an angle range.

The resolver test (`resolver_walks_inheritance_sockets_and_children`) reads each variant:

| File | What the resolver must return |
|---|---|
| `DoorSet.et` | mesh `Assets/Doors/DoorFrame.xob`; one child, `Door_Leaf.et` on `socket_door_LEFT` |
| `Door_Leaf.et` | mesh `Assets/Doors/Door_Leaf.xob`; a rotating door, range -120° marked explicit, closed angle 0; no sliding door |
| `Door_Plain.et` | a rotating door from the base chain, range `DEFAULT_ANGLE_RANGE_DEG`, not explicit |
| `Door_Sliding.et` | no rotating door, since `Enabled 0` drops it; a sliding door opening 2.05 m |

The walker test places the frame on the house socket as a `DoorFrame` instance and the leaf
0.5 m along the frame's socket as a `DoorLeaf` instance with range -120° and the BLAS
`blas/Door_Leaf.bvh`.

## Format

- Encoding: ASCII Enfusion prefab text, one root entity per file, with invented GUIDs and
  resource paths; nothing here is game content.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads; the door fields
  are those the resolver's door and sliding-door parameters take (`AngleRange`, `ClosedAngle`,
  `InitialAngle`, `Enabled`, `OpenedDistance`).
- Adding a file: inherit `Door_Base.et` for a rotating leaf, keep IDs and GUIDs unique, and add its
  assertion to the resolver test.

## Producers and consumers

- Producers: people; the files are written by hand.
- Consumers: `resolver_walks_inheritance_sockets_and_children` in
  `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs`, and
  `walker_places_door_set_window_and_furniture_from_fixtures` in
  `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs`, which walks `DoorSet.et` and
  `Door_Leaf.et` through the house.

## Boundaries

- Depends on: the `.et` grammar `parse_et` reads; `Prefabs/Houses/House_Base.et`, which places the
  door set.
- Used by: the two tests above.
- Rules: the pivot names `socket_door_LEFT` and `socket_door_left_01` match the synthetic model
  sockets the walker test builds, and the table above is pinned by the resolver test; a change to a
  door field updates that test in the same change.
