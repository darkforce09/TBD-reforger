# Map streaming model

The `map_streaming_model` crate: what the map's streaming host, its loaders and an embedding
frontend share without naming a browser or a GPU. It holds the twelve world-layer switches, the
boot scope and the preference readers a frontend hands the map host, the boot progress events
the loaders report with the satellite fetch's Range planning, the memory budget ledger with its
satellite floor walk, and `MapAssetSink`, the contract the loaders write the renderer through.

## Contents

```text
crates/streaming/map_streaming_model/
├── Cargo.toml  the package: `render_primitives`, `serde`, `thiserror`; streaming category, tier 1, any target
└── src/        the preferences, the boot progress, the memory budget model and the asset sink contract
```

## How it works

```text
frontend ── HostPreferences, WorldLayerPrefs, ProgressFn ──▶ map host and loaders
                                                               │ BootEvent per landed unit
map host and loaders ── SharedMapAssetSink ──▶ dyn MapAssetSink (the renderer's impl)
                    └── memory_budget::Ledger (held in the map engine's live ledger)
```

The loaders depend on this crate, never on the renderer: every upload, texture write, layer
switch and camera read they make is a `MapAssetSink` (or `MapViewport`) call on the slot of a
`SharedMapAssetSink`. The frontend's renderer cell, an `Rc<RefCell<Option<_>>>` of a sink,
coerces to that handle, and an empty slot (before boot, after teardown) hands out no sink, so the
write is skipped at the call site. The sink names the browser's decoded image as an associated
type, so this crate compiles natively and its tests run on the host.

## Getting started

Run from the repository root:

```bash
cargo test -p map_streaming_model   # the budget, the shared sink handle and the layer switches
```

## Configuration

No feature and no environment variable.

## Public surface

- `asset_sink`: `MapAssetSink`, `MapViewport`, `MapAssetSinkSlot`, `SharedMapAssetSink` and the
  payloads `TextureLayerSpec`, `TextureRegion`, `GlyphAtlasImage`, `ForestDensityRaster` and
  `WorldGlyphLane`.
- `boot_progress`: `BootEvent`, `BootSeg`, `ProgressFn`, `split_range`, `Ordered` and the fetch
  sizes.
- `host_preferences`: `BootstrapScope`, `HostPreferences`, `RenderPreferences`.
- `memory_budget`: `Ledger`, `Asset`, `Decision`, `Entry`, `LevelBytes`, `FloorWalk`,
  `floor_for_budget`, `satellite_resident_bytes`, `budget_bytes_from_settings`, `MIB`,
  `DEFAULT_BUDGET_MB`.
- `world_layer_preferences`: `WorldLayerPrefs`.
- `Error` and `Result` at the crate root; the common names in `prelude`.

## Boundaries

- Depends on: `render_primitives` (the polygon mesh and hairline payloads), `serde` (the stored
  world-layer switches), `thiserror`.
- Used by: `map_streaming_host`; `map_asset_loading`: the world, occluder, forest, label, water
  and relief loaders, the satellite loads and the live memory ledger; `map_renderer`, whose render
  engine implements the sink (`crates/map_rendering/map_renderer/src/asset_sink.rs`); and the
  single-page app (`apps/frontend`, every target), whose map view and Mission Creator import the
  preferences, bootstrap scope and boot progress types directly.
- Rules:
  - streaming category, tier 1, any target: no browser crate, no GPU crate
    (`cargo xtask verify crate-tiers`);
  - an empty slot hands out no sink and writes reach the renderer's own cell in call order
    (`an_empty_slot_hands_out_no_sink`,
    `writes_through_the_shared_handle_reach_the_renderer_cell_in_call_order`);
  - the budget rules are held by the tests in `src/memory_budget/tests/`.
