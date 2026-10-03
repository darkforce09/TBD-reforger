# Map streaming host

The browser entry point of map streaming: `bootstrap` loads a terrain's DEM and hillshade,
satellite basemap and grid, and in the full scope world objects, forest, water and labels, into
the render engine, and the
`MapHost` it leaves behind refreshes them after each camera settle and answers the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s camera, place-name, water and
line-of-sight queries. It compiles only for wasm32 with the `render` feature.

## Contents

```text
legacy/map_engine/src/streaming/host/
├── bootstrap.rs    `bootstrap`: the scoped boot sequence, its progress segments and memory accounting
├── mod.rs          the module tree; re-exports the host calls, holds the render context
├── preferences.rs  hillshade, grid, basemap and world-layer changes applied to the mounted map
├── queries.rs      camera snapshot, fly-to, named places, water mask and occluder queries
├── state.rs        `MapHost` and its shared handles
├── terrain.rs      the manifest's DEM and satellite blocks, the DEM load, hillshade upload, full raster
└── viewport.rs     the camera gesture flag, the settle debounce and the viewport refresh passes
```

## How it works

```text
bootstrap(scope from HostPreferences):
  Full only: Files(World, 7 + 2 + density bins)
  fetch /map-assets/<terrain>/manifest.json; hold DEM and hillshade forecasts, then at once:
    DEM: raw TBDE or streamed PNG ─> u16 samples ─> metres ─> hillshade ─> texture lane 1
         ─> Finish(Terrain)
    satellite: index, budget floor, tiles ─> Finish(Satellite)
  DEM grid ─> dem_out; TerrainAndImagery only: u16 raster ─> full_dem_out (FullResolutionDem)
  grid, hillshade and basemap from the preferences
  Full only: world ─> forest ─> water ─> airfield apron ─> labels ─> ≤ 12 viewport passes
  Finish(World)
camera change ─> settle (120 ms debounce, 250 ms at most) ─> flush_viewport: ≤ 6 passes
```

`RENDER_CTX`, a thread-local the page registers and clears, holds the mounted engine and host
pair; every query and preference call goes through it and answers `None`, `false` or an empty
list when no map is mounted. `bootstrap` closes the terrain, satellite and world segments on every
path, a failed manifest fetch included, holds DEM and hillshade forecasts (width × height × 4
bytes each) until the real sizes replace them, releases the hillshade once uploaded, and records
the heap growth of the other loads. The `TerrainAndImagery` scope (a fire-planning map) issues no
world-object, forest, water or label request and keeps the DEM's source `u16` raster, over the
manifest's `worldBounds`, in the caller's `FullResolutionDemHandle` for 2 m height sampling; the
`Full` scope (the Mission Creator) drops it once the metres are built.
The grid spans `TERRAIN_M` (12,800 m) on both axes; a `map` basemap falls back to the satellite
with a console warning when its tiles are missing.

Settle passes (`SETTLE_DEBOUNCE_MS`, `SETTLE_MAX_LATENCY_MS`) stop once neither the world nor the
forest did work; during a gesture they skip the DEM vector sync and, until it has uploaded once,
the forest. `is_water` and `is_known_dry_land` answer `false` for every unknown, so a placement
guard asking `is_known_dry_land` may refuse a legal spot but never accepts water.

## Boundaries

- Depends on: `crate::frame::EngineHandle`; the streaming loaders, bridge and memory budget;
  `crate::world` (the DEM loader, the relief host, basemaps, water, forest and label hosts);
  `terrain_elevation` (DEM decode and grid), `terrain_relief::hillshade`, `water_bodies`,
  `vegetation`, `world_chunks::terrain_manifest`; `world_line_of_sight` and
  `label_layout::importance` for the query types;
  `crate::diagnostics`; `futures`, `serde_json`, `wasm-bindgen-futures`, `web-sys`, `js-sys`.
- Used by: the shared map seam in `apps/frontend/src/foundation/map_view/` (handles,
  `bootstrap` in the terrain-and-imagery scope, settles); the Mission Creator in
  `apps/frontend/src/workspaces/editor/`, through its canvas
  mount and world-asset bridge (handles, `bootstrap`, `RENDER_CTX`), pointer and wheel gestures
  (gesture flag, settles), settings dialogs (hillshade, grid, basemap, world layers), camera dock
  (`named_locations`, `fly_to`), tools and overlays (`camera_snapshot`, `with_occluder`,
  `with_occluder_host`) and `__editorCamSet` harness gate (`flush_viewport`); and
  `crate::world::environment::locations::loader`, which reads `WORLD_LABEL_FILES`.
- Rules:
  - the Mission Creator's boot-progress tests read these seven files by path and require the DEM
    to stream against its content length (`the_terrain_dem_is_streamed_against_its_content_length`),
    the density bins to be declared before the world loads
    (`every_world_batch_declares_its_files_before_it_fetches_them`) and each segment to close on
    every path (`every_segment_is_closed_and_the_overlay_waits_for_a_full_bar`), all in
    `apps/frontend/src/workspaces/editor/tests/t628_boot_progress.rs`;
  - a viewport pass takes the `MapHost` out of its handle, so a query or a second flush during
    the pass finds no host and answers empty instead of waiting or panicking.
