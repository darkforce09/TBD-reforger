# World file identifiers

The identifier types the archive records and the world object row hold: one type per thing an
identifier names, so a prefab's catalogue identifier, a wall's identifier and a road segment's
identifier cannot be mixed up, while every file keeps the bytes it had with bare primitive fields.

## Contents

```text
crates/world_formats/world_file_formats/src/ids/
├── archived_accessors.rs    `archived_string_id_accessors!`: `as_str`, `to_native`, `Display` and `str` equality on an archived string id
├── building_element_ids.rs  `WallId`, `DoorId`, `WindowId`, `StairsId`, `FurnitureId`: the elements of a building level
├── mod.rs                   the module tree, the re-exports and the test module
├── prefab_ids.rs            `PrefabId` (`u32`) and `InstancePrefabId` (`u16`, the object row width) with their conversions
├── terrain_feature_ids.rs   `TerrainId`, `RoadSegmentId`, `ForestRegionId`, `WaterFeatureId`
└── tests/                   unit tests of the conversions, the row width and the JSON form
```

## How it works

Each identifier is declared with the `newtype_ids` macros (`string_id!` over a `String`,
`integer_id!` over an integer), which give it `new`, `as_str` or `get`, `into_inner`, `Display`,
`From` conversions and a transparent serde form. The declaration passes rkyv's `Archive`,
`Serialize` and `Deserialize` derives through, and rkyv archives a one-field struct exactly as its
field, so an archive holding `WallId` has the bytes of one holding `String`. The archived forms
(`ArchivedWallId`, `ArchivedPrefabId`, …) carry the reads a zero-copy reader needs: `as_str` and
`to_native` on a string id, `get` and `to_native` on `ArchivedPrefabId`.

| Identifier | Inner | Held by |
|---|---|---|
| `PrefabId` | `u32` | `PrefabEntry`, `OccluderDescriptor`, `BuildingBlueprint` |
| `InstancePrefabId` | `u16` | `ObjectInstancePod` |
| `TerrainId` | `String` | `TypeInventory` |
| `RoadSegmentId` | `String` | `RoadSegmentArchive` |
| `ForestRegionId` | `String` | `ForestRegion` |
| `WaterFeatureId` | `String` | `WaterBody` (lakes and ponds), `WaterLine` (rivers) |
| `WallId` | `String` | `WallRec`, and `DoorRec` and `WindowRec` as the wall they sit in |
| `DoorId`, `WindowId`, `StairsId`, `FurnitureId` | `String` | `DoorRec`, `WindowRec`, `StairsRec`, `FurnitureRec` |

The catalogue identifier is 32 bits wide in the archives and 16 bits wide in the object row, so
the two are separate types: `PrefabId::from(InstancePrefabId)` always widens, and
`InstancePrefabId::try_from(PrefabId)` fails above `u16::MAX`. `InstancePrefabId` is
`#[repr(transparent)]` and `bytemuck::Pod`, so the row stays a 32-byte `Pod`.

## Boundaries

- Depends on: `newtype_ids` (the declaring macros), `rkyv` and `bytemuck` (the derives the
  declarations pass through).
- Used by:
  - `crate::archives` (`prefabs`, `roads`, `forest`, `water`, `blueprints`) and
    `crate::pod::instance`, whose records hold the identifiers;
  - the developer tools' writers, which construct them, and the map engine's readers, which read
    them through the accessors (`map_engine::io::ids`).
- Rules:
  - an identifier's bytes in rkyv, in the `Pod` row and in JSON are exactly its inner value's
    (`archive_wire_identity_tests.rs` under `../archives/tests/`, the row identity test in
    `../pod/tests/instance_tests.rs`, `identifiers_serialise_as_their_bare_values` in
    `tests/identifier_accessor_tests.rs`);
  - a consumer reads an identifier through its accessor, never through its tuple field.
