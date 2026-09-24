# Render benchmark and engine statistics

The render engine's performance readouts: `render_bench` times a run of offscreen frames, the
stress-quad pool loads the map with synthetic instances, and `stats` reports the engine's counters
as one JSON object.

## Contents

```text
apps/website/map-engine/src/diagnostics/bench/
├── frame_1.rs  `render_bench` over n offscreen frames, and the stress-quad pool
├── frame_2.rs  `stats`, the engine's counters as JSON, and the vector-lane counts it reports
└── mod.rs      the module tree
```

## How it works

Both files compile only for wasm32 with the `render` feature. `render_bench(n)` clamps n to
1–20 000, draws the engine's current batch list n times into an offscreen target the size of the
surface through `encode_main_pass`, and times each frame's CPU encode and its submit; it then waits
up to 3 s for the queue to drain and resolves to JSON with `n`, `submit_wall_ms`, `total_wall_ms`,
`cpu_avg_ms`, `cpu_p95_ms`, `cpu_max_ms`, `submit_avg_ms`, `fps_equiv` and `drained`.

`seed_stress(n, seed)` fills the `Stress` lane with n deterministic quads from
`crate::world::scene::stress_chunk_into`, in chunks of the graphics engine's `CHUNK_CAPACITY`, and
records the generation and upload times. `clear_stress`, which `seed_stress` calls first, keeps
the last batch of the list as the calibration batch and destroys every other batch's buffers,
every textured lane's texture and the lane pool.

`stats()` reports the backend, the stress instance and chunk counts, GPU and staging bytes, the
generation, upload and GPU frame times (`gpu_frame_ms` is `null` unless the timestamp timer has a
sample), the basemap mode, tiles and bytes, the world buildings' instance and outline counts, the
instance count of each sprite lane (trees, props, badges,
[slots](/documentation_v2/glossary.md#slot), the slot drag, clusters and vehicles, taken from the
compute cull when it runs), the vector-lane counts, atlas bytes, the upload counters, the
compute-cull counters, and the CPU render time of the last frame with its moving average.
`set_vector_stat` records a vector lane's count for it; the upload belts in `crate::frame::upload`
and `clear_vector_lane` call it.

## Boundaries

- Depends on: `crate::frame` (the engine, its batch list, bindings, `encode_main_pass` and the
  compute cull), `crate::overlay::lanes` (lane ids), `crate::world::scene` (the anchor and the
  stress quads), `crate::diagnostics::timing` and `crate::diagnostics::readback::scene` (clocks and
  the async sleep), and `website_graphics_engine::layout::CHUNK_CAPACITY`.
- Used by: the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s viewport bridge
  (`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs`), which publishes `render_bench`
  as `window.__editorBench(n)` and shows `stats()` in its debug HUD once a second; the editor
  gate's smoke harness in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests.rs`,
  which calls `window.__editorBench` when it exists; the upload belts, through `set_vector_stat`.
  No code in the repository calls `seed_stress` or `clear_stress`.
- Rules: `stats()` is one flat JSON object whose keys the HUD reads by name (`chunks`,
  `tree_glyphs`, `render_cpu_ms_ema`), so a renamed key blanks the HUD without a compile error;
  the benchmark draws through the same `encode_main_pass` as a live frame, so it measures the real
  frame path.
