# Prefab catalogue source

The source of `prefab_catalog`: one module per concern of the prefab catalogue, the payload
decoding, the crate's error and prelude, and the Everon fixtures other crates' tests share.

## Contents

```text
crates/world_formats/prefab_catalog/src/
├── error.rs               `Error` and `Result` (the payload and archive errors, wrapped) and `InvalidPrefabId`
├── footprint_lookups.rs   the footprint ring and the building and fence lookups
├── lib.rs                 the crate root: module header, `mod` lines and the re-exports
├── numeric_prefab_ids.rs  the prefab a catalogue `prefabId` or a chunk `pid` number names
├── prefab_rows.rs         the prefab rows, the prefab map and the catalogue and census archives
├── prefab_tables.rs       the tables one prefab load decides, from either served form
├── prelude.rs             the common names for glob import
├── render_classes.rs      render classes, their wire codes and the instance row narrowing
├── test_fixtures.rs       the Everon catalogue in both served forms, behind `cfg(test)` or `test_fixtures`
├── tests/                 unit tests of every module
└── world_payload.rs       `bytes_to_json` and `WorldError`: gzip-or-plain JSON payload decoding
```

## How it works

`render_classes`, `world_payload` and `numeric_prefab_ids` are the bottom; `prefab_rows` builds on
the render classes and the numeric ids, `footprint_lookups` on the numeric ids, and `prefab_tables` joins all of them into one load's tables.

## Boundaries

- Depends on: `world_file_formats`, `serde_json`, `flate2`, `rkyv`, `thiserror`.
- Used by: `world_chunks`, the map engine and the developer tools, through the crate root.
- Rules: `lib.rs` holds only the module header, `mod` lines and `pub use` lines
  (`cargo xtask verify crate-anatomy`).
