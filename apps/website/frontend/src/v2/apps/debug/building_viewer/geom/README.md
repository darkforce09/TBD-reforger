# Building viewer static lanes

The blueprint half of the building viewer's plan drawing: one building blueprint, its optional
mesh drawing and the viewed floor, tessellated into packed vertex payloads the bench uploads.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/building_viewer/geom/
└── static_lanes.rs  `build_static_lanes`: one view's plates, walls, apertures, arcs and ghosts
```

## How it works

`build_static_lanes` returns a `StaticLanes`, the payload struct declared in the parent file
`apps/website/frontend/src/v2/apps/debug/building_viewer/geom.rs`. With a mesh drawing the
structure is the mesh's: the eye-height section cut draws the walls, the slab faces paint the floor
as a height-ramped heightfield, and lower floors show only through this floor's voids; the
blueprint adds its plates, apertures, furniture, stairs, swing arcs and rings as annotations.
Without a drawing, because the building has no occlusion sidecar, the blueprint supplies the walls
and plates itself. The Roof view paints the footprint plate and the roof as an eave-to-ridge
heightfield (the mesh's top surface, or the blueprint's roof grid without a drawing), with every
floor's wall centerlines as ghosts.

## Boundaries

- Depends on: the parent `geom.rs`, through `use super::*` (`StaticLanes`, the colour constants,
  `ramp`, `to_world`, `rect_corners`, `seg`, `push_strip`, `append_polygon`) and `ViewFloor` from
  `apps/website/frontend/src/v2/apps/debug/building_viewer.rs`; `website_map_engine::world`:
  `architecture::blueprint::structure` (`BuildingBlueprint`, `BuildingLevel`),
  `architecture::section::cutter` (`BuildingDrawing`, `HeightField`, `through_voids`, the plan cell
  and floor window constants) and `terrain::roads::styling::expand_polyline_strip`.
- Used by: `geom.rs`, which re-exports `build_static_lanes`;
  `apps/website/frontend/src/v2/apps/debug/building_interior.rs`, whose `build_interior_lanes`
  starts from it and routes the result onto the bench's `INTERIOR_*` lanes; the unit tests in
  `apps/website/frontend/src/v2/apps/debug/building_viewer/tests/geometry_and_lanes.rs` and
  `apps/website/frontend/src/v2/apps/debug/tests/building_interior.rs`.
- Rules: a pure function of its inputs, with no browser or engine handle, so the native test build
  covers it; the mesh replaces the blueprint's walls whenever a drawing exists
  (`mesh_drawing_replaces_walls_and_paints_heightfield`), and a lower floor shows only through
  voids (`lower_floor_ghosts_only_through_voids`), both in the parent folder's
  `geometry_and_lanes.rs` test file named above.

## Related documentation

- [Building viewer](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) — the
  bench's purpose and behaviour.
