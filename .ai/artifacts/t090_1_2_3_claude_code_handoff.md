# T-090.1.2.3 — Claude Code handoff (basemap tile prefetch)

**Slice:** T-090.1.2.3 · **Executor:** claude-code · **Depends on:** T-090.1.2.1 @ `19bc785`  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_3_basemap_tile_prefetch.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_3_basemap_tile_prefetch.md)  
**Resume guide:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md)

## Problem

Satellite panning: **~40 fps** (static ~165 fps) + **tiles pop in / flicker**. Root cause: `useTerrainBasemapLayer.ts` mounts one `BitmapLayer` per tile; each VP8L URL fetches on mount; pan crosses boundaries → blank frames until decode.

## Can run parallel to T-090.1.2.2

**Frontend only** — no ortho/pyramid rebuild. Safe to implement while seams slice runs.

## Do

1. **Tile cache** — module keyed `{z,x,y}` → `ImageBitmap` or preloaded `HTMLImageElement`; reuse across pan frames  
2. **Prefetch ring** — extend visible AABB by 1 tile margin at current LOD; warm cache before pan reaches edge  
3. **Hold previous** — don’t unmount old tiles until replacement decoded (opacity crossfade or z-order keep-under)  
4. **Optional:** decode VP8L in **Worker** if main-thread decode is the 40 fps bottleneck  
5. Keep `MAX_VISIBLE_BASEMAP_TILES=64` behavior; don’t regress T-057 pan fps contract (**≥55 fps** target while panning)

**Primary files:** `apps/website/frontend/src/features/tactical-map/layers/useTerrainBasemapLayer.ts`, possibly new `basemapTileCache.ts`

## Do not

- Change `tileUrl` Y-flip or world bounds  
- Rebuild map assets / pyramid  
- Edit docs/registry (Cursor sync after ship)

## Verify

```bash
cd apps/website/frontend && npm run build && npm run lint && npm test
make ci-local-frontend
```

Manual **P1:** pan across tile boundaries — no visible pop-in; pan fps materially closer to idle (operator eyeball).

## Ship

Tag **`T-090.1.2.3`** · prefix **`T-090.1.2.3:`** · `.ai/artifacts/t090_1_2_3_verify_log.md`

Return: **"Ready for Cursor doc sync."**
