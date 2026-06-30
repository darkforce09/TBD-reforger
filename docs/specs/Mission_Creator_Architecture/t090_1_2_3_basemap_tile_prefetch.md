# T-090.1.2.3 — Basemap tile prefetch & pan stability

**Ticket:** T-090 · **Slice:** T-090.1.2.3  
**Status:** **QUEUED** — pan flicker / tiles pop in while viewport moves  
**Executor:** claude-code  
**Depends on:** **T-090.1.2.1** shipped @ `19bc785` (pyramid stable); ideally after **T-090.1.2.2** (ortho seams)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

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

## Ship

Tag **`T-090.1.2.3`** · manual **P1** — pan across tile boundary with no visible pop-in.

Handoff: `.ai/artifacts/t090_1_2_3_claude_code_handoff.md` (write when promoted to `ready`).
