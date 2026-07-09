# T-151.7.1 — Claude Code handoff (selection tint / drag FPS / zoom-at-cursor)

**Spec (wins on conflict):**
[`t151_7_1_interaction_hotfix.md`](../../docs/specs/Mission_Creator_Architecture/t151_7_1_interaction_hotfix.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `ab6bcb11` (tag **T-151.7**) — **never `main`**.

## Operator report

After T-151.7 interaction:

1. **Selection tint flaky** — deselect / change selection sometimes does not update ring colors.
2. **Drag ~1000** drops FPS by ~40.
3. **RMB + wheel zoom** — cursor no longer stays on the same map point (regression vs pre-W7).

## CURRENT STATE

| Piece | Status |
|-------|--------|
| W7 gestures on wgpu | Shipped @ `ab6bcb11` |
| Selection tint | **Buggy** (cluster short-lane + OOB patch) |
| Drag overlay | Works but **recreates GPU buffer every rAF** |
| Zoom-at-cursor | **Broken** (canvas vs container rect) |

## What you are building

1. **B1** — Correct selection tint in detail + cluster; no silent OOB patches.
2. **B2** — T-061 drag: one overlay upload on start; per-frame delta uniform only; buffer reuse.
3. **B3** — One coordinate origin for wheel/pan/resize.
4. Verify log; tag **T-151.7.1**.

## Do not

- Edit docs/registry/CLAUDE.
- Redesign gesture SM.
- Start W8/W9.
- “Fix” by falling back to Deck.

## Key files

| Concern | Path |
|---------|------|
| Selection / drag GPU | `wgpu/wgpuSlots.ts` |
| Wheel vs pan rect | `WgpuTacticalMap.tsx` |
| Delta every rAF | `tools/useSelectTool.ts` |
| Silent OOB patch / buffer recreate | `crates/map-engine-render/src/engine.rs` |

## Gotchas

- `applySelectionOnlyVisible` uploads **k** instances but `lastIds` stays length **n** — patch index ≠ GPU row.
- `patch_slot_lane` returns quietly when OOB — looks like “sometimes works”.
- `syncDrag` on every delta rebuilds Map(n) + `create_buffer_init` — that is the −40 fps.
- Container needs `position:relative` if canvas is absolute for matching rects.

## Return

- SHA + tag **T-151.7.1**
- `.ai/artifacts/t151_7_1_verify_log.md`
- **Ready for Cursor doc sync.**
