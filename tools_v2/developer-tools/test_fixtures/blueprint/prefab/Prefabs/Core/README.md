# Blueprint prefab fixture bases

The root prefabs that the synthetic building, prop and furniture prefabs inherit from, written in
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) prefab text for the blueprint compiler's
prefab resolver and prefab walker tests.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/Core/
├── Building_Base.et   the building root: `SCR_DestructibleBuildingEntity`, placeholder mesh, static body
├── Furniture_base.et  the empty `GenericEntity` root the furniture composition inherits
└── Prop_Base.et       the prop root: an empty `MeshObject` and an enabled `RplComponent`
```

## Format

- Encoding: ASCII Enfusion prefab text, one root entity per file, with invented GUIDs and
  resource paths; nothing here is game content.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads; each file is a
  class head with no base, so it ends an inheritance chain.
- Adding a file: give it an ID and GUIDs no other fixture uses, reference it from a sibling folder
  as `"{GUID}Prefabs/Core/<name>.et"`, and run `cargo test -p developer-tools blueprint::prefab`.

## Producers and consumers

- Producers: people; the files are written by hand.
- Consumers:
  - `resolver_walks_inheritance_sockets_and_children` in
    `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs` resolves
    `Prefabs/Houses/House_Wood.et` and asserts that its chain ends at `Building_Base.et` and that
    this file's placeholder mesh `Common/Models/Default.xob` never wins over the house's own mesh;
  - `walker_places_door_set_window_and_furniture_from_fixtures` in
    `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs` walks the house, whose table and
    chairs inherit `Prop_Base.et` and whose furniture composition inherits `Furniture_base.et`.

## Boundaries

- Depends on: the `.et` grammar `parse_et` reads.
- Used by: the two tests above, through the prefab folder root
  `tools_v2/developer-tools/test_fixtures/blueprint/prefab/`.
- Rules: the resolver test pins the chain `Prefabs/Houses/House_Wood.et` →
  `Prefabs/Houses/House_Base.et` → `Building_Base.et` and the mesh override; a rename updates every
  `"{GUID}Prefabs/Core/…"` base reference in the sibling folders in the same change.
