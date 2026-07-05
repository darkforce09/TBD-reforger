# T-090.8.1 — Claude Code handoff (forest mass render)

**Active slice:** T-090.8.1 · **Executor:** claude-code · **CWD:** repo root on `main`

## Preflight

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090
./scripts/ticket prompt T-090 --slice T-090.8.1   # extracts spec §Claude Code prompt
```

**Flag on for manual:** `VITE_WORLDMAP_ENABLED=1 make web` — **hard refresh** after each deploy (module caches).

## What this slice is

First time **forests become visible** on the map — as green **polygon mass**, not individual tree icons.

| Source | Path | Role |
|--------|------|------|
| TBDD density | `packages/map-assets/everon/objects/density/{cx}_{cy}.bin` (625 grids, 32 m cells) | marching squares → `world-forest` fill |
| Region hulls | `packages/map-assets/everon/objects/forest-regions.json.gz` (36 Path B regions) | `landCoverRegions.ts` → coarse forest/field/waterBody polygons |

Export gates F1/F2/F6 already PASS @ T-090.3.2. This slice is **render only**.

## Dependencies shipped

- T-090.3.2 @ `a055df95` — density + regions export
- T-090.5.3 @ `155651b9` — worker + chunkStore (use for viewport-scoped density load)
- T-090.5.2.x — roads/buildings unchanged underneath

## Primary files

| Action | Path |
|--------|------|
| NEW | `worldmap/forestMass.ts` (+ test) |
| NEW | `worldmap/landCoverRegions.ts` (+ test) |
| EXTEND | `worldmap/useWorldMapLayers.ts` |
| MAY EXTEND | `workers/worldObjectsCore.ts` (density decode off main thread) |

## Layer contract (plan §4.2)

- `world-forest` — PolygonLayer fill, `rgba(34,120,60,α)` per N3
- `world-forest-outline` — PathLayer or polygon stroke @ ≥ −1.5
- `pickable: false` — pick ships T-090.9

## Gates (this slice)

| ID | Check |
|----|-------|
| F3 | @ −2 filled forest polygons, no tree icons inside |
| LOD3 | vitest: −2 forests=polygons, trees=hidden, buildings=rects |
| N11 P2b | load ≤3000 ms, +20 MB resident, ≥55 fps @ −2 |

F4 hover tooltip → **T-090.9** (do not block ship on tooltip UI).

## After ship

Cursor doc sync → active slice **T-090.5.4** (sea-band + contours).
