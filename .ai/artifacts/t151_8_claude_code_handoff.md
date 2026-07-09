# T-151.8 — Claude Code handoff (culling + density ladder)

**Spec (wins on conflict):**
[`t151_8_culling_density.md`](../../docs/specs/Mission_Creator_Architecture/t151_8_culling_density.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `804f779a` (tag **T-151.7.3**) — **never `main`**.

## LANGUAGE GATE (D5)

Rust owns cull / density / heatmap / damage. TS = dumb UI + thin wasm only.
See `.ai/tickets/CLAUDE_CODE_PROMPT.md` §T-151 language gate.

## Operator note

T-151.7.3 collapsed slot GPU into Rust (`wgpuSlots.ts` 56 LOC). Next: scale the world glyph
path with real cull + density ladder before Deck flip (W9). Good enough > perfect (Grok 4.5);
compute cull may be partial if CPU+ladder ship clean.

## CURRENT STATE

| Piece | Status |
|-------|--------|
| WorldResidency + viewport pin | Rust (W3) |
| `visible_world_rect` | Rust |
| TBDD decode | Rust (forest mass only today) |
| `INSTANCE_BUDGET` | Hard-cap drop (no heatmap) |
| Density texture / heatmap | **Missing — W8** |
| Compute cull | Comment stub only |
| Damage-driven render | Continuous rAF |
| SlotGpuBridge | Rust (7.3) |

## What you are building

1. CPU draw-set cull (Class S).
2. TBDD → density texture + budget→heatmap swap (Class R).
3. WebGPU compute cull if feasible.
4. Damage-driven render + band table.
5. Verify log; tag **T-151.8**.

## Do not

- Edit docs/registry/CLAUDE.
- Put ladder/cull policy in TypeScript.
- Start W9 Deck delete.
- Grow `wgpuSlots.ts` past 60.

## Key files

| Concern | Path |
|---------|------|
| Residency | `world/residency.rs`, `chunk_math.rs` |
| LOD / budget | `world/lod_gates.rs` |
| TBDD | `geometry/tbdd.rs` |
| Camera rect | `camera/ortho.rs` |
| Engine | `map-engine-render/src/engine.rs` |
| Thin FE | `wgpu/wgpuWorldLoader.ts` |

## Gotchas

- Today `rebuild_glyph_buffers` **drops** over budget — replace with heatmap swap.
- Deck `treeStore` / `worldObjectsCore` stay as oracle until W9 — do not port workers.
- Prefer Rust unit tests for cull set + density sums; thin wasm smoke for vitest floor.

## Return

- SHA + tag **T-151.8**
- `.ai/artifacts/t151_8_verify_log.md`
- **Ready for Cursor doc sync.**
