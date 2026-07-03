# T-090.1.2.5 — Satellite water composite: verify log

**Slice:** T-090.1.2.5 · **Executor:** claude-code · **Date:** 2026-07-03

## Mask provenance (P0 spike summary)

Spike artifact: [`t090_1_2_5_water_source_spike.json`](t090_1_2_5_water_source_spike.json)

| Mask | Source | Provenance |
|------|--------|------------|
| **Ocean** | **A — DEM ≤ sea level** | Engine `GetTerrainSurfaceY` DEM (T-091.0), 68.5 % of frame ≤ 0 m; depth drives the `oceanDark→oceanBright` ramp (engine palette from `TBD_SatelliteExportPlugin.c` `SetupColors`) |
| **Inland** | **E — supertexture water appearance, DEM-filtered** | Pixels the engine's own SAP renderer drew with the underwater/seabed treatment (smooth desaturated grey, no vegetation colour), cross-filtered by engine DEM: coast exclusion, engine-exact-flat pad rejection (kills the NW-airfield runways + graded bases), slope rejection ≤18°/8° (kills SE-massif rock faces), 8 m density opening (kills roads/roofs), min-area 2000 m². **38 accepted bodies** — central lake 18.8 ha @ world (4618, 5972), the island river network, coastal wetlands — every body visually audited via crop montage before compositing. |

Blocked candidates (full trail in the spike JSON): **B** `Eden_<N>_layer.edds` decoded (BGRA8 splat weights, multi-block LZ4 — bit-exact) but the material palette is locked in `Eden_<N>.ttile`/`Eden.ent` (non-zlib pak codec; zlib/zstd/brotli/raw-LZ4 all fail); **C** `.Rivers/*_flow.edds` ×30 + `.Shore` atlas readable but placements live in `Eden.ent`; **D** Workbench was in game mode (WorldEditorAPI unavailable) and slice rules forbid new mod plugins — not needed after E passed.

**Forbidden-methods attestation:** no hand-painted lakes, no AI rivers, no solid rectangles. Every water pixel derives from engine data (DEM heights or engine-rendered supertexture pixels).

Composite parameters (also in staging `TBD_SatExport_meta.json` `waterComposite` block): water alpha **0.8** (SAP seabed/riverbed texture ghosts through), ocean ramp full-dark at 80 m depth, inland flat colour RGB(52, 88, 112), feather r=3 px **inward-only**.

## Automated gates (all exit 0)

```
$ node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
  ok: catalog 2500 cells
  ok: meta source/cells/dims/mpp/bounds
  ok: ortho 12800x12800
  ok: ortho stddev 0.0468 (> 0.02)
  ok: orientation guard: ortho matches north-up DEM (AE ratio 0.003 < 0.2)
  ok: committed satellite/0/0/0.webp
verify-sap-ortho OK

$ node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
verify-unified-satellite: OK everon — 12800x12800, 14 mips, 17 VP8L blocks, 152.8 MB

$ EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
verify-tile-pyramid: OK everon — levels [0,1,2,3,4,5,6], 5461 tiles, 256px, 5461 VP8L lossless

$ make verify-terrain
maxDeltaM=0.204 thresholdM=1
verify-terrain-alignment: OK

$ cd packages/tbd-schema && node scripts/verify-terrain-manifest.mjs
PASS  Manifest validates against terrain-manifest.schema.json
PASS  Manifest matches terrains.ts for everon

$ cd apps/website/frontend && npm run build && npm run lint
build clean (pre-existing chunk-size warning only) · lint clean
```

## Orientation-guard methodology (water-composite mode)

The legacy guard classified land as HSL saturation > 12 % — the blue ocean (68 % of frame)
would misread as "land" and blow the AE ratio. `verify-sap-ortho.mjs` now switches on
`meta.waterComposite`: land = NOT(blue-water hue window 0.50–0.68 with a 5 % saturation
floor), compared against the same north-up DEM sea mask at the same `ORIENT_MAX 0.2`.

**Why this is not circular:** the composite applied the DEM mask in ortho *pixel* space. If
any pipeline stage vertically flips the image (the T-090.1.2 upside-down bug class), the blue
coastline is mirrored while the independently-anchored DEM coast is not — the AE ratio jumps
from 0.003 to ~0.35 and the guard fails. The guard still catches exactly the failure mode it
was built for.

## W3 byte-identity proof (land untouched)

Full-resolution comparison of `everon-sap-ortho.pre-water.png` vs the composited ortho
against the exact water mask (DEM ≤ 0 ∪ inland mask):

```
land px: 51,150,808   changed on land: 0   changed in water: 112,689,192
W3 PASS — land byte-identical
```

The feather is inward-only (blurred alpha is re-clamped to the binary water mask), so no
land pixel can ever be tinted.

## Manual acceptance W1–W4 (self-check; operator to confirm)

| ID | Check | Self-check |
|----|-------|-----------|
| **W1** | Coast reads as water | **PASS** — [`t090_1_2_5_crop_coast.png`](t090_1_2_5_crop_coast.png): blue with depth ramp (bright shelf → dark deep), seabed texture ghosting through at α 0.8; no grey-seabed ambiguity |
| **W2** | ≥2 inland bodies visible | **PASS** — [`t090_1_2_5_crop_lake.png`](t090_1_2_5_crop_lake.png) central lake (18.8 ha @ world 4618, 5972) + [`t090_1_2_5_crop_river.png`](t090_1_2_5_crop_river.png) river network (NW stream, west river, central river through the lake continuing SE to the south coast) — 38 bodies total |
| **W3** | Land SAP unchanged outside mask | **PASS** — byte-identity proof above (0 changed land pixels); roads next to rivers stay grey, villages/fields untouched |
| **W4** | H1/H2 alignment + north-up unchanged | **PASS** — same [0,0,12800,12800] contract, no resample/flip anywhere (composite is per-pixel in place); orientation guard AE 0.003; `make verify-terrain` anchors maxDeltaM 0.204 unchanged |

## LFS churn note

Expected, per spec ("each ortho change triggers full pyramid rebuild"):
- `satellite/everon-sat.tbd-sat` rewritten — **152.8 MB** (was 205.9 MB; water compresses better than raw seabed)
- `tiles/satellite/**` z0–6 + `full.webp` rewritten — 5,462 lossless WebP files

## Post-ship operator feedback (2026-07-03)

Overall: **strong progress** — ocean, central lake, and main river network read well. Two inland-mask gaps remain for **T-090.1.2.5.1**:

| Gap | Report | Likely filter interaction |
|-----|--------|---------------------------|
| **Road false positives** | Paved roads / road-adjacent yards tinted blue (operator screenshots @ ~4776,9268 and similar) | Candidate **E** grey-detection treats desaturated asphalt like SAP “water appearance”; 8 m density opening drops *thin* roads but wide/connected paved areas survive `minAreaM2` 2000 m² and `flatFracMax` 0.5 |
| **Hill river false negatives** | Stream beds in mountainous terrain still dry | `slopeMeanMaxDeg` 8° + opening fragments steep narrow channels; lake/river *surface* sits above bed in DEM so bed slope ≠ channel mask |

W1/W2 bar met on audited crops; refine inland mask only (keep ocean **A** unchanged), then rebuild ortho + bundle.
