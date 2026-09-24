# T-090.1.2.3 — Basemap tile prefetch & pan stability

**Ticket:** T-090 · **Slice:** T-090.1.2.3  
**Status:** **CANCELLED** @ Map Engine v2 — superseded by `tbd-sat` + A3 has no tile pyramid  
**Executor:** —  
**Authority:** [`t090_legacy_raster_pipeline.md`](t090_legacy_raster_pipeline.md)

---

## In one sentence

Stop Satellite basemap **flicker and pop-in** when panning — prefetch and cache tile textures so the map stays visually continuous while new VP8L tiles decode (operator: **~40 fps** while panning with flicker; static view much higher).

---

## Problem

`useTerrainBasemapLayer.ts` mounts one `BitmapLayer` per visible tile; each `image` URL fetches on mount. Panning crosses tile boundaries → old layers unmount / new layers mount → **blank frames until fetch+decode**. Operator: **~40 fps** while panning with **significant flicker** (static ~165 fps — decode + layer churn, not idle GPU cap).

---

## Goal

1. **Prefetch** a 1-tile ring (or configurable margin) beyond viewport at current LOD.
2. **Cache** decoded textures (`ImageBitmap` or Deck texture cache) keyed by `{z,x,y}`.
3. **Hold previous tiles** visible until replacement is ready (no flash to grid).
4. Maintain **≥55 fps** pan contract + `MAX_VISIBLE_BASEMAP_TILES` cap behavior.

---

## Out of scope

- Pyramid rebuild, stitch, lossless encode
- Map cartographic view (T-090.1.1) — but reuse cache for both views if shared loader

---

## Implementation notes

| Area | Direction |
|------|-----------|
| **Cache** | New module e.g. `basemapTileCache.ts` — LRU cap ~128 entries; store `ImageBitmap` |
| **Prefetch** | After `computeLod`, warm `{z,x±1,y±1}` via idle fetch |
| **Hold** | Layer opacity 0→1 when loaded; or keep prior z tiles until new z ready |
| **Worker** | If decode-bound: VP8L decode off main thread |
| **Tests** | Vitest: cache hit returns same bitmap ref |

## Manual acceptance

| ID | Pass |
|----|------|
| **P1** | Pan across tile boundary — no blank flash |
| **P2** | Pan fps **≥55** (was ~40) |
| **P3** | Max zoom sharpness unchanged |

Log: `.ai/artifacts/t090_1_2_3_verify_log.md`

---

## Ship

Tag **`T-090.1.2.3`**

Handoff: [`.ai/artifacts/t090_1_2_3_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_3_claude_code_handoff.md) · send-off [`.ai/artifacts/t090_1_2_3_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_3_SEND_TO_CLAUDE.md)

Resume: [`t090_1_2_satellite_backlog.md`](t090_1_2_satellite_backlog.md)

