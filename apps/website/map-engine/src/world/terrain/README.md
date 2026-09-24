# Terrain

The ground itself as the map engine reads and draws it: the elevation model and its sampling, the
relief drawn from it (hillshade, contour lines, the sea band), the road network, the satellite
image and water.

## Contents

```text
apps/website/map-engine/src/world/terrain/
├── dem/        the elevation model: manifest, PNG and raw decoders, sampling, the vector grid
├── mod.rs      the module tree
├── relief/     the hillshade, contour lines with summit rings, and the sea band
├── roads/      road segments and their strips, the airfield apron, and fence, pier and rail strips
├── satellite/  the satellite container reader, its browser loads, and the texture layers
└── water/      the bathymetry mask, the inland water archive, and the sea fill mesh
```

## How it works

Every child reads files of one terrain's asset folder, which the terrain's `manifest.json` names
and the API serves under `/map-assets/<terrain>/`:

| Child | Files it reads (Everon, `assets_v2/terrains/everon/`) |
|---|---|
| `dem/` | `dem/everon-dem-16bit.png`, or a raw `dem/elevation.dem` when the manifest declares one |
| `relief/` | none: it draws from the elevation model's metres cache and vector grid |
| `roads/` | `objects/roads.json.gz` and its archive `roads/road_network.rkyv` |
| `satellite/` | `satellite/everon-sat.tbd-sat`, and the map tiles under `tiles/map/` for the map view |
| `water/` | `water/bathymetry.tbd-bath` and `water/water_vectors.rkyv`, when declared; Everon declares none |

`crate::streaming::host` drives the loads at boot. The elevation model and the satellite image
load side by side: the elevation model becomes the hillshade texture, after which
`relief::host::DemVectors` builds the 8 m vector grid that the contours, the sea band, the airfield
apron and the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s height readout
sample, and the satellite image fills the basemap texture. The roads load with the world's
objects, and the water files when the manifest declares them. Positions are world metres inside
the manifest's `worldBounds` (0 to 12 800 m on both axes for Everon).

The parts that fetch in the browser or upload to the GPU compile only for wasm32 with the `render`
feature; the rest is plain computation that the native tools reuse:

| Gate | Files |
|---|---|
| `world` (the whole module) | `dem/` sampling, grid and PNG decode; `relief/` contours, hillshade, sea band; `roads/` styling and mesh |
| `io` | `dem/raw.rs` |
| `streaming` | `roads/` network, airfield and strips; `satellite/streamer/`; `water/` vectors and mesh |
| wasm32 and `render` | `dem/loader.rs`, `relief/host.rs`, `satellite/quadtree/`, `satellite/textures.rs`, `water/loader.rs` |

## Public surface

- `dem`: `DemManifest`, the sampling functions, the PNG and raw decoders, `DemVectorGrid` and
  `sample_grid_meters`.
- `relief`: `build_hillshade_image`, `DemVectors`, `ContourRing` and `SeaBandGeometry`.
- `roads`: `RoadSegment` and its readers, `compose_roads_mesh`, the airfield and strip composers,
  and `expand_polyline_strip`.
- `satellite`: `load_satellite`, `load_map_basemap`, `show_satellite_basemap`, the container
  reader and the `RenderEngine` texture-layer methods.
- `water`: `WaterHost`, `WaterMask`, `WaterVectors` and `compose_sea_mesh`.

## Boundaries

- Depends on: `crate::io` (the containers and archives); `crate::streaming` (fetches, boot
  progress, the manifest's blocks, the memory budget, chunk math); `crate::frame` (the render
  engine); `crate::overlay` (zoom gates and lanes); `crate::world::mesh`, `crate::world::scene` and
  `crate::world::environment` (road class names, footprint corners); `crate::camera::math`
  (rounding); `crate::spatial::los::terrain`, whose items `dem/sample/` re-exports;
  `website-graphics-engine`, `wgpu`, the `png` crate and the browser's APIs.
- Used by:
  - `crate::streaming`, which loads, holds and uploads everything here;
  - `crate::frame`, which keeps the texture lanes; `crate::spatial::los::terrain` and
    `crate::editing::tools::line_of_sight`, which take the `DemManifest`;
    `crate::world::environment`, which reads road segments and the elevation model; and
    `crate::world::mesh`, which composes contour rings and the sea band;
  - the Mission Creator's canvas and pointer handlers in
    `apps/website/frontend/src/v2/apps/editor/`, which sample the vector grid, and its tests, which
    read the satellite container and loading code; the debug benches in
    `apps/website/frontend/src/v2/apps/debug/`, which stroke lines with the road strips;
  - the world export, the map raster pipeline and the map checks in
    `tools_v2/developer-tools/src/`, which write and verify the terrain files.
- Rules: the module compiles only with the `world` feature and each file under its gate above; a
  binary file is validated before it is read, and one of another schema or container version is
  refused rather than guessed, as each child's tests hold; a manifest block this build cannot read
  (another encoding, a missing path) is skipped, never half-read.

## Related documentation

- [Everon dataset](/assets_v2/terrains/everon/README.md) — the terrain files this module reads.
