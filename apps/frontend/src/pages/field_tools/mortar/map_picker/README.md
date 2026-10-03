# Mortar calculator map picker

The Everon map of the mortar calculator: a terrain-and-imagery map view with the guns, the target
and the solution drawn on it, where a click places the chosen position and a drag moves a placed
marker.

## Contents

```text
apps/frontend/src/pages/field_tools/mortar/map_picker/
├── engine_overlay.rs  the browser half: fire-mission lanes uploaded to the engine and the marker drag listeners
├── marks.rs           what is drawn: the overlay scene and the lane buffers the render engine takes
├── mod.rs             the map panel: the placement picker, the map container, the Arland note
├── mount.rs           mounting the map view in the browser with the page's heights and click handling
├── picking.rs         which position a click or a drag moves, and the 10-figure grid it writes
└── profile.rs         the terrain under the lead gun's line of fire, for the crest check
```

## How it works

`mount.rs` mounts the map through `crate::foundation::map_view::mount::mount_map_view` with the
terrain-and-imagery preferences and the page's `TerrainHeights`, so a terrain height resolves
once the map's full 2 m elevation raster has loaded. `picking.rs` turns a click into the chosen placement,
written as a 10-figure grid reference; a press on a placed marker is grabbed by
`engine_overlay.rs` and never reaches the map's pan. `marks.rs` builds the guns, the target, the
gun-to-target lines and each gun's dispersion ellipse on the
`overlay_instances::fire_mission_marks` lanes. `profile.rs` samples the ground from the lead gun to the
target into the solver's `TerrainProfile`. Arland has no elevation model, so it gets a note instead
of a map.

## Boundaries

- Depends on: `crate::foundation::map_view` (`mount`, `handles`, `navigation`, `navigation_math`,
  `terrain_height`, `terrain_preferences`, `engine_mount`); `map_coordinates::grid_reference`;
  `map_engine` (the crest profile type, the Everon terrain manifest);
  `overlay_instances::fire_mission_marks`, `unit_symbology::markers`, `map_draw_lanes`, the
  terrain sampler `terrain_line_of_sight::elevation_profile` and `terrain_elevation::manifest`;
  the mortar inputs.
- Used by: the mortar page (`page.rs`).
- Rules: a click writes only the position chosen in "Place on the map"; the browser-only code
  compiles for wasm32 alone; the picker panel around it is wasm32-only as well.

## Related documentation

- [Mortar calculator page](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the page's behaviour, its data and its decisions.
