# World environment

What stands on the terrain and how the 2D map shows it: the prefab catalogue and the buildings,
the vegetation and land cover, the cartographic labels, and the render class every placed object
of the streamed world is sorted into.

## Contents

```text
apps/website/map-engine/src/world/environment/
├── buildings/   the prefab catalogue, building footprints and the building, outline and fence lanes
├── classify.rs  render classes of placed objects, and the reader of a chunk's JSON instance rows
├── locations/   town names, road names and spot heights: sources, placement, declutter, upload
├── mod.rs       the module tree
├── tests/       unit tests for the render classes and the instance rows
└── vegetation/  the forest mass, tree counts for the glyph budget, and the land-cover regions
```

## How it works

Every placed object has one render class, decided by its prefab's catalogue kind
(`render_class_for_prefab`):

| Catalogue kind | Render class | Code |
|---|---|---|
| `building`, and `water` of class `pier` or `dock` | `building` | 0 |
| `tree` | `tree` | 1 |
| `vegetation` | `vegetation` | 2 |
| `prop`, `utility`, `vehicle` | `prop` | 3 |
| `rock` | `rockLarge` | 4 |
| any other | none (`NO_CLASS`): never drawn or picked | 255 |

The code is the class's index in `RENDER_CLASS_CODES`, the byte a chunk row carries on the wire.
`narrow_instance_row_v2` reads a chunk's JSON instance row `[prefabId, x, y, z, yaw, pitch, roll,
scale]`, where everything after `y` may be missing and takes its identity value (a scale that is
not positive counts as 1).

`crate::streaming` fetches the terrain's catalogue, chunks, density bins, regions and labels and
calls into the children: `buildings/` classifies the catalogue and draws the building footprints,
`vegetation/` builds the forest mass and counts trees against the glyph budget, and `locations/`
places and declutters the labels. The parts that upload to the GPU or fetch in the browser compile
only for wasm32 with the `render` feature; the rest is plain computation the native tools reuse.

## Public surface

- `classify`: `RENDER_CLASS_CODES`, `class_code`, `NO_CLASS`, `render_class_for_prefab`,
  `OVERSIZED_HALF_EXTENT_M` and `narrow_instance_row_v2`.
- `buildings::prefab`: the catalogue rows and maps and their archive; `buildings::obb`: the
  footprint corners and lookups.
- `vegetation::loader::ForestMassHost`, `vegetation::canopy` counts, `vegetation::regions`.
- `locations::loader::LabelHost`, and the label placement, declutter and archive functions.

## Boundaries

- Depends on: `crate::io` (the catalogue, regions and labels archives, the density bins);
  `crate::overlay` (level-of-detail gates, lanes, text packers); `crate::streaming` (chunks,
  residency, fetch); `crate::world::terrain` (the elevation model and roads), `crate::world::mesh`
  and `crate::world::scene`; `crate::frame` and `wgpu` for the uploads.
- Used by:
  - `crate::streaming`, which loads, holds and uploads everything here;
  - `crate::spatial`: the world object index filters on `NO_CLASS`, and the world occluder takes
    its catalogue from `PrefabRow`;
  - `crate::overlay::symbology`, which packs the labels, and `crate::world::terrain::roads` and
    `crate::world::mesh`;
  - the world export, the map raster pipeline and the map checks in
    `tools_v2/developer-tools/src/`.
- Rules: the render class order is a wire format and never changes
  (`class_codes_match_wire_order` in `tests/classify_tests.rs`); a kind the table does not map is
  never drawn, so a new catalogue kind needs its row (`render_class_truth_table`); a missing or
  non-finite trailer of an instance row takes its identity value
  (`narrow_instance_row_v2_reads_trailers_and_defaults`); `classify.rs` compiles only with the
  `streaming` feature.
