# Chunk draw buffers

The `chunk_draw_buffers` crate: the CPU-side composers that turn the resident world chunks into
the packed buffers the map draws (the draw set of chunks, the tree, prop and building-badge icon
instances, the pier, bridge-rail and fence strips, the building footprint fill and outline), the
world-layer toggles that gate them, and `WorldResidency`, the owner of the chunk scheduler's
residency and these draw buffers, which applies every rebuild request the residency returns.

## Contents

```text
crates/streaming/chunk_draw_buffers/
├── Cargo.toml  the package: `chunk_scheduler`, the overlay, terrain, world object and graphics dependencies, tier 5
└── src/        the draw buffers, the toggles, the statistics and the world residency
```

## How it works

```text
WorldResidency call ─> chunk_scheduler::ChunkResidency method ─> DrawRebuild ─> apply_draw_rebuild
  AllBuffers ─> rebuild_buffers (footprints, strips, draw set, glyphs)
  ZoomUnderUnchangedPin ─> fill band crossed? ─> rebuild_buffers, else refresh_draw_set_and_glyphs
  DrawSetAndGlyphs ─> refresh_draw_set_and_glyphs
  GlyphLookup ─> rebuild_glyph_lookup_from_prefabs
any buffer rebuilt ─> buffers_revision + 1 ─> the map engine's world loader uploads
```

`WorldResidency` keeps the residency's public method names, so its callers see one type; the
draw buffers read the residency only through its public accessors and never write it. The folder
README in `src/` details the glyph lookup, the instance budgets, the strips, the fills and the
statistics.

## Getting started

Run from the repository root:

```bash
cargo test -p chunk_draw_buffers   # the residency lifecycle, the prefab lanes, Everon glyphs and strips
```

The Everon cases read the export under `assets/terrains/everon/` and the glyph atlas keys under
`assets/glyphs/`; the prefab lane case needs the Everon `objects/prefabs.rkyv` (a Git LFS file).

## Configuration

No feature and no environment variable. The tests enable `chunk_scheduler`'s `test_fixtures`
feature from `[dev-dependencies]` to set the viewport, the zoom and resident chunk rows directly.

## Public surface

- `world_residency::WorldResidency`: every public method of `chunk_scheduler`'s
  `ChunkResidency` under the same name, plus the draw getters of `revision`, the toggles and
  visibility of `toggles` and `residency_statistics::WorldResidency::stats_json`.
- `Error` and `Result` at the crate root; the common names in `prelude`.

## Boundaries

- Depends on: `chunk_scheduler` (the residency, its accessors and rebuild requests),
  `map_draw_lanes` and `label_layout` (gates, `INSTANCE_BUDGET`, glyph sizes and keys),
  `prefab_catalog` (class codes, oriented boxes), `world_chunks` (`ChunkId`, `WorldChunk`),
  `map_coordinates` (`Bbox`, `TerrainSizeM`), `render_primitives` (colour normalisation, icon
  packing, pixel-size floors), `vegetation` (tree counts and the heatmap rule), `road_network`
  (airfield structures and box, strip composers), `thiserror`.
- Used by: `map_asset_loading`, directly: the world loader and the occluder loader; the
  frontend's debug world line-of-sight bench (`apps/frontend/src/workspaces/debug/world_los/`).
- Rules:
  - streaming category, tier 5 (`cargo xtask verify crate-tiers`);
  - every rebuild request is applied before the call that produced it returns
    (`class_s_draw_set_equals_strict_reference`, `compose_memo_stable_then_bumps`);
  - the archive and JSON prefab lanes compose the same buffers
    (`everon_archive_lane_builds_the_same_residency_as_the_json_lane`).
