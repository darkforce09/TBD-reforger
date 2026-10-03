# Buildings

The building, outline and fence lanes the 2D map draws. The prefab rows and the footprint lookups
are the [`prefab_catalog`](/crates/world_formats/prefab_catalog/README.md) crate, which every caller
imports directly.

## Contents

```text
legacy/map_engine/src/world/environment/buildings/
├── buffers.rs  the engine's upload of building fills, outlines and fence strips as draw lanes
└── mod.rs      the module tree: `buffers`
```

## How it works

The building fill and outline buffers are composed by `WorldResidency::rebuild_buffers` in
`crate::streaming::buffers::footprint`. `upload_world_buildings`, `upload_world_building_outlines`
and `upload_world_fence_strips` (`buffers.rs`) turn those buffers into the engine's draw lanes,
positions made relative to the scene anchor.

## Boundaries

- Depends on: `crate::frame` (`RenderEngine`, bindings), `graphics_engine::draw` (the line and
  polygon uploads), `map_draw_lanes::lane_roles`, `map_coordinates::terrain_frames` (`ANCHOR`),
  `render_primitives` (the building instance layout and geometry) and `wgpu`.
- Used by: the world loader's upload in `crate::streaming::loaders::world_loader`, which calls the
  three upload methods with the buffers `crate::streaming::buffers::footprint` composes.
- Rules: `buffers.rs` compiles only for wasm32 with the `render` feature.

## Related documentation

- [Map object prefab schema](/contracts/definitions/map-object-prefab.schema.json) — the rows
  of the prefab catalogue.
