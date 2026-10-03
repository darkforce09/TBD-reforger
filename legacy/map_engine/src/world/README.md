# The static world

The map engine's half of the static world the map draws: the browser loaders, GPU belts and CPU
mesh composition for the terrain and what stands on it. The ground's data models are crates under
`crates/terrain/` and `crates/world_objects/` (elevation, relief, satellite imagery, water, roads,
vegetation, place names, building interiors), which every caller imports directly. All of it is
read from a terrain's asset folder under `assets/terrains/` and describes the ground: nothing here
is authored, undone or saved.

## Contents

```text
legacy/map_engine/src/world/
├── environment/   what stands on the ground: building belts, the label loader, the forest loader and belts
├── mesh.rs        CPU meshes of the static world: land cover, contour and forest outline lines
├── mod.rs         the module tree
├── scene.rs       the calibration and stress scenes on the anchor, and the anchor-relative rectangle
├── terrain/       the ground itself: the elevation loader, relief host, satellite layers and water loader
└── tests/         unit tests for the meshes and the synthetic scenes
```

## How it works

`crate::streaming` fetches a terrain's files and chunks and hands them to the modules here, which
parse them with the terrain and world-object crates and compose their draw buffers: `terrain/` for
the ground and its imagery, `environment/` for the objects placed on it. The browser's upload
methods turn the composed buffers into draw lanes.

Geometry sent to the GPU is stored relative to `map_coordinates::terrain_frames::ANCHOR`, the
Everon terrain centre (6400 m, 6400 m), so its f32 coordinates stay within 6400 m of zero; the
camera's opening view and pan bounds sit beside it in that crate. `scene.rs`'s `world_rect_rel`
turns a world rectangle into that frame, and `calibration_instances` (two known quads) and
`stress_chunk` (deterministic random quads) are synthetic scenes for the render checks and
benchmarks. The instance layouts they fill belong to `render_primitives`, which never learns the
12.8 km world these numbers describe.

`mesh.rs` composes meshes on the CPU and touches no GPU: land-cover polygons coloured by kind,
contour lines with summit rings in a second colour, and contour hairlines; `FOREST_OUTLINE_RGBA`
is the forest outline's colour. The
mesh buffer types, the triangulation and the ring loops it builds on are
`render_primitives::draw::{compose, triangulate}`, which the terrain modules and the debug building
viewer also import directly.

## Public surface

- `scene`: `world_rect_rel` and the synthetic scenes, for `crate::frame`, `crate::overlay`,
  `crate::diagnostics` and every upload that places geometry.
- `mesh`: `compose_landcover_mesh` with `LandcoverInput` and `landcover_fill`,
  `compose_two_tone_contours`, `compose_contour_hairlines` and `FOREST_OUTLINE_RGBA`, for the
  relief host, the forest loader and the world loader.
- `environment`, `terrain`: see each folder's README.

## Boundaries

- Depends on: `render_primitives` (instance layouts, triangulation, mesh composition) and
  `map_coordinates` (the anchor); `world_file_formats` for the containers; the terrain crates
  (`terrain_elevation`, `terrain_relief`, `satellite_imagery`, `water_bodies`, `road_network`)
  and the world-object crates (`vegetation`, `place_names`), with `prefab_catalog`,
  `world_chunks`, `label_layout` and `map_draw_lanes`; `crate::streaming` and `crate::frame` for
  the parts that load and draw.
- Used by: `crate::streaming`, `crate::spatial`, `crate::frame` and `crate::diagnostics`; the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) reaches the loaded world
  through `crate::streaming::host`.
- Rules: the static world and the authored [mission](/documentation/glossary/g_to_m.md#mission)
  document share nothing: no file here names `crate::data` or `yrs`, and no file under
  `legacy/map_engine/src/data/` names this module (`cargo xtask verify engine-layers`,
  rule 7); a type here never gains a dirty flag; the module and `mesh.rs` compile with the
  `world` feature, and `scene.rs` only with `streaming`.
