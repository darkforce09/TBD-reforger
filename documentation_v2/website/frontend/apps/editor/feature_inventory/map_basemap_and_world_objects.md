**Status:** live

# Map basemap and world objects

What the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) draws under a
[mission](/documentation_v2/glossary/g_to_m.md#mission): the satellite or map basemap, the terrain's
world objects (roads, buildings, forest, props, labels) and the per-user switches for them, plus
the world-object interactions that are not built yet.

## Where it lives

- Code: the basemap in [`apps/website/map-engine/src/world/terrain/satellite/`](/apps/website/map-engine/src/world/terrain/satellite/README.md);
  world streaming in [`apps/website/map-engine/src/streaming/`](/apps/website/map-engine/src/streaming/README.md)
  and its [host](/apps/website/map-engine/src/streaming/host/README.md); the world-object
  overlays and their zoom thresholds in [`apps/website/map-engine/src/overlay/`](/apps/website/map-engine/src/overlay/README.md);
  the per-user switches in `apps/website/frontend/src/v2/apps/editor/shell/world_layer_prefs.rs`
  ([shell README](/apps/website/frontend/src/v2/apps/editor/shell/README.md)) and the "Editor
  Preferences" dialog in
  [`apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/`](/apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/README.md).
- Entry: the "Editor preferences moved" button in the "Mission Settings…" dialog opens "Editor
  Preferences"; nothing else opens it.
- Related features: [map viewport and camera](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| MAP-BASEMAP-001 | Satellite basemap | shipped |
| MAP-BASEMAP-002 | Map-style basemap and the Satellite/Map switch | partial |
| MAP-BASEMAP-003 | Road strokes baked onto the satellite image | not built |
| MAP-WORLD-001 | Forest as filled areas, outlines and tree glyphs | shipped |
| MAP-WORLD-002 | World-layer switches | shipped |
| MAP-WORLD-003 | Hover tooltip on a world object | not built |
| MAP-WORLD-004 | Read-only inspect panel for a world object | not built |
| MAP-WORLD-005 | Filter and search world objects by type | not built |
| MAP-WORLD-006 | Map legend | not built |
| MAP-WORLD-007 | Height-trust badge on the inspect panel | not built |
| MAP-AI-001 | Ask an AI about a world object | not built |
| MAP-HILLSHADE-001 | Hillshade toggle and strength | shipped |
| MAP-DETAIL-001 | Zoom-dependent world detail | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
MAP-HILLSHADE-001 and MAP-DETAIL-001 are rows this inventory adds for shipped code.

### MAP-BASEMAP-001 — Satellite basemap

1. At boot the streaming host loads a small satellite preview first, then the full-resolution
   image, under the grid (`apps/website/map-engine/src/world/terrain/satellite/quadtree/basemap.rs`,
   `apps/website/map-engine/src/streaming/host/bootstrap.rs`).
2. The boot overlay's "Loading satellite…" segment tracks it.

### MAP-BASEMAP-002 — Map basemap and the switch

1. "Editor Preferences" holds a "Basemap" section with "Satellite" and "Map" buttons. The choice
   is saved per user in the browser's local storage under `tbd-mc-editor-prefs`; the default is
   satellite.
2. The map style stitches up to 16 × 16 tiles of 256 px into one texture.
3. Switching from Map back to Satellite does not bring the satellite image back until the page
   reloads (see Known discrepancies). The map itself carries no switch.

### MAP-BASEMAP-003 — Road strokes on the satellite image

Not built: no road overlay is baked onto the satellite image, and its ticket is cancelled.

### MAP-WORLD-001 — Forest

1. Forest draws as a filled mass up to zoom 1, as an outline from zoom -1.5, and as tree glyphs
   from zoom 0 (`apps/website/map-engine/src/overlay/lod.rs`).
2. The "Forest mass" and "Trees" switches hide each part. Forests are drawn areas, not
   first-class region objects a mission maker can select.

### MAP-WORLD-002 — World-layer switches

1. "Editor Preferences" lists twelve checkboxes under "World layers", in this order: "Roads",
   "Buildings", "Forest mass", "Trees", "Props", "Contours", "Sea", "Fences", "Airfield",
   "Height labels", "Town labels", "Road names".
2. Every layer starts on except "Props". The switches are saved per user in the same
   `tbd-mc-editor-prefs` record, which also reads a `tbd-mc-world-layers` key once.
3. A switch applies to the overlays and to the world loader at once, without a reload.

### MAP-WORLD-003 to MAP-WORLD-007 and MAP-AI-001 — World-object interaction

Not built. Hovering the map changes only the mouse cursor, and only over slots, vehicles and
comments; no world object can be hovered, inspected, filtered, searched, explained by a legend,
badged for height trust or sent to an AI. The streaming scheduler's `pick_nearest`
(`apps/website/map-engine/src/streaming/scheduler/queries.rs`) is the only world-object picking
code, and nothing calls it.

### MAP-HILLSHADE-001 — Hillshade

"Mission Settings…" holds a "Show hillshade" toggle and a "Hillshade strength — N%" slider
(`apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/environment_sections.rs`);
both are stored in the mission document.

### MAP-DETAIL-001 — Zoom-dependent detail

Buildings appear from zoom -2.5, props from zoom 3, town labels from zoom -4.5 and road names
from zoom 0 or 1 by road class; unit symbols cluster at zoom -4 and below
(`apps/website/map-engine/src/overlay/lod.rs`,
`apps/website/map-engine/src/overlay/symbology/instances/symbols.rs`).

### Known discrepancies

- The "Satellite" button promises the satellite image back — `load_map_basemap` replaces the
  satellite texture in the basemap slot, and `show_satellite_basemap` only resets its opacity,
  so the map tiles stay until a reload (`basemap.rs`,
  `apps/website/map-engine/src/streaming/host/preferences.rs`).

## Data

- `GET /map-assets/…`: the terrain's satellite, map tiles and world-object chunks, served from
  `assets_v2/terrains/` through the streaming loaders; the mission document is not involved.
- Local storage `tbd-mc-editor-prefs`: the basemap style and the twelve world-layer switches,
  per browser and per user, never in the mission.

## Design

- The basemap sits under the grid and the world overlays; labels and glyphs thin out by zoom.
- Design target: the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md),
  the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
  and the [world-object interaction spec](/documentation_v2/tickets/specs/t090_9_world_object_interaction.md). Differences: the basemap switch sits in a
  dialog rather than on the map, and there is no legend, tooltip or inspect panel.

## Open work

- [T-090.9 — World-object interaction (hover, inspect, filter, legend)](/documentation_v2/tickets/specs/t090_9_world_object_interaction.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-090_9_plan.md)): world objects gain a hover
  tooltip, a read-only inspect panel, a type filter and search, a legend and the height-trust
  badge.
- [T-090.7 — Eden AI world object schema (exact field contract)](/documentation_v2/tickets/specs/t090_eden_ai_world_object_schema.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-090_7_plan.md)): the field contract an AI
  question about a world object reads.
- [T-090.5 — Map object render layer (Eden-like static world)](/documentation_v2/tickets/specs/t090_5_map_object_render_layer.md)
  (deferred, no plan): the static world render layer the interactions sit on.
- [T-090.8 — Forest & vegetation regions (first-class areas)](/documentation_v2/tickets/specs/t090_8_forest_vegetation_regions.md)
  (deferred, no plan): forests become region objects.
- [T-1058 — Fix map basemap switch back never restoring the satellite imagery](/.ai/tickets/T-1058.toml)
  (idea, no plan): switching back to Satellite restores the image without a reload.

## Decisions

- The basemap style and the world-layer switches are per-user browser preferences, not mission
  settings: they change what one mission maker sees, never the mission; the Mission Settings
  dialog says so on its "Editor preferences moved" button ("Basemap view and world layers are now
  per-user editor preferences.").
- World objects are read-only in the Mission Creator: the terrain comes from the game's export,
  and a mission never edits it.
