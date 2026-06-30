# T-090.1.2.1 — Lossless satellite pyramid · verify log

**Terrain:** everon · **Date:** 2026-07-01 · **Executor:** claude-code · **Parent ortho:** T-090.1.2 @ `c2730a3`

Rebuilt the Everon Satellite tile pyramid from the staged 12800² SAP ortho as **lossless WebP (VP8L)
z0–6**, replacing the lossy `cwebp -q 80` z0–5 pyramid. Manifest `maxZoom` 5 → 6;
`tiles.satellite.encoding: "webp-lossless"`.

## Build

```
scripts/map-assets/build-tile-pyramid.sh \
  --input  packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out    packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
→ [pyramid] source 12800x12800; tile=256 enc=lossless zoom 0..6
→ [pyramid] wrote 5461 tiles to packages/map-assets/everon/tiles/satellite
→ [pyramid] OK  0/0/0.webp + full.webp present   (BUILD_EXIT=0)
```

Ortho **not** re-stitched/re-decoded (SAP pipeline shipped @ `c2730a3`); `--lossless`, no `--flip-v`.

### Build-script performance fix (necessary to ship)
The original per-tile loop **re-decoded the entire level PNG for every tile** (`magick "$LV" -crop
…+offx+offy` per tile). At z6 that is a 16384² (268 Mpx) decode ×4096 ≈ **~4 s/tile → ~4 hours**;
the first full run was aborted mid-z6. Rewrote the inner loop to **single-pass crop** (`magick -crop
256x256 +adjoin tile_%d.png` → one decode per level, row-major scene index `i` ⇒ `x=i%n, y=i/n`)
plus **parallel `cwebp` across all cores** (`xargs -P$(nproc)`). Proven byte-identical to the old
per-tile crop: z2 16/16 tiles and a full z0–3 rebuild **85/85 tiles `cmp` clean** vs the prior
output. Full z0–6 lossless rebuild now completes in minutes.

## Disk

```
du -sh packages/map-assets/everon/tiles/satellite   → 299M
du -sh packages/map-assets/everon/tiles/satellite/6 → 195M
full.webp (4096px, VP8L)                            →  18M
```
Within the spec LFS budget (250–450 MB lossless).

### Tile counts
| z | tiles | expected (2^z)² |
|---|-------|-----------------|
| 0 | 1 | 1 |
| 1 | 4 | 4 |
| 2 | 16 | 16 |
| 3 | 64 | 64 |
| 4 | 256 | 256 |
| 5 | 1024 | 1024 |
| 6 | **4096** | 4096 |
| **pyramid total** | **5461** | 5461 |

`find tiles/satellite -name '*.webp' | wc -l` = **5462** (5461 pyramid + `full.webp`).

### Codec spot-check (`head -c16 | tail -c5`)
`0/0/0`, `6/0/0`, `6/63/63`, `6/32/32` → all **`PVP8L`** (VP8L lossless). No VP8 lossy tiles.

## Automated ship gates — all PASS

### Gate 1 — `node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon` (exit 0)
```
  ok: catalog 2500 cells
  ok: meta source/cells/dims/mpp/bounds
  ok: ortho 12800x12800
  ok: ortho stddev 0.0535 (> 0.02)
  ok: orientation guard: ortho matches north-up DEM (AE ratio 0.078 < 0.2)
  ok: committed satellite/0/0/0.webp (75638 B)
verify-sap-ortho OK
```

### Gate 2 — `EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon` (exit 0)
```
verify-tile-pyramid: OK everon — levels [0,1,2,3,4,5,6], 5461 tiles, 256px, 5461 VP8L lossless
```
(Hardened: complete `[minZoom..maxZoom]` levels required + VP8L assertion — a VP8 lossy tile now fails.)

### Gate 3 — `make verify-terrain` (exit 0)
```
PASS  Manifest validates against terrain-manifest.schema.json
PASS  Manifest matches terrains.ts for everon
verify-terrain-manifest: OK
PASS  Anchors validate (…/everon/anchors/verification.json)
PASS  DEM PNG 6400×6400
…anchor elevation verify: 11/11 PASS, maxDeltaM=0.204 thresholdM=1
verify-terrain-alignment: OK
```
(Schema accepts the new `tiles.satellite.encoding` enum; Ajv `strict:true` clean.)

### Gate 4 — `make ci-local-frontend` (Node 26.4.0) (exit 0)
```
✓ built in 518ms
 Test Files  3 passed (3)
      Tests  26 passed (26)
```
No frontend source changed (`computeLod` already clamps to manifest `maxZoom`, will select z6).

### Gate 5 — `node scripts/map-assets/verify-spike-ops-log.mjs TERRAIN=everon` (exit 0)
```
verify-spike-ops-log: OK (K7 + K2/K3/K4 gate↔artifact)
```

## Manual acceptance (operator, in-browser)

| ID | Criterion | Status / evidence |
|----|-----------|-------------------|
| **L1** | Max deck zoom on field/road — pixel-sharp, no watercolor blur | **PENDING OPERATOR** — z6 = 0.78 m/px lossless VP8L now backs deck zoom 6 (was z5 1.56 m/px lossy stretch); requires eyeball confirm |
| **L2** | North-up unchanged (airfield N, mountains SE) | **Backed PASS** — `verify-sap-ortho` orientation guard AE ratio 0.078 < 0.2; `tileUrl` single Y-flip + bounds unchanged |
| **L3** | H1/H2 click alignment unchanged | **Backed PASS** — no coords/bounds/`tileUrl` change; `verify-terrain` anchors 11/11 PASS (maxΔ 0.204 m) |
| **L4** | Pan/zoom ≥55 fps | **PENDING OPERATOR** — `MAX_VISIBLE_BASEMAP_TILES=64` cap + LOD memoization unchanged; requires in-browser fps check |

L1/L4 are pixel/fps judgments that require a human in the running editor; L2/L3 are backed by the
automated orientation + anchor gates above.
