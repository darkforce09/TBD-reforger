# T-090.5.5 — Claude Code handoff (tree / veg / prop glyphs)

**Active slice:** T-090.5.5 · **Executor:** claude-code · **CWD:** repo root on `main`

## Preflight

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090
./scripts/ticket prompt T-090 --slice T-090.5.5
make map-glyphs-verify   # baseline before adding SVGs
```

**Flag on for manual:** `VITE_WORLDMAP_ENABLED=1 make web` — **hard refresh** after each deploy.

## What this slice is

First time **individual world instances** render as Deck **IconLayer** glyphs — trees from PH-P2 (501,861 indexed in worker), plus vegetation/props at their LOD gates. Forest mass polygons remain the island-zoom read; glyphs appear only at deckZoom ≥ `TREE_GLYPH_MIN_ZOOM` (0).

| Layer id | Slot | Source |
|----------|------|--------|
| `world-trees` | 9 | worker `visibleInstances` → tree kinds |
| `world-props` | 10 | worker `visibleInstances` → prop/rock kinds |

## Dependencies shipped

- T-090.5.3 @ `155651b9` — `visibleInstances(bbox, deckZoom)` budget-capped SoA
- T-090.8.1 @ `e28d073a` — forest mass below glyph band (LOD3 @ −2: polygons, no icons)
- T-090.5.4 @ `bd481cf1` — cartographic underlay complete
- T-090.5.2.x — `worldGlyphAtlas.ts`, building badges (pattern to follow)

## Primary files

| Action | Path |
|--------|------|
| NEW | `worldmap/treePropLayer.ts` (+ test) |
| EXTEND | `worldmap/useWorldMapLayers.ts` — slots 9–10 |
| EXTEND | `packages/map-assets/glyphs/svg/*` + `make map-glyphs-build` |
| READ | `workers/worldObjectsCore.ts` — `VisibleSet` shape |
| READ | `worldmap/chunkStore.ts` — viewport subscription pattern |

## Gates

| ID | Check |
|----|-------|
| LOD3 | @ −2 trees **hidden**; @ 0+ trees **visible**; buildings unchanged |
| R7 | `make map-glyphs-verify` — every `render.iconKey` in catalog has SVG + atlas entry |
| R8 | vitest rotation pick @ 90° ≠ 0° |
| N11 | INSTANCE_BUDGET 150k; ≥55 fps in tree-visible band (manual R5) |

## Out of scope

- World supercluster (LOD5 — forbidden)
- Pick/hover UI (**T-090.9**)
- Legacy `tiles/map/` retirement (**T-090.10.2** — after visual parity sign-off)

## After ship

Cursor doc sync → active slice **T-090.9** (interaction) or **T-090.10.2** (legacy retirement) per operator call.

**Prior verify:** [t090_5_4_verify_log.md](t090_5_4_verify_log.md) @ `bd481cf1`
