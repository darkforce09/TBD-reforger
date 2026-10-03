# World environment

The map engine's half of what stands on the terrain: the browser loaders and GPU belts that show
the buildings, the forest mass and the cartographic labels on the 2D map. The prefab catalogue
and render classes, the vegetation data and the place names are crates
([`prefab_catalog`](/crates/world_formats/prefab_catalog/README.md),
[`vegetation`](/crates/world_objects/vegetation/README.md),
[`place_names`](/crates/world_objects/place_names/README.md)) every caller imports directly.

## Contents

```text
legacy/map_engine/src/world/environment/
├── buildings/   the building, outline and fence lane uploads
├── locations/   the browser label loader: town names, road names and spot heights
├── mod.rs       the module tree: `buildings`, `locations`, `vegetation`
└── vegetation/  the forest mass loader and its density lane
```

## How it works

Every placed object has one render class, decided by its prefab's catalogue kind
(`render_class_for_prefab` in `prefab_catalog::render_classes`):

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
calls into the children: `buildings/` uploads the building footprints, `vegetation/` loads the
density bins and draws the forest mass, and `locations/` loads the label sources and uploads the
label lanes that `place_names` places and declutters. Every loader and belt in the children
compiles only for wasm32 with the `render` feature; the plain computation the native tools reuse
lives in the crates.

## Public surface

- `buildings::buffers`: the building, outline and fence upload methods on `RenderEngine`.
- `vegetation::loader::ForestMassHost` and the forest density lane in `vegetation::buffers`.
- `locations::loader::LabelHost`.

## Boundaries

- Depends on: `vegetation`, `place_names`, `label_layout`, `road_network`, `terrain_elevation`
  and `world_chunks`; `world_file_formats::density` (the density bins); `map_draw_lanes` (lanes
  and zoom gates); `crate::streaming` (boot progress, statistics, host state);
  `crate::world::terrain`, `crate::world::mesh` and `crate::world::scene`; `browser_platform`
  for fetch; `crate::frame`, `render_primitives` and `wgpu` for the uploads.
- Used by: `crate::streaming`, which loads, holds and uploads everything here.
- Rules: the render class order is a wire format and never changes, and a kind the table does not
  map is never drawn (both pinned in `prefab_catalog`).
