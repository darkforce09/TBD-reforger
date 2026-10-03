# Prefab catalogue

The `prefab_catalog` crate: a terrain's prefab catalogue as the map reads it. Every streamed object
is classified by its prefab's row: the render class it is drawn and picked as, the building or
fence footprint it is outlined with, and whether it is oversized. The crate also decodes every
served world payload, gzip or plain JSON, and names the one error a world parse answers with.

## Contents

```text
crates/world_formats/prefab_catalog/
├── Cargo.toml  the package: `world_file_formats`, `serde_json`, `flate2`, `rkyv`, the dev-only `test_fixtures` feature, tier 2
└── src/        the prefab rows, render classes, footprint lookups, prefab tables and payload decoding
```

## How it works

The catalogue is `objects/prefabs.json.gz` or its archive `objects/prefabs.rkyv`, in a terrain's
asset folder such as `assets/terrains/everon/`. `narrow_prefab_rows` keeps each row with a
numeric `prefabId` and a string `kind` (a missing `class` reads `unknown`) as a `PrefabRow` whose
id is a `PrefabId` (u32); a numeric `prefabId` that is not a whole number in `0..=u32::MAX`
refuses the catalogue with `InvalidPrefabId` (`WorldError::InvalidPrefabId` on the byte lane), never
a cast. `build_prefab_maps` gives each row its render-class code (`render_class_for_prefab`), keyed
by its `PrefabId`; a chunk's numeric `pid` finds it through `prefab_id_from_f64`, which names a
prefab only for the exact `f64` of a `u32` (a fractional or negative `pid`, `-0.0` included, names
none), and `build_prefab_maps` also reports whether any classified prefab has a
half extent of `OVERSIZED_HALF_EXTENT_M` (64 m) or more. `catalog_from_bytes` yields the same pair
from the archive after validating it and checking its schema version, its terrain and that its
census counts every row; `row_to_archive` writes a row and refuses one it cannot encode
faithfully. The census, `objects/type-inventory.rkyv`, rides inside the catalogue and also ships
alone (`inventory_to_archive`, `inventory_from_bytes`).

`building_prefab_lookup` keeps the catalogue's buildings, and its water-kind piers and docks, with
their half extents (2 m when missing) and the zoom from which a landmark glyph may mark them;
`fence_prefab_lookup` keeps its fence props. `obb_corners(x, y, half_x, half_y, rotation_deg)`
turns a footprint into its four corners, 0° facing north and turning clockwise.

`tables_from_bytes` tells the two served forms apart by their first bytes: an empty buffer is
`WorldError::EmptyPayload`, `1f 8b` is gzip JSON (`bytes_to_json`, then `tables_from_json`), and
anything else goes to the validating rkyv reader (`catalog_from_bytes`, then
`tables_from_catalog`). Both lanes give the same `PrefabTables`: the prefab map, the oversized flag
and the two footprint lookups. A catalogue built for another terrain, or one that repeats a prefab
id, is refused.

## Getting started

Run from the repository root:

```bash
cargo test -p prefab_catalog   # the rows, classes, lookups, tables and census kinds over the Everon export
```

The tests read the committed Everon catalogue (`assets/terrains/everon/objects/prefabs.json.gz`
and `type-inventory.json`) and build the archive form from it; the census kind tests read
`contracts/definitions/map-object-enums.schema.json`.

## Configuration

One feature, `test_fixtures`, off by default: it compiles the `test_fixtures` module (the Everon
catalogue in its gzip JSON and archive forms, and `gzip`) for the tests of other crates and is
enabled only from their `[dev-dependencies]`. The crate reads no environment variable.

## Public surface

- `prefab_rows`: `PrefabRow`, `PrefabEntry`, `PrefabCatalog`, `narrow_prefab_rows`, `build_prefab_maps`, `catalog_from_bytes`, `from_archive`, `rows_from_archive`,
  `row_to_archive`, `inventory_to_archive`, `inventory_from_bytes`, `PREFAB_CATALOG_ALIGN`.
- `numeric_prefab_ids`: `prefab_id_from_f64`.
- `render_classes`: `RENDER_CLASS_CODES`, `class_code`, `NO_CLASS`, `render_class_for_prefab`,
  `OVERSIZED_HALF_EXTENT_M`, `narrow_instance_row`, `narrow_instance_row_v2`, `InstanceRowV2`.
- `footprint_lookups`: `obb_corners`, `building_prefab_lookup`, `fence_prefab_lookup`,
  `BuildingPrefabInfo`, `FencePrefabInfo`.
- `prefab_tables`: `PrefabTables`, `tables_from_json`, `tables_from_catalog`, `tables_from_bytes`.
- `world_payload`: `WorldError`, `bytes_to_json`.
- `instance_kinds`: `INSTANCE_KINDS`, the census buckets (also in `prelude`).
- `Error`, `Result` and `InvalidPrefabId` (the row the JSON narrowing refuses) at the crate root; the common names in `prelude`; `test_fixtures` under
  `cfg(test)` or the `test_fixtures` feature.

## Boundaries

- Depends on: `world_file_formats` (the catalogue and census archives, `access_checked`,
  `PrefabId`, `TerrainId`), `serde_json`, `flate2`, `rkyv`, `thiserror`.
- Used by: `world_chunks`, `world_store`, `road_network`, `vegetation` and
  `world_line_of_sight`; the streaming crates `chunk_scheduler` and `chunk_draw_buffers`; the
  world export (`tools/map_assets/world_export_pipeline`), which
  mints its census buckets from `INSTANCE_KINDS`; the type-inventory gate of
  `tools/commands/schema_tooling`, which sums them; the checks in `tools/developer_tools/src/`.
- Rules:
  - world formats category, tier 2 (`cargo xtask verify crate-tiers`);
  - the archive decodes to exactly the rows the JSON gives
    (`everon_catalogue_archive_equals_the_json_rows`), the committed Everon `prefabs.rkyv` is
    byte-for-byte the archive rebuilt from the JSON and survives a decode and re-encode
    (`the_committed_everon_archive_round_trips_byte_for_byte`), and the two lanes give equal tables
    (`tables_from_bytes_sniffs_gzip_versus_rkyv`);
  - a `prefabId` that is fractional, negative or above `u32::MAX` is a typed error
    (`narrowing_refuses_a_prefab_id_that_is_not_a_whole_u32`,
    `a_json_catalogue_with_a_fractional_prefab_id_is_refused`);
  - a chunk `pid` names a prefab only when it is the exact `f64` of a `u32`
    (`prefab_id_from_f64_names_a_prefab_only_for_the_exact_f64_of_a_u32`);
  - a catalogue built for another terrain, of another schema version, with a drifted class code,
    with a census that does not count its rows or with a repeated prefab id is refused
    (`a_catalogue_for_another_terrain_is_refused`,
    `wrong_schema_version_is_refused_even_though_the_bytes_validate`,
    `a_drifted_class_code_is_refused`, `a_census_that_does_not_match_the_row_count_is_refused`,
    `a_catalogue_with_a_duplicate_prefab_id_is_refused`);
  - `INSTANCE_KINDS` is the enums schema's `kind` set minus its `regionKind` set, `road` last
    (`instance_kinds_match_enums_schema`, `instance_kinds_keep_the_emitted_by_kind_order`);
  - the render class order is a wire format and never changes (`class_codes_match_wire_order`),
    and a kind the table does not map is never drawn (`render_class_truth_table`);
  - a rotation of 360° equals 0° and keeps the footprint's area
    (`obb_360_equals_0_and_area_invariant`).

## Related documentation

- [Map object prefab schema](/contracts/definitions/map-object-prefab.schema.json) — the rows
  of the prefab catalogue.
