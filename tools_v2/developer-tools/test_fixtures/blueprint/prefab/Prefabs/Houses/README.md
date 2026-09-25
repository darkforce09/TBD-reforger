# Blueprint house prefab fixtures

The synthetic building the blueprint compiler's prefab tests start from: a base house that carries
the mesh, the socket mappings and the architectural children, and a variant that adds a furniture
composition, written in [Enfusion](/documentation_v2/glossary.md#enfusion) prefab text.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/Houses/
├── House_Base.et  the building: mesh, interior query box, socket mappings, door set, two windows, a probe
└── House_Wood.et  the variant the tests resolve: a material override and the furniture composition `F1`
```

## Format

- Encoding: ASCII Enfusion prefab text with invented GUIDs and resource paths; nothing here is game
  content.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads. `House_Base.et`
  inherits `Prefabs/Core/Building_Base.et`, sets the mesh `Assets/Houses/House.xob`, maps the bone
  prefixes `socket_door_left` and `socket_win` to the door set and the window in
  `SlotBoneMappings`, and places a door set on `socket_door_left_01`, a `$grp` of two windows on
  `socket_win_01` and `socket_win_02`, and a `Prefabs/Core/Probe.et` probe that the tree does not
  hold. `House_Wood.et` inherits it and adds `Prefabs/Furniture/Furniture_01.et` as child `F1`.
- Adding a file: keep IDs and GUIDs unique and add its assertions to the resolver test.

## Producers and consumers

- Producers: people; the files are written by hand.
- Consumers:
  - `resolver_walks_inheritance_sockets_and_children` in
    `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs` resolves `House_Wood.et` and
    asserts the class `SCR_DestructibleBuildingEntity`, the chain `House_Base.et` →
    `Prefabs/Core/Building_Base.et`, the mesh from `House_Base.et`, both socket mappings, and the
    children in order: door set, two windows, probe, then the furniture composition;
  - `walker_places_door_set_window_and_furniture_from_fixtures` in
    `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs` walks `House_Wood.et` over
    synthetic models into eleven instances, records the mesh-less probe as a note rather than an
    instance, and validates the result against
    `contracts_v2/definitions/building-instances.schema.json`;
  - `compiler_fixtures_resolve_from_root_crate_and_source_directory` in
    `tools_v2/developer-tools/src/tests/repository_paths.rs` checks that `House_Wood.et` resolves
    from the checkout root, the crate and the blueprint source folder.

## Boundaries

- Depends on: `Prefabs/Core/`, `Prefabs/Doors/`, `Prefabs/Windows/` and `Prefabs/Furniture/`.
- Used by: the three tests above.
- Rules: the socket names match the synthetic house model the walker test builds, and the child
  order is pinned by the resolver test; the path-resolution test names `House_Wood.et` by path, so
  a rename updates it in the same change.
