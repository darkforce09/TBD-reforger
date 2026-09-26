# Blueprint prop prefab fixtures

The two synthetic [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) props that the furniture
composition places: a table and a chair.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/Props/
├── Chair.et  a chair: `Prop_Base.et` with the mesh `Assets/Props/Chair.xob`
└── Table.et  a table: `Prop_Base.et` with the mesh `Assets/Props/Table.xob` and a static rigid body
```

## Format

- Encoding: ASCII Enfusion prefab text with invented GUIDs and resource paths; nothing here is game
  content.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads; both files
  inherit `Prefabs/Core/Prop_Base.et` and set only a mesh, plus a rigid body on the table.
- Adding a file: inherit `Prefabs/Core/Prop_Base.et`, keep IDs and GUIDs unique, and place it from
  `Prefabs/Furniture/Furniture_01.et`.

## Producers and consumers

- Producers: people; the files are written by hand.
- Consumers: `walker_places_door_set_window_and_furniture_from_fixtures` in
  `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs`, which writes a synthetic model at
  each mesh path and expects the table as BLAS `blas/Table.bvh` with low cover and both chairs
  sharing `blas/Chair.bvh`; `resolver_walks_inheritance_sockets_and_children` in
  `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs` reads their placements through the
  furniture composition.

## Boundaries

- Depends on: `Prefabs/Core/Prop_Base.et`.
- Used by: `Prefabs/Furniture/Furniture_01.et` and the two tests above.
- Rules: the mesh paths match the model paths the walker test writes; a change here updates that
  test in the same change.
