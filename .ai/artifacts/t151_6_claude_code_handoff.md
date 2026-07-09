# T-151.6 — Claude Code handoff (mission entities: slots / selection / drag / clusters)

**Shipped:** @ `033ff715` (tag **T-151.6**) — verify [`t151_6_verify_log.md`](t151_6_verify_log.md).
Cursor doc-sync done. **Next:** T-151.7 interaction rewire.


**Spec (wins on conflict):**
[`t151_6_mission_entities.md`](../../docs/specs/Mission_Creator_Architecture/t151_6_mission_entities.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `a98fb421` (tag **T-151.5.1**) — **never `main`**.

## Operator note

T-151.5.1 forest fidelity is **good enough for now**. Path B / 32 m cell polish waits for Fable 5
(**T-149**). Do **not** reopen forest this slice.

## CURRENT STATE (wgpu @ T-151.5.1)

| Layer | Status |
|-------|--------|
| Basemap / hillshade / grid | Yes |
| Sea / landcover / contours / roads / forest / buildings | Yes |
| Tree / prop / badge glyphs | Yes (W5) |
| Forest iso=2 + glyph-band green hide | Yes (5.1) |
| **Slot rings / selection / drag / clusters** | **No — W6** |
| Gesture / pick rewire | No — W7 |

Deck oracle still draws slots via `useIconLayer` + `useClusterIconLayer` + `slotIconCache`.

## What you are building

1. Dedicated **ring + disc** atlas (not world-glyphs) + `LaneRole::{Slots,SlotDrag,Clusters}`.
2. GPU upload from **`MissionDoc.refresh()` → `slot_xy_ptr` / `slot_len`** with dirty ranges.
3. Selection tint + T-061 drag overlay (delta uniform).
4. T-065 cluster discs from Rust `ClusterIndex` under exact gates.
5. Wire `WgpuTacticalMap`; verify log; tag **T-151.6**.

## Do not

- Edit docs/registry/CLAUDE.
- Rewire `useSelectTool` / camera (W7).
- Use Zustand `slotsById` as GPU SoT (SoA wins).
- Touch forest / Path B / T-149.
- Delete Deck layers (T-151.9).

## Key files

| Concern | Path |
|---------|------|
| SoA / refresh | `crates/map-engine-wasm/src/lib.rs`, `doc/{store,soa}.rs` |
| Cluster | `spatial/cluster.rs`, `state/slotClusterIndex.ts` |
| Deck oracle | `layers/useIconLayer.ts`, `useClusterIconLayer.ts` |
| Cache / gates | `state/slotIconCache.ts`, `state/constants.ts` |
| Icon pipeline | `crates/map-engine-render/src/{engine,scene}.rs` |
| Mount | `WgpuTacticalMap.tsx`, `wgpu/wgpuWorldLoader.ts` (W5 pattern) |
| Doc shell | `state/wasmDoc.ts`, `ydoc.ts`, `useMapStore.ts` |

## Gotchas

- Slot art is a **procedural ring** (canvas in Deck) — upload a tiny atlas; do not steal glyph 0–27.
- `upload_icon_lane` today kinds 0/1/2 = trees/props/badges — extend carefully (new kinds or new API).
- Wgpu mount does not yet run `useSelectTool` — selection/drag may only show when Zustand is set
  (Deck dual-mount A/B or scripted store). Full pick = W7.
- `WasmMissionDoc` does not currently expose `refresh`/`slot_xy_ptr` on the shell — call through
  `md.wasm` (or thin wrappers) after attach.
- Marquee lane already exists (role 7 / `LaneRole::Marquee`) — keep draw order coherent.

## Return

- SHA + tag **T-151.6**
- `.ai/artifacts/t151_6_verify_log.md`
- **Ready for Cursor doc sync.**