# Map view

The shared seam every page uses to put a live terrain map on a canvas: canvas sizing, render
engine creation, the camera fitted to a terrain's world bounds, the damage-driven frame pump,
resize tracking, drag-pan, wheel-zoom, click-to-pick in map metres, and ground heights at the
terrain's native 2 m resolution. The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
builds its canvas mount from these parts; a map picker mounts a whole view with one call.

## Contents

```text
apps/frontend/src/foundation/map_view/
├── camera_fit.rs           `WorldBounds` from a manifest's `worldBounds`, `ViewState`, `fit_view`
├── device_size.rs          `device_size`: CSS size × device pixel ratio → canvas backing size
├── engine_mount.rs         `size_canvas`, `force_webgl_from_location`, `create_engine`
├── frame_pump.rs           `start_frame_pump`: the damage-driven pump with an after-frame hook
├── handles.rs              `MapViewHandles`: engine, host, grids, heights and the disposal flag
├── mod.rs                  the module tree
├── mount.rs                `mount_map_view`: a whole navigable terrain view in one call
├── navigation.rs           drag-pan, wheel-zoom and click-to-pick listeners, `MapClick`
├── navigation_math.rs      wheel zoom rate, click-versus-drag slop, pixel → map metres
├── resize.rs               `observe_container_resize`: container and window resize tracking
├── terrain_height.rs       `TerrainHeights`: 2 m heights from the full-resolution raster
├── terrain_preferences.rs  `terrain_and_imagery_preferences`: the fixed layers of a map picker
└── tests/                  unit tests for sizing, camera fit, navigation, heights, preferences
```

## How it works

```text
mount_map_view(MapViewMount, MapViewHandles)
├── size_canvas ──> canvas.width/height = device_size(container rect, devicePixelRatio)
├── GET /map-assets/<terrain>/manifest.json ──> WorldBounds ──> fit_view(bounds, css size)
│   (missing or malformed ──> MapViewError::ManifestUnavailable)
├── create_engine(force_webgl) ──> resize, camera bounds, first view, damage-driven
│   (failure ──> MapViewError::EngineFailed(reason))
├── start_plain_frame_pump ──> draws only when the engine is damaged
├── observe_container_resize ──> ResizeObserver on the container + window resize
├── attach_navigation ──> primary drag pans, wheel zooms at the cursor,
│                         a release within 4 px of its press ──> on_click(MapClick)
└── streaming::host::bootstrap(scope from the preferences)
    └── TerrainAndImagery: manifest, DEM + hillshade, satellite, basemap tiles, grid;
        the full-resolution raster lands in MapViewHandles::heights
```

`MapClick` carries the map position in metres (x east, y north) and the ground height there from
`TerrainHeights::height_at`, which bilinearly samples the full-resolution raster and answers `None`
before it loads, outside the terrain, or when the boot scope does not keep it. A pixel converts to
metres through the same orthographic camera the engine draws with (`map_metres_at`).

The Mission Creator does not call `mount_map_view`: its boot interleaves the mission document, and
its gestures route tools, selection and placement. Its canvas mount
(`apps/frontend/src/workspaces/editor/mission_editor/canvas_mount.rs`) takes
`force_webgl_from_location`, `size_canvas` and `MapViewHandles` from here, its boot tasks call
`create_engine` with the editor's fixed 12.8 km camera square, its frame loop runs on
`start_frame_pump`, its input listeners attach `observe_container_resize`, and its terrain boot
runs the full scope, which leaves the heights empty.

## Public surface

- `camera_fit`: `WorldBounds` (`new`, `from_manifest_json`, `width`, `height`, `centre`),
  `ViewState`, `fit_view`.
- `device_size`: `device_size`.
- `navigation_math`: `WHEEL_ZOOM_PER_PX`, `CLICK_SLOP_PX`, `wheel_zoom_delta`, `is_click`,
  `map_metres_at`.
- `terrain_height`: `TerrainHeights` (`new`, `handle`, `height_at`).
- `terrain_preferences`: `terrain_and_imagery_preferences`, `HILLSHADE_OPACITY`,
  `SATELLITE_BASEMAP`.
- Browser only: `engine_mount` (`CanvasSize`, `size_canvas`, `force_webgl_from_location`,
  `EngineStartup`, `create_engine`), `frame_pump` (`start_frame_pump`,
  `start_plain_frame_pump`), `handles::MapViewHandles`, `mount` (`MapViewMount`,
  `MapViewError`, `mount_map_view`), `navigation` (`MapClick`, `attach_navigation`),
  `resize::observe_container_resize`.

## Boundaries

- Depends on: `camera_math` (`ortho::state`); `browser_platform` (`fetch`); `map_engine`
  (`frame::engine::RenderEngine`, `frame::RafPump`, `streaming::host` for the boot, the host and grid handles and the
  camera settle, `streaming::bridge` for the preferences and progress types,
  `world::terrain::dem::full_resolution` for the heights); `web-sys`, `js-sys`, `wasm-bindgen`,
  `serde` and `serde_json`.
- Used by: the Mission Creator's canvas mount, boot tasks, input listeners and frame loop under
  `apps/frontend/src/workspaces/editor/`; map pickers under
  `apps/frontend/src/pages/`.
- Rules: nothing here imports from `pages` or `apps`; the pure modules compile natively (`device_size.rs`, whose
  only native caller is its test, in the test build only) and carry their unit tests in `tests/`; the browser modules are `#[cfg(target_arch = "wasm32")]` on their
  `pub mod` lines in `mod.rs`.

## Related documentation

- [Shared foundations](/apps/frontend/src/foundation/README.md) — where the map seam sits
  among the other foundations.
- [Elevation model](/legacy/map_engine/src/world/terrain/dem/README.md) — the
  full-resolution raster and the vector grid the heights come from.
