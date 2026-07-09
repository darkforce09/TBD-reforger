# T-151.8 verify log — Culling + density ladder

**Tag:** `T-151.8`  
**Baseline:** T-151.7.3 (`804f779a`)  
**Worktree:** `tbd-reforger-wgpu-spike/`  
**Date:** 2026-07-09

## Scope shipped

1. CPU **draw-set** cull (strict visible rect ∩ pinned ∩ cells) — Class S  
2. Exact-count **density ladder** + heatmap — Class R  
3. Damage-driven render skip — Class R  
4. WebGPU compute cull — **DEFERRED** (this section)

`DRAW_CULL_MARGIN_M = 0`. Streaming/pin keeps preload; glyph/heatmap compose iterates **draw_ids only**. Hub “+ margin” is satisfied by residency preload for fetch; draw cull is strict visible rect.

Props / badges: unchanged hard composition under LOD gates; **no** heatmap ladder this slice.  
TBDD visual heatmap deferred; count-grid satisfies hub Class R (TBDD corner sums are not instance counts).

DensityHeat draw order: after forest outline, before world tree glyphs.

---

## Compute cull (hub W8)

Status: **DEFERRED**  
Shipped instead: CPU `draw_set` (strict rect ∩ pinned) + exact-count ladder.  
Not shipped: VERTEX|STORAGE compaction, draw_indirect, WebGPU-only instance cull.  
No TypeScript substitute.  
Follow-up: T-151.8.1 or fold into post-W9.

---

## Class S — draw-set

| Gate | Result | Evidence |
|------|--------|----------|
| `draw_chunk_ids(strict) == chunk_ids_for_rect(chunk_rect_for_bbox(strict)) ∩ pinned ∩ cells` (sorted) | PASS | `class_s_draw_set_equals_strict_reference` |
| Every draw id ∈ pinned | PASS | same |
| Preload pin ⊃ draw-set (strict subset when preload expands) | PASS | same (`pin_set.len() > draw.len()`) |
| `chunks_draw == draw_ids.len()` in stats | PASS | `class_r_chunks_draw_matches_draw_ids_len` |

---

## Class R — density ladder

| ID | Assert | Result |
|----|--------|--------|
| R1 | `exact_tree_count` == hand-sum of row lens | PASS (`density_ladder::r1_*`) |
| R2 | heatmap false @ 150000; true @ 150001 | PASS (`r2_heatmap_boundary`) |
| R3 | Σ density texels over draw_ids == exact_tree_count | PASS (`r3_*` + residency swap test) |
| R4 | heatmap true → `tree_glyph_count() == 0` | PASS (`class_r_heatmap_swap_and_full_pack`) |
| R5 | under budget → glyphs pack every instance (no silent drop); heatmap false | PASS (predicate-only gate; pack_trees when !heatmap) |

---

## Class R — damage-driven render

| Gate | Result | Evidence |
|------|--------|----------|
| dirty → submit | PASS | `damage::class_r_*` |
| clean second frame → no submit | PASS | `class_r_clean_second_frame_skips` |
| pan marks dirty | PASS | `class_r_pan_marks_dirty` |
| continuous → always submit | PASS | `class_r_continuous_always_submits` |

Engine: `RenderEngine::render` early-outs when `!dirty && !continuous` (`submitted_last_frame`, `uniform_bytes_last_frame=0`). HUD calls `set_continuous_render(true)` so FPS counter keeps updating; idle Class R path remains when continuous is false.

---

## LANGUAGE GATE

| Check | Value | Pass |
|-------|-------|------|
| `wgpuSlots.ts` LOC | 56 | ≤ 60 |
| New cull/ladder math in TS | 0 | PASS (wasm getters + `upload_density_grid` only) |
| Vitest | 393/393 | PASS |
| Entry isolation | no `map_engine_wasm_bg` in `dist/assets/index-*.js` | PASS |
| wasm | `map_engine_wasm_bg.wasm` ~4.09 MB | built |

---

## Band table (S4) — operator fill after hard-refresh `?engine=wgpu`

Pinned cameras @ 800×600 (or device under test). Fill from `engine.stats()` + `residency.stats()` / `window.__wgpuWorldStats`.

| zoom | target | chunks_draw | tree_glyph_count | heatmap | gpu_frame_ms | submitted_idle |
|------|--------|-------------|------------------|---------|--------------|----------------|
| −6 | Everon center | *operator* | *operator* | 0/1 | null if no TIMESTAMP_QUERY | false on 2nd frame when continuous=false |
| −2 | Everon center | *operator* | *operator* | 0/1 | *operator* | *operator* |
| 0 | Everon center | *operator* | *operator* | 0/1 | *operator* | *operator* |
| +2 | Everon center | *operator* | *operator* | 0/1 | *operator* | *operator* |

`fps` is optional FE HUD — not a Class R gate. With HUD continuous=true, `submitted_idle` must be verified by temporarily calling `set_continuous_render(false)` (devtools) or a one-shot native damage test (already PASS above).

---

## Manual operator (post-ship)

- **S1:** Pan/zoom — `chunks_draw` ≤ `chunks_pinned`; off-screen pins do not inflate glyph count.  
- **S2:** Over-budget draw_set → heatmap on, tree glyphs 0; zoom in under budget → glyphs return.  
- **S3:** Idle with continuous=false → second frame `submitted_last_frame` false.  
- **S4:** Fill band table above from live stats.

---

## VERIFY (automated)

```text
cargo fmt --check                          PASS
cargo clippy --all-targets -- -D warnings  PASS
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings  PASS
cargo test -p map-engine-core --all-features  PASS
cargo test -p map-engine-render              PASS
cargo build --workspace && make wasm         PASS
npm test (393) / build / lint                PASS
! grep map_engine_wasm_bg index-*.js         PASS
wc -l wgpuSlots.ts ≤ 60                      PASS (56)
```

**Ready for Cursor doc sync.**
