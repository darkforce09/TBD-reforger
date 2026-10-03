# World store source

The source of `world_store`: the store, its error and the crate root that declares them.

## Contents

```text
crates/world_formats/world_store/src/
├── error.rs    `Error` and `Result`: a refused payload or road network archive
├── lib.rs      the crate root: module header, `mod` lines and re-exports
├── prelude.rs  the names most readers import
├── store.rs    `WorldStore`: manifest, prefab table, roads, regions and one chunk at a time
└── tests/      unit tests for the store's loads and the Everon census
```

## Boundaries

- Depends on: `world_chunks`, `prefab_catalog`, `road_network`, `vegetation`,
  `world_file_formats`, `map_coordinates`, `serde_json` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
