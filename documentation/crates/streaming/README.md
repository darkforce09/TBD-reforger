**Status:** live

# Streaming crate documentation

The documentation of the streaming crates under `crates/streaming/`: `map_streaming_host`, the
browser map host, `map_asset_loading`, the browser loaders, the live memory budget and the asset
statistics, `chunk_scheduler` and `chunk_draw_buffers`, the chunk residency and its draw buffers,
and `map_streaming_model`, the preferences, boot progress, budget model and asset sink contract
they share, which together get a terrain's served map data into the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map. Developers and AI
agents read it below the crates' code READMEs, for the flow across crates, the design, the open
work and the decisions.

## Contents

```text
documentation/crates/streaming/
└── map_streaming.md  boot, viewport passes, chunk residency, the memory budget and the loaders
```

## Code

- [Map streaming host](/crates/streaming/map_streaming_host/README.md) — the boot sequence,
  settle passes, view preferences and queries.
- [Map asset loading](/crates/streaming/map_asset_loading/README.md) — the world, occluder,
  terrain and environment loaders, the mesh composition, the live memory budget and the asset
  statistics.
- [Chunk scheduler](/crates/streaming/chunk_scheduler/README.md) and
  [chunk draw buffers](/crates/streaming/chunk_draw_buffers/README.md) — the chunk residency and
  the draw buffers composed over it.
- [Map streaming model](/crates/streaming/map_streaming_model/README.md) — the preferences, boot
  progress, budget model and `MapAssetSink` contract.

## Boundaries

- Depends on: the code of `crates/streaming/` and the Mission Creator code that calls it, which
  every claim is checked against; the
  [feature doc template](/documentation/standards/templates/feature_doc.md); the ticket manager
  (`ttm`) for open work.
- Used by: the streaming crates' code READMEs, which link the feature doc under Related
  documentation; the [library crate documentation](/documentation/crates/README.md) index; the
  [map rendering documentation](/documentation/crates/map_rendering/README.md).
- Rules: the feature doc describes the committed code; a disagreement with the code goes under
  its Known discrepancies.

## Related documentation

- [Map rendering documentation](/documentation/crates/map_rendering/README.md) — the render engine
  that implements the asset sink the loaders write through.
