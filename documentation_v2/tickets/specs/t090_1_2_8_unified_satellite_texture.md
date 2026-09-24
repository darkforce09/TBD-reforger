# T-090.1.2.8 — Unified satellite texture (binary + GPU mips, no tile flicker)

**Ticket:** T-090 · **Slice:** T-090.1.2.8  
**Status:** **SHIPPED** @ `db9057ef` (tag **T-090.1.2.8**) · format **tbd-sat v1** · source = SAP ortho (T-090.1.2.4 @ `0d6fe485` P0 FAIL)  
**Executor:** claude-code (website + root scripts)  
**Depends on:** **T-090.1.2.4** shipped FAIL · input = `everon-sap-ortho.png` + lossless pyramid encode path  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Deliver Satellite like **Arma Reforger’s map feel**: **one continuous texture** with **GPU mipmaps** for smooth zoom — **not** 5461 separate WebP tiles that fetch/decode/pop on pan.

---

## Problem (operator-reported)

Mission Creator Satellite today:

| Symptom | Cause |
|---------|--------|
| **Tile pop-in / flicker** on pan | `useTerrainBasemapLayer.ts` mounts up to **64** `BitmapLayer`s; each `{z,x,y}.webp` fetches + decodes on LOD change |
| **Visible grid at max zoom** | SAP source (baked aprons) — **not fixable** without new source (T-090.1.2.4 FAIL) | Accept at max zoom; invisible at game-map zoom |
| **302M LFS / 5461 files** | Lossless pyramid is correct for CDN-style maps; **wrong shape** for a single-user editor viewport |

**Arma Reforger in-game map:** samples a **virtualized terrain color texture** — zoom feels continuous; no discrete 256 px web tiles reloading in the editor UX sense.

---

## Goal (110% bar)

1. **One asset** per terrain satellite (Everon first): binary or single lossless file + **embedded mip chain** (not 5461 HTTP requests).
2. **Frontend:** one GPU texture (or ≤4 chunked uploads), **linear mip filtering** — zoom/pan without layer mount/unmount flicker.
3. **Maintain** alignment: `worldBounds [0,0,12800,12800]`, north-up, `metersPerPixel: 1`.
4. **Performance:** pan ≥55 fps @ max zoom; initial load acceptable (progress UX if >50 MB).
5. **Fallback:** keep pyramid path behind manifest flag until `.2.8` verified.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| **Primary mode** | `manifest.tiles.satellite.delivery: "unified"` (new) vs legacy `"pyramid"` |
| **Mip generation** | Prefer **GPU** or **build-time** mip chain from ortho — not runtime tile pyramid |
| **Binary format v1** | Custom **`tbd-sat`** or **KTX2** — spike picks one in P0 (document in artifact) |
| **Max dimension** | 12800 base (match engine ortho); mips down to ≤256 |
| **Chunking** | If single file >100 MB: **spatial chunks in one bundle** with index header — still **one fetch**, not per-viewport tile HTTP |
| **T-090.1.2.3** | **Superseded for 110%** — prefetch/cache remains interim for legacy pyramid only |

---

## Format sketch (v1 — implementer spike)

```text
tbd-sat v1 header:
  magic, version, terrainId, worldBounds, baseWidth, baseHeight, mipCount, encoding
  mip[n]: offset, length, width, height
payload: contiguous RGBA or BC7/WEBP blocks per mip (lossless VP8L acceptable inside bundle)
```

Manifest extension (Ajv update in **T-090.0.2** pattern):

```json
"satellite": {
  "delivery": "unified",
  "path": "satellite/everon-sat.tbd-sat",
  "url": "/map-assets/everon/satellite/everon-sat.tbd-sat",
  "source": "engine-render-ortho",
  "encoding": "tbd-sat-v1"
}
```

---

## Implementation phases

### P0 — Format spike

- Compare: `full.webp` + browser mipmaps vs bundled RGBA mips vs KTX2
- Pick format; document in `.ai/artifacts/t090_1_2_8_format_spike.json`

### P1 — Build script

- `scripts/map-assets/build-unified-satellite.mjs` — input ortho PNG → `everon-sat.tbd-sat`
- Verify script: magic, mip count, byte-exact mip dimensions

### P2 — Frontend loader

- `satelliteUnifiedTexture.ts` — fetch once → decode → `luma.gl` `Texture` with mips
- `useTerrainBasemapLayer.ts` — branch: unified = **one** `BitmapLayer` or custom layer sampling texture with zoom-based LOD (no per-tile URLs)

### P3 — Ship

- LFS commit bundle; manifest `delivery: unified`; manual: pan @ max zoom — **no pop-in**

---

## Out of scope

- Mission object binary base — **T-110**
- Map cartographic pyramid — **T-090.1.1** (may reuse unified delivery later)
- CDN tile server for public website maps

---

## Manual acceptance

| ID | Pass |
|----|------|
| **U1** | Pan @ max zoom — no tile pop-in / flicker (operator) |
| **U2** | Zoom in/out — smooth (GPU mip), no discrete layer swap |
| **U3** | Smooth zoom; detail acceptable @ operational zoom levels (SAP source — grid may show @ max MC zoom) |
| **U4** | Pan fps ≥55 |

---

## Related

- Source ortho: **SAP** (T-090.1.2.2 @ `a3efdf6`) — engine ortho **FAIL** @ `0d6fe485`
- Spike: [`.ai/artifacts/t090_1_2_4_engine_render_spike.json`](../../../.ai/artifacts/t090_1_2_4_engine_render_spike.json)
- Legacy pyramid: **T-090.1.2.1** · interim prefetch **T-090.1.2.3**
- Long-term terrain objects: **T-110**

---

## Claude Code prompt — T-090.1.2.8 (copy-paste)

Extract: `./scripts/ticket prompt T-090`

```
Read CLAUDE.md first.

Implement **T-090.1.2.8** — unified satellite texture (binary + GPU mips, no tile flicker).

═══ PREFLIGHT ═══
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090
  test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png

═══ READ ═══
  1. .ai/artifacts/t090_1_2_8_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_2_8_unified_satellite_texture.md
  3. .ai/artifacts/t090_1_2_4_engine_render_spike.json  (source locked = SAP)
  4. apps/website/frontend/src/features/tactical-map/layers/useTerrainBasemapLayer.ts
  5. packages/map-assets/everon/manifest.json

═══ PROBLEM ═══
  Satellite pan/zoom flickers — 5461 WebP tiles, up to 64 BitmapLayers. T-090.1.2.4 proved
  no cleaner engine ortho source. Pivot: one bundled texture + GPU mip chain (Reforger feel).
  Input = existing SAP ortho (12800² staging PNG or pyramid rebuild into bundle).

═══ SHIPPED (do not reopen) ═══
  - T-090.1.2.4 @ 0d6fe485 — engine ortho P0 FAIL
  - T-090.1.2.2 @ a3efdf6 — SAP bridge (source)
  - T-090.1.2.1 @ 19bc785 — pyramid encode reference

═══ LOCKED ═══
  - manifest delivery: "unified" vs legacy "pyramid" flag
  - P0 format spike → tbd-sat v1 or KTX2 (document in artifact JSON)
  - Keep pyramid fallback until U1 verified
  - No docs/registry edits

═══ DO ═══
  1. P0 — format spike JSON (.ai/artifacts/t090_1_2_8_format_spike.json)
  2. build-unified-satellite.mjs — SAP ortho → everon-sat.tbd-sat (or chosen format)
  3. verify-unified-satellite.mjs
  4. Frontend loader + useTerrainBasemapLayer unified branch (one GPU texture, mips)
  5. Manifest extension + LFS ship path
  6. .ai/artifacts/t090_1_2_8_verify_log.md — U1–U4
  7. Tag **T-090.1.2.8** · prefix **T-090.1.2.8:**

═══ VERIFY ═══
  node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
  cd apps/website/frontend && npm run build && npm run lint

═══ MANUAL ═══
  U1: pan @ max zoom — no tile pop-in
  U2: zoom in/out — smooth GPU mip
  U4: pan fps ≥55

═══ RETURN ═══
  - SHA + tag T-090.1.2.8 · format choice · **Ready for Cursor doc sync.**
```
