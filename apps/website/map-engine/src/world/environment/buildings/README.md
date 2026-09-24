# Buildings and the prefab catalogue

The prefab catalogue every streamed object is classified by, and the buildings the 2D map draws
from it: the catalogue's rows and render classes, read from JSON or from its binary archive; each
building's oriented footprint; and the buffers the map draws building fills, outlines and fences
from.

## Contents

```text
apps/website/map-engine/src/world/environment/buildings/
├── buffers.rs    the engine's upload of building fills, outlines and fence strips as draw lanes
├── footprint.rs  the footprint buffers: one oriented quad and outline ring per placed building
├── mod.rs        the module tree
├── obb.rs        oriented footprint corners, and the building and fence lookups of the catalogue
├── prefab.rs     the prefab catalogue: narrowed rows, render classes, its archive and census
└── tests/        unit tests for the footprints and the prefab catalogue
```

## How it works

The catalogue is `objects/prefabs.json.gz` or its archive `objects/prefabs.rkyv`, in a terrain's
asset folder such as `assets_v2/terrains/everon/`. `narrow_prefab_rows` keeps each row with a
numeric `prefabId` and a string `kind` (a missing `class` reads `unknown`) as a `PrefabRow`, and
`build_prefab_maps` gives each its render-class code (`render_class_for_prefab` in
`apps/website/map-engine/src/world/environment/classify.rs`), keyed by the bits of its f64 id so a
chunk's row finds it exactly, and reports whether any classified prefab has a half extent of
`OVERSIZED_HALF_EXTENT_M` (64 m) or more. `catalog_from_bytes` yields the same pair from the
archive after validating it and checking its schema version, its terrain and that its census
counts every row; `row_to_archive` writes a row and refuses one it cannot encode faithfully. The
census, `objects/type-inventory.rkyv`, rides inside the catalogue and also ships alone
(`inventory_to_archive`, `inventory_from_bytes`).

`building_prefab_lookup` keeps the catalogue's buildings, and its water-kind piers and docks, with
their half extents (2 m when missing) and the zoom from which a landmark glyph may mark them;
`fence_prefab_lookup` keeps its fence props. `obb_corners(x, y, half_x, half_y, rotation_deg)`
turns a footprint into its four corners, 0° facing north and turning clockwise.

`WorldResidency::rebuild_buffers` (`footprint.rs`) packs, when the buildings layer is on and the
zoom reaches `BUILDING_MIN_ZOOM`, the building rows of the pinned chunks in chunk-id order: ten
floats per building (centre, half extents, the rotation's cosine and sine, and a colour by class,
the default fill while a landmark glyph marks the building; a bridge becomes a casing and a deck;
piers and docks are left out) and the four edges of each outline. `upload_world_buildings`, `upload_world_building_outlines` and
`upload_world_fence_strips` (`buffers.rs`) turn those buffers into the engine's draw lanes,
positions made relative to the scene anchor.

## Boundaries

- Depends on: `crate::io::archives` (the catalogue and census archives, `access_checked`) and
  `crate::world::environment::classify`; for the buffers, `crate::streaming` (`WorldResidency`,
  the building zoom floor), `crate::frame`, `crate::overlay::lanes`, `crate::world::scene`
  (`ANCHOR`) and `wgpu`.
- Used by:
  - `crate::streaming`: the prefab and chunk loaders and the store read the catalogue; the
    scheduler's residency and queries use the lookups and rows; the scheduler, the toggles and the
    packers rebuild and gate the footprints; the world loader's upload calls the three upload
    methods;
  - `crate::spatial::los::world`, whose occluder takes its proxy boxes and labels from
    `PrefabRow`;
  - the world export and checks in `tools_v2/developer-tools/src/world_export_pipeline/` and
    `tools_v2/developer-tools/src/map_verification/`, which write and compare the catalogue
    archive.
- Rules: the archive decodes to exactly the rows the JSON gives
  (`everon_catalogue_archive_equals_the_json_rows` in `tests/prefab_tests.rs`); a catalogue built
  for another terrain, of another schema version, with a drifted class code or with a census that
  does not count its rows is refused (`a_catalogue_for_another_terrain_is_refused`,
  `wrong_schema_version_is_refused_even_though_the_bytes_validate`,
  `a_drifted_class_code_is_refused`, `a_census_that_does_not_match_the_row_count_is_refused`); a
  rotation of 360° equals 0° and keeps the footprint's area (`obb_360_equals_0_and_area_invariant`
  in `tests/obb_tests.rs`); `buffers.rs` compiles only for wasm32 with the `render` feature, and
  the rest only with `streaming`.

## Related documentation

- [Map object prefab schema](/contracts_v2/definitions/map-object-prefab.schema.json) — the rows
  of the prefab catalogue.
