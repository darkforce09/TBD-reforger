# T-151.8 — Claude Code handoff (culling + density ladder)

**Shipped:** @ `f4ffbfff` (tag **T-151.8**) + compute cull @ `ec59d10e` (tag **T-151.8.1**) —
verify [`t151_8_verify_log.md`](t151_8_verify_log.md). Cursor doc-sync in progress.
**Next:** T-151.9 Deck flip + retirement.

**Spec (wins on conflict):**
[`t151_8_culling_density.md`](../../docs/specs/Mission_Creator_Architecture/t151_8_culling_density.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` — **never `main`**.

## Shipped summary

- CPU draw-set cull (strict visible ∩ pinned; Class S)
- Exact-count density ladder + heatmap (Class R)
- Damage-driven render skip
- WebGPU compute cull (`VERTEX|STORAGE` + `draw_indirect`) — **shipped in 8.1, not deferred**
- LANGUAGE GATE held (`wgpuSlots.ts` 56 LOC)
