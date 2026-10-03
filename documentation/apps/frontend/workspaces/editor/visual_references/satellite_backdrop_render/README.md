**Status:** live

# Satellite backdrop render

Design-phase reference for the mood of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s satellite map: a rendered
satellite view of a wooded island. It gives colour context and is not an implementation source;
the built map is drawn by the map engine the editor under `apps/frontend/src/workspaces/editor/`
drives.

## Contents

```text
documentation/apps/frontend/workspaces/editor/visual_references/satellite_backdrop_render/
└── satellite_backdrop_render.png  the render; a render has no html export
```

## How it works

The render shows a high-contrast satellite image of an island in a dark sea under a cyan grid,
with coordinate labels in degrees, orange target reticles and crosses, and the banner "TOP SECRET
// OPERATION BLACK WAVES // SATELLITE FEED: ACTIVE // ASSET TRACKING".

The built map differs: it draws the chosen terrain's own satellite image, or the map-style
basemap, streamed by the map engine, with grid references on the top and left edges and none of
the render's reticles, degree labels or banner. The
[map basemap and world objects](/documentation/apps/frontend/workspaces/editor/feature_inventory/map_basemap_and_world_objects.md)
area of the feature inventory holds the built behaviour.

## Code

- [Satellite basemap](/crates/streaming/map_asset_loading/src/terrain/satellite_quadtree/) — the map's
  satellite imagery that stands where the render's image is.
- [Mission Creator](/apps/frontend/src/workspaces/editor/) — the editor that shows the map.

## Boundaries

- Depends on: nothing; the png is self-contained.
- Used by: the visual references README.
- Rules: the set is kept as captured.
