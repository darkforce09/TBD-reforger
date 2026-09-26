# Blueprint prefab fixtures

The root of a loose-file prefab source for the blueprint compiler's prefab tests: synthetic
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) `.et` prefabs in the grammar's real shape, laid
out so their resource paths (`Prefabs/…`) resolve against this folder. Nothing here is game
content; the IDs, GUIDs and paths are invented.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/
└── Prefabs/  the synthetic house, doors, windows, props and furniture, grouped by kind
```

## How it works

The tests reach this folder as `fixture("prefab")`, the helper in
`tools_v2/developer-tools/src/blueprint/tests/module/tests.rs` that joins the checkout root to
`tools_v2/developer-tools/test_fixtures/blueprint/`. The resolver test serves it through
`DirSource` (`tools_v2/developer-tools/src/enfusion_pak/loose_source.rs`), which stands in for the
game paks and matches each path component case-insensitively. The walker test copies the whole
folder into a temporary directory, writes synthetic `.xob` models at the mesh paths the prefabs
name, and walks the house from there, so the fixtures themselves hold no models.

## Format

- Encoding: ASCII Enfusion prefab text, one root entity per `.et` file, under `Prefabs/<kind>/`.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads; the
  [synthetic prefab tree README](/tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/README.md)
  lists the shapes the files exercise.
- Adding a file: add it under `Prefabs/`, as that README describes.

## Producers and consumers

- Producers: people; the files are written by hand.
- Consumers: `resolver_walks_inheritance_sockets_and_children`
  (`tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs`),
  `walker_places_door_set_window_and_furniture_from_fixtures`
  (`tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs`) and
  `compiler_fixtures_resolve_from_root_crate_and_source_directory`
  (`tools_v2/developer-tools/src/tests/repository_paths.rs`).

## Boundaries

- Depends on: the `.et` grammar and the `PrefabResolver` and `Walker` in
  `tools_v2/developer-tools/src/blueprint/bvh/`.
- Used by: the three tests above.
- Rules: the tests address the folder as `prefab` under the blueprint fixture root, so a move
  updates the three test files above in the same change; the walker test copies every file here,
  this README included, into its temporary directory.

## Related documentation

- [Prefab text tokenizer](/tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/README.md) —
  the `.et` tokenizer and block shapes.
- [Occlusion sidecars and prefab placement](/tools_v2/developer-tools/src/blueprint/bvh/README.md) —
  the prefab walk these fixtures exercise.
