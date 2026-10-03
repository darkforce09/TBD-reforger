# Map asset sink

The contract the map's streaming host and loaders write the renderer through, so they depend on
this crate and never on the renderer: `MapAssetSink` with its `MapViewport` supertrait, the CPU
payloads its calls take, and the shared handle the host, the loaders and their futures hold.

## Contents

```text
crates/streaming/map_streaming_model/src/asset_sink/
├── mod.rs       the module tree; re-exports the traits, the handle and the payloads
├── payloads.rs  `TextureLayerSpec`, `TextureRegion`, `GlyphAtlasImage`, `ForestDensityRaster`, `WorldGlyphLane`
├── sink.rs      `MapAssetSink`: renderer facts, layer switches, vector lanes, world objects, labels, forest, textures
├── slot.rs      `MapAssetSinkSlot` (a place holding a sink once booted) and `SharedMapAssetSink`
├── tests/       tests of the shared handle over a recording sink
└── viewport.rs  `MapViewport`: zoom, camera target, visible bounds, `set_view`, `on_camera_changed`
```

## How it works

```text
frontend: Rc<RefCell<Option<Renderer>>> ──coerces──▶ SharedMapAssetSink<Image>
loader:   handle.borrow_mut().sink_mut() ─ None (not booted) ─▶ skip the write
                                          └ Some(&mut dyn MapAssetSink) ─▶ upload / write / switch
```

`MapAssetSinkSlot` is implemented for `Option<S>` of every sink `S`, which is what lets the
frontend's renderer cell coerce to the handle without a wrapper. The calls fall into seven
groups: renderer facts (`backend_is_webgl2`, the two texture limits, `stats_json`), layer
switches (`set_world_layer_visible`, `set_lane_opacity`, `set_grid`), vector lanes
(`clear_vector_lane`, polygon meshes, strip triangles, hairlines), world objects (building fills
and outlines, fence strips, the three glyph lanes, the glyph atlas), labels (towns, road names,
spot heights), the forest density lane (raster, parameters, stored outline count) and the texture
layers (`tex_layer_begin`, the RGBA and browser-image writes, `tex_layer_commit`). The fallible
ones answer `Error::AssetRejected` with the renderer's own account.

## Boundaries

- Depends on: `crate::error`; `render_primitives::draw::compose` (`PolyMeshGpu`, `HairlineGpu`).
- Used by: the map engine's streaming host, loaders and satellite loads (through their
  `BrowserAssetSinkHandle`), and the render engine, which implements both traits
  (`crates/map_rendering/map_renderer/src/asset_sink.rs`).
- Rules:
  - payloads are CPU data only; the browser image is the sink's associated type;
  - an empty slot hands out no sink (`an_empty_slot_hands_out_no_sink`).
