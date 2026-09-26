# The static world

The ground the map draws and the line-of-sight checks trace: the terrain itself, what stands on
it, and the inside of its buildings. All of it is read from a terrain's asset folder under
`assets_v2/terrains/` and describes the ground: nothing here is authored, undone or saved.

## Contents

```text
apps/website/map-engine/src/world/
├── architecture/  the model of a building: floor blueprints, placed compounds and section cuts
├── environment/   what stands on the ground: the prefab catalogue, buildings, vegetation, labels
├── mesh.rs        CPU meshes of the static world: land cover, contour and forest outline lines
├── mod.rs         the module tree
├── scene.rs       the scene anchor, the camera's opening view, the calibration and stress scenes
├── terrain/       the ground itself: elevation model, relief, roads, satellite imagery and water
└── tests/         unit tests for the meshes and the synthetic scenes
```

## How it works

`crate::streaming` fetches a terrain's files and chunks and hands them to the modules here, which
parse, classify and compose them: `terrain/` for the ground and its imagery, `environment/` for the
objects placed on it, `architecture/` for the buildings a user looks inside. `crate::spatial`
traces sight lines through the same data, and the browser's upload methods turn the composed
buffers into draw lanes.

`scene.rs` holds `ANCHOR`, the Everon terrain centre (6400 m, 6400 m): geometry sent to the GPU is
stored relative to it, so its f32 coordinates stay within 6400 m of zero, and `world_rect_rel`
turns a world rectangle into that frame. The camera opens on `INITIAL_TARGET` at `INITIAL_ZOOM`
and pans within `EVERON_BOUNDS`; `calibration_instances` (two known quads) and `stress_chunk`
(deterministic random quads) are synthetic scenes for the render checks and benchmarks. The
instance layouts they fill belong to `website-graphics-engine`, which never learns the 12.8 km
world these numbers describe.

`mesh.rs` composes meshes on the CPU and touches no GPU: land-cover polygons coloured by kind,
contour lines with summit rings in a second colour, and the hairlines of the forest outline. It
re-exports the graphics engine's triangulation so the terrain modules and the debug building
viewer reach it through this crate.

## Public surface

- `scene`: `ANCHOR`, `world_rect_rel`, and the synthetic scenes, for `crate::frame`,
  `crate::overlay`, `crate::diagnostics` and every upload that places geometry.
- `mesh`: `compose_landcover_mesh`, `compose_two_tone_contours`, `compose_contour_hairlines`, the
  mesh buffer types and `triangulate`.
- `architecture`, `environment`, `terrain`: see each folder's README.

## Boundaries

- Depends on: `website-graphics-engine` (instance layouts, triangulation, mesh composition);
  `crate::io` for the archives; `crate::streaming` and `crate::overlay` for the parts that load
  and draw; and `crate::spatial::bvh` for the building meshes.
- Used by:
  - `crate::streaming`, `crate::spatial`, `crate::frame`, `crate::overlay`, `crate::diagnostics`
    and `crate::editing::tools::line_of_sight`;
  - the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s canvas and input
    handlers in `apps/website/frontend/src/v2/apps/editor/`, which read the elevation model, and
    the debug benches in `apps/website/frontend/src/v2/apps/debug/`, which draw buildings;
  - the blueprint tooling, the world export, the map raster pipeline and the map checks in
    `tools_v2/developer-tools/src/`.
- Rules: the static world and the authored [mission](/documentation_v2/glossary/g_to_m.md#mission)
  document share nothing: no file here names `crate::data` or `yrs`, and no file under
  `apps/website/map-engine/src/data/` names this module (`cargo xtask verify engine-layers`,
  rule 7); a type here never gains a dirty flag; the module compiles only with the `world`
  feature, `architecture` and `mesh.rs` only with `io`, and `scene.rs` only with `streaming`.
