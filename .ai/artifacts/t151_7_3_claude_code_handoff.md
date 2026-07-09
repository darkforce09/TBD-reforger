# T-151.7.3 — Claude Code handoff (Rust collapse: kill fat wgpuSlots.ts)

**Shipped:** @ `804f779a` (tag **T-151.7.3**) — verify [`t151_7_3_verify_log.md`](t151_7_3_verify_log.md).
Cursor doc-sync done. **Next:** T-151.8 culling + density ladder.


**Spec (wins on conflict):**
[`t151_7_3_rust_collapse.md`](../../docs/specs/Mission_Creator_Architecture/t151_7_3_rust_collapse.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `69ca1c08` (after **T-151.7.2**) — **never `main`**.

## Operator / architecture mandate

**LANGUAGE GATE (D5):** Rust owns engine policy. TS = dumb UI only.
See `.ai/tickets/CLAUDE_CODE_PROMPT.md` §T-151 language gate.

**As much Rust as possible.** TypeScript = dumb UI only (React, pointer events, Zustand).
Do **not** grow another TS controller with GPU/selection/drag/cluster policy.

`wgpuSlots.ts` (~521 LOC) is the main offender — collapse it.

## CURRENT STATE

| Layer | Owner today | Target |
|-------|-------------|--------|
| SoA / MissionDoc | Rust | Rust |
| pack_* / cluster_mode | Rust `slots_gpu` (+ TS mirror) | **Rust only** |
| sync / selection / drag / cluster lanes | **TS `wgpuSlots.ts`** | **Rust bridge** |
| React subscribe / atlas canvas pixels | TS | TS (thin) |
| useSelectTool / mapCamera | TS | TS (dumb UI) |

## What you are building

1. Rust `SlotGpuBridge` (name flexible) on `RenderEngine` + `MissionDoc`.
2. Thin wasm API (≤ ~10 methods).
3. `wgpuSlots.ts` ≤ **60** LOC.
4. Preserve 7.1/7.2 operator fixes (don’t reintroduce tint/FPS/zoom bugs).
5. Verify log with `wc -l` before/after; tag **T-151.7.3**.

## Do not

- Edit docs/registry/CLAUDE.
- Fix bugs by adding more TypeScript policy.
- Port `useSelectTool` to Rust this slice.
- Start W8/W9.

## Key files

| Concern | Path |
|---------|------|
| FAT bridge | `wgpu/wgpuSlots.ts` |
| Pack SoT | `crates/map-engine-core/src/slots_gpu.rs` |
| GPU lanes | `crates/map-engine-render/src/engine.rs` |
| Doc SoA | `crates/map-engine-wasm/src/lib.rs` |
| Behaviors to keep | `.ai/artifacts/t151_7_{1,2}_verify_log.md` |

## Gotchas

- 7.2 taught: **full rematerialize** selection from SoA — don’t revive OOB patch-by-index.
- 7.1 taught: drag **delta-only** per frame — phase machine must live in Rust the same way.
- Cluster markers can still be fed from FE `getClusterMarkers` via `set_cluster_markers` until
  a later slice owns clustering end-to-end.
- Canvas atlas generation can stay in TS; upload bytes once via `ensure_slot_atlas`.

## Return

- SHA + tag **T-151.7.3**
- `.ai/artifacts/t151_7_3_verify_log.md`
- **Ready for Cursor doc sync.**