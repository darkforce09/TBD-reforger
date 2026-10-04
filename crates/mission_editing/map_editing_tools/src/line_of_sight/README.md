# Line-of-sight tool

The headless half of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
line-of-sight tool: the two-click sight ray and the one-click viewshed disc, the terrain and object
verdicts over them, and the bytes and screen geometry the browser draws. A sight check is a
measurement, never [mission](/documentation/glossary/g_to_m.md#mission) content.

## Contents

```text
crates/mission_editing/map_editing_tools/src/line_of_sight/
├── capture.rs           `LosState`, `LosMode`, `ViewshedState`: capture, sub-mode, placement
├── host_registry.rs     the host-installed ray state, DEM point sampler and viewshed state cells
├── mod.rs               the module tree
├── object_verdict.rs    the object half of a sight line, folded with the terrain half
├── object_wash.rs       `ObjectPass`: budgeted coarse-to-fine object pass, merged palette
├── projection.rs        a placed shot and its elevation profile projected into screen space
├── terrain_survey.rs    DEM walks under a shot or an observer, bounded by the Everon manifest
├── terrain_verdict.rs   eye heights, the occlusion rule and the terrain verdict wording
├── tests/               unit tests for each file, and the no-document-write guard
├── viewshed_texture.rs  a viewshed raster packed for texture upload, and the placement entry point
└── wash_palette.rs      the viewshed wash colours and the raster encoder
```

## How it works

```text
click (host)
  │ Ray:      LosState::click   first click = observer, second = target (replaces the old shot)
  │ Viewshed: ViewshedState::place, then viewshed_texture::place_viewshed
  ▼
terrain_survey   build_profile (8 m steps)   |  viewshed_scheduler::submit_terrain (2000 m disc)
  ▼
terrain_verdict  occlusion: Clear | Blocked { first sample above the eye line } | Unknown
  ▼
object_verdict   NotLoaded | Clear | Blocked | Provisional, folded by format_combined
  ▼
projection / viewshed_texture + wash_palette + object_wash   what the browser draws
```

The host installs three handles into `host_registry.rs`: the live `LosState`, the DEM point sampler
(`PointSampler`, world `(x, y)` to ground metres) and the `ViewshedState`. Readers clone what they
find, and an empty cell answers the empty result, never a stale one.

- **Ray.** A shot is two world points with their click-time ground heights; the verdict and profile
  are derived again on every draw. `occlusion` runs eye to eye at `EYE_HEIGHT_OBSERVER_M` and
  `EYE_HEIGHT_TARGET_M` (1.8 m each) and blocks only where terrain rises more than
  `OCCLUSION_EPS_M` (0.01 m) above the line; fewer than two profile samples read `Unknown`
  (`LoS —`). Escape steps down: first the pending observer, then the placed shot.
- **Viewshed.** One click places the observer and clears the old raster; `place_viewshed` hands the
  work to the viewshed scheduler, which publishes the raster back into the viewshed state, and
  returns the upload payload (`ViewshedTexture`, rows padded to 256 bytes, north row first).
  Hidden ground carries the wash, visible ground stays transparent, and off-coverage reads lighter
  than hidden and never visible.
- **Objects.** The browser attaches an `ObjectVerdict` with `apply_objects`, which moves the block
  marker to the nearer of the terrain and object blocks. `ObjectPass` re-tests terrain-visible
  cells in 32 m, 16 m and 8 m blocks (`OBJECT_LEVELS`), nearest first, the 8 m level only within
  `OBJECT_FINE_RADIUS_M` (1000 m), under a per-frame budget (`OBJECT_PASS_BUDGET_MS`, 8 ms). An
  unreachable occluder pauses the pass on the same block, and `requeue_provisional` re-tests cells
  a proxy box decided once real geometry lands.

`LosMode` toggles `Ray` and `Viewshed` on the same toolbar button, and the pointer gesture shares
the ruler's point-capture arm (`crate::ruler::should_begin_ruler`).

## Boundaries

- Depends on: `terrain_line_of_sight` (the segment sampler in `elevation_profile`, `Viewshed`,
  `Visibility`, `compute_viewshed`), `world_line_of_sight::map_to_engine` for the object pass's
  engine frame, `terrain_elevation::manifest::DemManifest`, and
  `crate::viewshed_scheduler` for the viewshed placement; the tests read
  `terrain_relief::contours` for the contour colours.
- Used by:
  - `crate::viewshed_scheduler`, whose terrain lane reads the sampler, the manifest,
    the eye height and the viewshed state;
  - the Mission Creator's line-of-sight overlay and object wash
    (`crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/los_tool.rs`,
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/los_world_wasm.rs`), its pointer gestures
    (`crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures.rs`), the editor page
    (`crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`) and the toolbelt
    (`crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/toolbelt.rs`, which reads `LosMode`);
  - the debug building viewer (`crates/frontend/workspaces/debug_benches/src/building_viewer/geom.rs`),
    through `viewshed_texture`.
- Rules:
  - nothing here names the document or its mutators
    (`the_line_of_sight_tool_never_writes_the_document` in `tests/session_local.rs` scans the
    scrubbed source);
  - the wash encoder emits the north row first, matching the textured shader's flipped V
    (`encoder_flips_rows_so_north_is_texture_row_zero` in `tests/wash_palette.rs`);
  - the wash rationale in `wash_palette.rs` quotes the contour colours
    `terrain_relief::contours::CONTOUR_RGBA` and `CONTOUR_SUMMIT_RGBA`, the constants the relief
    host draws in, and the wash keeps its alpha under theirs
    (`viewshed_rationale_cites_live_contour_rgba`);
  - every DEM walk is bounded by `everon_manifest`, whatever terrain is open.

## Related documentation

- [Mission Creator feature inventory: bottom toolbelt](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/bottom_toolbelt.md) — the Line of Sight tool, ray and viewshed.
