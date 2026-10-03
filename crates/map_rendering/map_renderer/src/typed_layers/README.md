# Typed layer doors

The doors through which the render engine lends its lanes to the typed layers it holds as fields:
the symbology layers of `symbology_layers_gpu` and the world layers of `world_layers_gpu`. Each
door splits the engine into the layers and the parts they write through, so a layer can write
lanes while it is itself borrowed from the engine.

## Contents

```text
crates/map_rendering/map_renderer/src/typed_layers/
├── mod.rs                 the module tree
├── symbology_layers.rs    `SymbologyParts`, `RenderEngine::with_symbology` and the slot symbology's camera frame hook
└── world_layers.rs        `WorldParts` and `RenderEngine::with_terrain_line_of_sight_overlay`
```

## How it works

`RenderEngine::symbology_parts` lends the slot symbology, the glyph atlas and the icon lane cull
an `UntexturedLanes`, the camera, the shared text atlas slot and the counters they report;
`with_symbology` runs a closure on the slot symbology at work, which is how the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) binds, selects, drags and
previews [slots](/documentation/glossary/n_to_z.md#slot). `SlotSymbologyCameraSync` is the frame
hook that refreshes the slot lanes' zoom uniform, cluster gate and cluster markers on every camera
change. `RenderEngine::world_parts` lends the building, forest, terrain texture and terrain line of
sight overlay layers a `TexturedLanes` and the strip upload counter; `with_terrain_line_of_sight_overlay`
runs a closure on the overlay at work, which uploads and clears the viewshed wash. The asset sink
forwards the loaders' glyph atlas, world icon, building, forest and texture writes through the
same splits.

## Boundaries

- Depends on: `crate::lane_sinks`, `crate::upload::text_atlas` (the text atlas slot),
  `symbology_layers_gpu`, `world_layers_gpu`, `camera_math` (the camera) and `renderer_core`
  (`FrameHook`).
- Used by: `asset_sink.rs`, `boot.rs` (the frame hook registration), and the Mission Creator and
  the debug benches through `with_symbology` and `with_terrain_line_of_sight_overlay`.
- Rules: a layer is lent the parts here, never the engine; the engine names the layers' types and
  the layer crates never name the renderer.
