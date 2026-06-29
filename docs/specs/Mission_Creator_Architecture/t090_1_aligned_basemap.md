# T-090.1 — Aligned Cartesian basemap

**Ticket:** T-090 · **Slice:** T-090.1  
**Status:** Spec ready — **blocked on tile pyramid** (T-090.1 / T-121 — DEM shipped @ T-091.0)  
**Executor:** claude-code  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Render aligned Everon top-down tiles under the procedural grid in `<TacticalMap>`, clipped to terrain bounds, with grid-only fallback when tiles 404.

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-090.1 / T-121** | `packages/map-assets/everon/tiles/{z}/{x}/{y}.webp` exists (≥ z0) — **not** gated by T-091.0 (tiles deferred) |
| **T-090.0** | Manifest `tiles.urlTemplate`, `alignmentOrigin`, `bounds` validate |
| **Dev serve** | Tiles reachable at `/map-assets/everon/tiles/...` (symlink or Vite static — see DEV_RUNBOOK) |

---

## Problem

[`useBaseMapLayer.ts`](../../../apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts) draws only a procedural grid. Mission makers cannot visually align slots with in-game geography.

---

## Goal

1. Load terrain manifest via `manifestUrl` from [`terrains.ts`](../../../apps/website/frontend/src/features/tactical-map/coords/terrains.ts).
2. Add basemap layer using **Cartesian** coordinates — **BitmapLayer** (single extent) **or** TileLayer with explicit world bbox — **never default Web Mercator** ([`engineering_plan.md`](engineering_plan.md) §4.1).
3. Keep existing grid overlay on top (semi-transparent).
4. Clip to `worldBounds` from manifest.
5. Degraded mode: fetch 404 → grid-only + non-blocking toast (same as today).

---

## Out of scope

- DEM / Z (**T-091**)
- Hillshade (**T-091.2**)
- Arland assets (defer until Everon gate PASS)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Coordinate system | `COORDINATE_SYSTEM.CARTESIAN`, `flipY: false`, origin bottom-left |
| Tile URL | Manifest `tiles.urlTemplate` — default `/map-assets/everon/tiles/{z}/{x}/{y}.webp` |
| Zoom range | Manifest `minZoom`–`maxZoom` (default 0–5) |
| Layer order | Basemap **below** grid lines |
| Toggle | Basemap on/off lands in **T-091.2** (`MissionSettingsDialog`) — optional stub prop @ T-090.1 |

---

## Implementation specification

### Files to touch

| File | Change |
|------|--------|
| `layers/useBaseMapLayer.ts` | Fetch manifest; compose basemap + grid layers |
| `layers/useTerrainBasemapLayer.ts` | **New** — BitmapLayer or Cartesian TileLayer |
| `coords/terrainManifest.ts` | **New** — parse manifest JSON (shared with T-091.1) |
| `TacticalMap.tsx` | Pass `terrain` id; remount on change (existing `key`) |

### Horizontal alignment tests (required @ verify)

| ID | Test | Method |
|----|------|--------|
| **H1** | Grid origin | World (0,0) pixel = southwest map corner tile pixel |
| **H2** | Landmark | Pick 1 Biki-known coordinate (e.g. airfield center) — tile color/feature within **≤50 m** of expected world point @ z3 |
| **H3** | Bounds clip | Pan beyond 12800 m — basemap does not draw outside bounds |

Document measured H1/H2 results in PR / manual verify log.

---

## Verification gate (mandatory)

**Ship T-090.1 only when ALL PASS.**

### Automated

```bash
cd apps/website/frontend && npm run build && npm run lint
make verify-terrain
# Requires tiles on disk (T-090.1 / T-121 — not T-091.0):
test -d packages/map-assets/everon/tiles/0
```

### Manual (browser)

| ID | Step | Pass condition |
|----|------|----------------|
| M1 | Open `/missions/:id/edit` Everon | Basemap visible under grid |
| M2 | Compare to Reforger map screenshot | Coastline/airfield roughly aligned @ default zoom |
| M3 | Rename tile dir → 404 | Grid-only + toast; editor usable |
| M4 | Pan/zoom | ≥55 fps with basemap (no regression vs T-057) |
| M5 | Switch terrain Arland | Grid only (no Everon tiles on wrong terrain) |

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | Build/lint | `npm run build && npm run lint` exit 0 |
| S2 | Cartesian | No `@deck.gl/geo-layers` TileLayer without explicit bbox |
| S3 | H1/H2 | Manual log attached — horizontal alignment documented |
| S4 | Degraded | M3 pass |
| S5 | Perf | M4 pass |

---

## Related

- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — tile export runbook
- [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md) — basemap toggle in settings
