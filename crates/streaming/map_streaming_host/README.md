# Map streaming host

The `map_streaming_host` crate: the browser entry point of map streaming. `bootstrap` loads a
terrain's elevation model and hillshade, satellite basemap and grid, and in the full scope its
world objects, forest, water and labels, into the renderer; the `MapHost` it leaves behind
refreshes them after each camera settle and answers the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s camera, place-name, water
and line-of-sight queries; the view preferences switch the hillshade, the grid, the basemap and
the world layers of the mounted map. The host drives the loaders of
[`map_asset_loading`](/crates/streaming/map_asset_loading/README.md) and reaches the renderer only
through the asset sink of [`map_streaming_model`](/crates/streaming/map_streaming_model/README.md).

## Contents

```text
crates/streaming/map_streaming_host/
├── Cargo.toml  the package: `map_asset_loading`, `map_streaming_model`, the terrain crates; streaming category, tier 7, wasm32
└── src/        the boot, the host state, the camera settle, the view preferences and the queries
```

## How it works

```text
frontend ── bootstrap(sink handle, terrain, handles, progress, HostPreferences) ──▶ MapHost
   │ registers (sink handle, host handle) in RENDER_CTX
   ▼
camera change ─▶ schedule_camera_settle ─▶ flush_viewport: world, forest, relief, label passes
settings     ─▶ apply_hillshade / apply_grid / apply_basemap_view / refresh_world_layers
tools, docks ─▶ camera_snapshot, fly_to, named_locations, with_occluder, is_known_dry_land
```

The boot sequence, the settle passes and the queries are described in
[`src/README.md`](/crates/streaming/map_streaming_host/src/README.md). Every module compiles only
for wasm32; a native build is the crate root and its prelude, so the workspace's native lint and
test lanes compile it.

## Getting started

Run from the repository root:

```bash
cargo clippy -p map_streaming_host --target wasm32-unknown-unknown --all-targets -- -D warnings
```

## Configuration

No feature and no environment variable.

## Public surface

- `bootstrap`, `MapHost`, `HostHandle`, `DemGridHandle`, `new_host_handle`,
  `new_dem_grid_handle` and `RENDER_CTX`, for the frontend's canvas mount and map view.
- `flush_viewport`, `schedule_camera_settle` and `set_camera_gesture`, for the pointer and wheel
  gestures and the camera harness.
- `apply_hillshade`, `apply_grid`, `apply_basemap_view` and `refresh_world_layers`, for the
  settings dialogs.
- `camera_snapshot`, `fly_to`, `named_locations`, `with_occluder`, `with_occluder_host`,
  `with_water_mask`, `is_water` and `is_known_dry_land`, for the docks, tools and overlays.
- The common names in `prelude`. All of them exist on wasm32 only.

## Boundaries

- Depends on: `map_asset_loading` (the loaders, the asset sink handle, the asset statistics, the
  live memory budget); `map_streaming_model`; `terrain_elevation`, `terrain_relief`,
  `world_chunks`, `world_line_of_sight`, `water_bodies`, `label_layout`; `browser_platform`; `time_source`;
  `serde`, `serde_json`, `futures` and the browser bindings.
- Used by: the single-page app (`apps/frontend`, WebAssembly build only): its map view and the
  Mission Creator import the host directly.
- Rules:
  - streaming category, tier 7, wasm32: no GPU crate and no rendering crate
    (`cargo xtask verify crate-tiers`);
  - the Mission Creator's boot-progress tests read the boot's sources by path
    (`crates/frontend/workspaces/mission_creator_workspace/src/tests/t628_boot_progress.rs`).

## Related documentation

- [Map streaming](/documentation/crates/streaming/map_streaming.md) — the boot sequence,
  viewport passes, residency, memory budget and loaders as one flow.
