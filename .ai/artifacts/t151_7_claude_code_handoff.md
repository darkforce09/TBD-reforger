# T-151.7 — Claude Code handoff (interaction rewire + parity suite)

**Spec (wins on conflict):**
[`t151_7_interaction_rewire.md`](../../docs/specs/Mission_Creator_Architecture/t151_7_interaction_rewire.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `033ff715` (tag **T-151.6**) — **never `main`**.

## Operator note

T-151.6 rings show up. Next: **make wgpu editable** — select / drag / marquee / drop / Space /
CUR on `?engine=wgpu`. Do not redesign gestures; only swap camera math to ULP-0.

## CURRENT STATE (wgpu @ T-151.6)

| Capability | Status |
|------------|--------|
| World + glyphs + slot rings / clusters | Yes (W0–W6) |
| Raw LMB pan + wheel zoom | Yes (conflicts with select) |
| `useSelectTool` / pick / marquee / drag commit | **No — W7** |
| `onReady` / `flyTo` / cursor / asset drop / dbl-click | **No — W7** |
| Scripted Zustand selection/drag tint | Works (dev only) |

Deck mount remains the full interaction oracle.

## What you are building

1. `RenderEngine.unproject_xy` (if missing) + FE camera helper for wgpu.
2. `useSelectTool` driven by ULP-0 camera on wgpu (Deck path unchanged).
3. Wire all `TacticalMapProps` interaction callbacks on `WgpuTacticalMap`.
4. `MissionCreatorPage` passes the same callbacks to the wgpu branch.
5. Interaction parity suite; verify log; tag **T-151.7**.

## Do not

- Edit docs/registry/CLAUDE.
- Redesign gesture SM / thresholds / modifiers.
- Start W8 cull or W9 Deck delete.
- Touch forest / slot pack layout.

## Key files

| Concern | Path |
|---------|------|
| Gesture SM | `tools/useSelectTool.ts` |
| Deck oracle | `TacticalMap.tsx` |
| wgpu mount | `WgpuTacticalMap.tsx` |
| Page wiring | `mission-creator/MissionCreatorPage.tsx` |
| Camera | `crates/map-engine-core/src/camera/ortho.rs`, `engine.rs` |
| Picks | `state/slotSpatialIndex.ts`, `slotClusterIndex.ts` |
| CUR z | `dem/sampleElevation.ts`, `DemController.ts` |
| Parity | `features/_wasm/orthoCamera.parity.test.ts` |
| Slots (leave) | `wgpu/useWgpuSlots.ts` — already follows selection/drag store |

## Gotchas

- Today wgpu **LMB = pan** — must yield to `useSelectTool` (middle/right pan like Deck).
- `OrthoCameraJs` has `unproject_xy`; live `RenderEngine` may not — add thin forwarder.
- `MissionCreatorPage` wgpu branch currently omits interaction props — wire them.
- Cluster drill + Space `flyTo` need `onReady` → `TacticalMapApi`.
- Parity suite can start as vitest scripts against both mounts; full browser A/B OK in verify log.

## Return

- SHA + tag **T-151.7**
- `.ai/artifacts/t151_7_verify_log.md`
- **Ready for Cursor doc sync.**
