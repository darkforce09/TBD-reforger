# Terrain

The ground itself as the map engine loads and draws it in the browser: the elevation model's raw
grid loader, the relief lanes drawn from it (contour lines, the sea band), the satellite image's
loads and texture layers, and the water loader. The data models are the terrain crates under
`crates/terrain/`, which every caller imports directly.

## Contents

```text
legacy/map_engine/src/world/terrain/
├── dem/        the raw elevation grid's browser loader
├── mod.rs      the module tree
├── relief/     the contour and sea lane host
├── satellite/  the satellite image's browser loads and the texture layers
└── water/      the water files' browser loader
```

## How it works

Every child reads files of one terrain's asset folder, which the terrain's `manifest.json` names
and the API serves under `/map-assets/<terrain>/`:

| Child | Files it reads (Everon, `assets/terrains/everon/`) |
|---|---|
| `dem/` | `dem/everon-dem-16bit.png`, or a raw `dem/elevation.dem` when the manifest declares one |
| `relief/` | none: it draws from the elevation model's metres cache and vector grid |
| `satellite/` | `satellite/everon-sat.tbd-sat`, and the map tiles under `tiles/map/` for the map view |
| `water/` | `water/bathymetry.tbd-bath` and `water/water_vectors.rkyv`, when declared; Everon declares none |

`crate::streaming::host` drives the loads at boot. The elevation model and the satellite image
load side by side: the elevation model becomes the hillshade texture, after which
`relief::host::DemVectors` builds the 8 m vector grid that the contours, the sea band, the airfield
apron and the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s height readout
sample, and the satellite image fills the basemap texture. The water files load when the
manifest declares them; the roads (`objects/roads.json.gz` and `roads/road_network.rkyv`) load
with the world's objects through `road_network` in `crate::streaming`. Positions are world metres inside
the manifest's `worldBounds` (0 to 12 800 m on both axes for Everon).

The elevation model, the relief, the satellite container reader, the water data and the road
network are the terrain crates under `crates/terrain/` (`terrain_elevation`, `terrain_relief`,
`satellite_imagery`, `water_bodies`, `road_network`). The module declares itself with the `world`
feature, and every loader and belt in it (`dem/loader.rs`, `relief/host.rs`, `satellite/quadtree/`,
`satellite/textures.rs`, `water/loader.rs`) compiles only for wasm32 with the `render` feature;
the plain computation the native tools reuse lives in the crates.

## Public surface

- `dem`: the raw grid loader.
- `relief`: `DemVectors`.
- `satellite`: `load_satellite`, `load_map_basemap`, `show_satellite_basemap`, and the
  `RenderEngine` texture-layer methods with the `TexLane` bookkeeping.
- `water`: `WaterHost`.

## Boundaries

- Depends on: the terrain crates (`terrain_elevation`, `terrain_relief`, `satellite_imagery`,
  `water_bodies`); `world_chunks::terrain_manifest` (the manifest's blocks);
  `world_file_formats` (the `TBDB` header); `crate::streaming` (boot progress, the statistics
  bridge, the memory budget); `browser_platform` (fetches and Range fetches); `crate::frame` (the
  render engine); `map_draw_lanes` (zoom gates and lanes); `crate::world::mesh` and
  `crate::world::scene`; `render_primitives` (the quad instance layout), `wgpu` and the
  browser's APIs.
- Used by:
  - `crate::streaming`, which loads, holds and uploads everything here;
  - `crate::frame`, which keeps the texture lanes, and `crate::spatial::los::terrain` and
    `crate::world::environment::vegetation`, which build `TexLane`s;
  - the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s tests in
    `apps/frontend/src/workspaces/editor/tests/`, which read the satellite loading code by path.
- Rules: the module compiles only with the `world` feature and each loader and belt only for
  wasm32 with `render`; a
  binary file is validated before it is read, and one of another schema or container version is
  refused rather than guessed, as each child's tests hold; a manifest block this build cannot read
  (another encoding, a missing path) is skipped, never half-read.

## Related documentation

- [Everon dataset](/assets/terrains/everon/README.md) — the terrain files this module reads.
