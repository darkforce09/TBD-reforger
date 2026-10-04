# Terrain loaders

The browser loaders of the ground itself: the elevation model's raw grid, the relief's vector
grid with its contour and sea lanes, the satellite image and the cartographic map tiles, and the
water files. The data models are the terrain crates under `crates/terrain/`
([`terrain_elevation`](/crates/terrain/terrain_elevation/README.md),
[`terrain_relief`](/crates/terrain/terrain_relief/README.md),
[`satellite_imagery`](/crates/terrain/satellite_imagery/README.md),
[`water_bodies`](/crates/terrain/water_bodies/README.md)), which every caller imports directly.

## Contents

```text
crates/streaming/map_asset_loading/src/terrain/
├── elevation/           the raw elevation grid's browser loader
├── mod.rs               the module tree
├── relief/              `DemVectors`: the vector grid and its contour and sea lanes
├── satellite_quadtree/  the satellite preview and full mip chain, and the map tiles
└── water/               `WaterHost`: the water files' browser loader
```

## How it works

Every child reads files of one terrain's asset folder, which the terrain's `manifest.json` names
and the API serves under `/map-assets/<terrain>/`:

| Child | Files it reads (Everon, `assets/terrains/everon/`) |
|---|---|
| `elevation/` | a raw `dem/elevation.dem` when the manifest declares one (the PNG is the map host's) |
| `relief/` | none: it draws from the elevation model's metres cache and vector grid |
| `satellite_quadtree/` | `satellite/everon-sat.tbd-sat`, and the map tiles under `tiles/map/` for the map view |
| `water/` | `water/bathymetry.tbd-bath` and `water/water_vectors.rkyv`, when declared; Everon declares none |

The map host of `map_streaming_host` drives the loads at boot: the elevation model and the
satellite image load side by side, the elevation model becomes the hillshade texture, after which
`DemVectors` builds the 8 m vector grid that the contours, the sea band, the airfield apron and the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s height readout sample, and
the satellite image fills the basemap texture. Positions are world metres inside the manifest's
`worldBounds` (0 to 12 800 m on both axes for Everon). Every loader compiles only for wasm32.

## Public surface

- `elevation::loader`: `raw_block_is_readable`, `load_declared_raw`, `load_dem_raw`.
- `relief::dem_vectors::DemVectors`.
- `satellite_quadtree`: `load_satellite`, `load_map_basemap`, `show_satellite_basemap`,
  `sat_preview_only`.
- `water::loader::WaterHost`.

## Boundaries

- Depends on: the terrain crates; `world_chunks::terrain_manifest` (the manifest's blocks);
  `world_file_formats` (the `TBDB` header); `map_streaming_model` (boot progress, the asset sink);
  `crate::asset_statistics`, `crate::live_memory_budget`, `crate::mesh_composition`;
  `browser_platform` (fetches and Range fetches); `map_draw_lanes` (zoom gates and lanes).
- Used by: the map host of `map_streaming_host`, which loads, holds and syncs everything here;
  the Mission Creator's tests in `crates/frontend/workspaces/mission_creator_workspace/src/`, which read the satellite
  and relief code by path.
- Rules: a binary file is validated before it is read, and one of another schema or container
  version is refused rather than guessed; a manifest block this build cannot read (another
  encoding, a missing path) is skipped, never half-read.

## Related documentation

- [Everon dataset](/assets/terrains/everon/README.md) — the terrain files these loaders read.
