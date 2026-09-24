# Building interior ray lanes

The building viewer's pointer-facing interior geometry: which door leaf a click lands on, and the
line-of-sight probe ray drawn on the viewed floor, coloured by what it crosses.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/building_interior/
└── ray_lanes.rs  `door_at` and `build_ray_lane`: the door hit test and the probe-ray lane
```

## How it works

`door_at` finds the door of a compound building under a plan point on the viewed elevation band,
matching the leaf where it hangs now or its closed footprint, with a slack of
`DOOR_HIT_SLACK_M` (0.18 m), so a click on an open door closes it too. `build_ray_lane` clips the
observer-to-target ray to the viewed band with the raycaster's own `clip_t_to_band`, so the drawing
and the verdict cannot disagree, and colours each span between hits by a state machine over the
trace: clear green, cyan after glass, yellow-green after canopy, yellow after furniture cover, and
red from a terminal block to the target. A dot marks each hit whose elevation lies inside the band.
A ray that never enters the band returns an empty lane.

## Boundaries

- Depends on: the parent file `apps/website/frontend/src/v2/apps/debug/building_interior.rs`,
  through `use super::*`: the ray colours, `DOOR_HIT_SLACK_M`, the band test for an instance, and
  the building viewer's `geom` helpers it imports (`to_world`, `push_strip`, `quad`,
  `rect_corners`); from `website_map_engine::world`, `CompoundBuilding`, `Instance` and `Rigid`
  under `architecture::compound`, `LosHit`, `LosHitKind` and `clip_t_to_band` under
  `architecture::blueprint`, and `terrain::roads::styling::expand_polyline_strip`.
- Used by: `building_interior.rs`, which re-exports both functions; the building viewer's live
  host, `apps/website/frontend/src/v2/apps/debug/building_viewer/live.rs` (`build_ray_lane`) and
  `apps/website/frontend/src/v2/apps/debug/building_viewer/live/wiring.rs` (`door_at`); the unit
  tests in `apps/website/frontend/src/v2/apps/debug/tests/building_interior.rs`.
- Rules: the ray lane is clipped with the raycaster's own band rule
  (`ray_lane_is_clipped_to_the_viewed_band`), its colours follow the hit state machine
  (`ray_lane_colors_follow_the_hit_state_machine`, `ray_lane_colours_glass_and_foliage`), and a
  door counts as hit on its leaf or its closed footprint
  (`door_at_hits_leaf_and_closed_footprint`), all in that test file.

## Related documentation

- [Building viewer](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) — the
  bench's purpose and behaviour.
