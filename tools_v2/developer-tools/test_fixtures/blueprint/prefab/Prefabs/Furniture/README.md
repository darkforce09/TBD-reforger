# Blueprint furniture composition fixture

One synthetic [Enfusion](/documentation_v2/glossary.md#enfusion) furniture composition: a table
and a `$grp` group of two chairs, placed by their prefab coordinates rather than by model sockets.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/Furniture/
└── Furniture_01.et  the composition: table `T1` and chairs `C1` and `C2` with coords, angles and a scale
```

## Format

- Encoding: ASCII Enfusion prefab text with invented GUIDs and resource paths; nothing here is game
  content.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads. The file inherits
  `Prefabs/Core/Furniture_base.et` and holds one anonymous child list: a `Prefabs/Props/Table.et`
  entry and a `$grp` block of two `Prefabs/Props/Chair.et` entries, each with an `ID`, `coords`,
  `angles` and, on `C2`, `scale 1.152`.
- Adding a file: keep IDs unique and add its assertions to the resolver test.

## Producers and consumers

- Producers: people; the file is written by hand.
- Consumers:
  - `resolver_walks_inheritance_sockets_and_children` in
    `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs` resolves the file and asserts
    three children, the table first at coords (1.035, 0.28, -7.666) with yaw 91.667°, and the
    third child's angles (88.816, -180, 96.7) and scale 1.152;
  - `walker_places_door_set_window_and_furniture_from_fixtures` in
    `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs` walks it as the house's child
    `F1` and expects the instances `F1/T1`, `F1/C1` and `F1/C2`, of kind `Furniture`, placed from
    prefab coordinates, the two chairs sharing one BLAS.

## Boundaries

- Depends on: `Prefabs/Core/Furniture_base.et` and the props in `Prefabs/Props/`.
- Used by: the two tests above; `Prefabs/Houses/House_Wood.et` places it as `F1`.
- Rules: the coordinates, angles and scale are pinned by both tests; a change here updates them in
  the same change.
