**Status:** live

# Map rendering crate documentation

The documentation of the map rendering crates under `crates/map_rendering/`: `map_renderer`, the
render engine of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map,
the typed GPU layers it holds, `symbology_layers_gpu` and `world_layers_gpu`, and the render
diagnostics that measure it, `map_render_diagnostics`. Developers and AI
agents read it below the crates' code READMEs, for the path from a mounted canvas to a drawn
frame, the design, the open work and the decisions.

## Contents

```text
documentation/crates/map_rendering/
└── map_rendering_overview.md  the rendering crates, the path from a mounted canvas to a drawn frame, open work
```

## Code

- [Map renderer](/crates/map_rendering/map_renderer/README.md) — the render engine: boot, frame
  path, viewport, statistics, asset sink, lane sinks and upload belts.
- [Symbology layers](/crates/map_rendering/symbology_layers_gpu/README.md) — the slot symbology,
  glyph atlas, icon lane cull, world icon lanes and lane preferences on the GPU.
- [World layers](/crates/map_rendering/world_layers_gpu/README.md) — the building, forest density,
  terrain texture and terrain line of sight overlay layers.

## Boundaries

- Depends on: the code of `crates/map_rendering/` and the Mission Creator code that calls it,
  which every claim is checked against; the
  [feature doc template](/documentation/standards/templates/feature_doc.md); the ticket manager
  (`ttm`) for open work.
- Used by: the map rendering crates' code READMEs, which link the overview under Related
  documentation; the [library crate documentation](/documentation/crates/README.md) index; the
  [crate boundary rules](/documentation/standards/crate_boundary_rules.md).
- Rules: the overview describes the committed code; a disagreement with the code goes under its
  Known discrepancies.

## Related documentation

- [GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) — the GPU
  device and frame crates the render engine draws with.
- [Map streaming](/documentation/crates/streaming/map_streaming.md) — the host and loaders that
  write through the render engine's asset sink.
