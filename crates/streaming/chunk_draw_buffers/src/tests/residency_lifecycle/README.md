# Residency lifecycle tests

Unit tests that drive a `WorldResidency` over a synthetic 25 × 25 grid of 512 m cells, one
building a chunk (or a dense forest with one tree glyph): the requested chunk set, the pin key,
known-empty chunks and the retry cap, LRU eviction, apply-frame accounting, picking, the draw
set, the tree heatmap and the glyph memo.

## Contents

```text
crates/streaming/chunk_draw_buffers/src/tests/residency_lifecycle/
├── cases_1.rs  the cases: lifecycle, eviction, budget accounting, draw set, heatmap, glyph memo
└── mod.rs      the module tree; the fixtures: grid setup, chunk bytes, viewport driver, tree injection
```

## Boundaries

- Depends on: the crate root (`WorldResidency`, `DrawBuffers`);
  `chunk_scheduler` (`IngestOutcome`, `FETCH_FAILURE_CAP`, `LRU_MIN_CHUNKS`, the test
  hooks that set the viewport and zoom and rewrite a resident chunk); `vegetation::canopy`,
  `map_draw_lanes::zoom_gates`, `map_coordinates::chunk_math`, `prefab_catalog` class codes,
  `flate2` and `serde_json`.
- Used by: nothing outside the folder; `crates/streaming/chunk_draw_buffers/src/tests/mod.rs`
  compiles it only in test builds.
- Rules: the cases and what each holds are listed in
  `crates/streaming/chunk_draw_buffers/src/tests/README.md`; each case keeps its assertions and
  fixtures; `cargo test -p chunk_draw_buffers` runs them.
