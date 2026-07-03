# T-090.1.2.5.2 — Exact road-geometry water refine + one-button pipeline: verify log

**Slice:** T-090.1.2.5.2 · **Executor:** claude-code · **Date:** 2026-07-03
**Parents:** T-090.1.2.5 @ `6396960f` · T-090.1.2.5.1 @ `82488c6f`
**Source spike:** [`t090_1_2_5_2_source_spike.json`](t090_1_2_5_2_source_spike.json)

## Spike verdict — G1-B (with the road network decoded EXACTLY)

`Eden.topo` fully decoded (`scripts/map-assets/decode-topo.mjs`, offline pak read):

- **Format:** header 0x18 B (`@0x10` sectionCount 6, `@0x14` recordsPerSection 888); record = `[u8 type][u32 vertexCount][count × (f32LE x, f32LE y)][u32 K][K × u32 attrs]`; sections 2–6 = LOD re-encodings; **y axis = north-up image metres**. Trailing `PWLN` group = 2-vertex spans (powerlines) — undecoded, not needed.
- **All five type classes are ROAD/AIRFIELD line work** (type 0 = runway lines, 0.833 engine-flat overlap — orientation proof; types 1/2/3/5 = road hierarchy). **No hydro layer in `.topo`** — colour-overlay showed type-1 routes crossing ridges and connecting airfield/towns; the flow-monotonicity test confirmed no watercourse-consistent class.
- **Consequence:** exact **road-corridor subtraction**. The residual `.2.5.1` FP bodies measured **roadFrac 0.72–0.84** (a `.topo` polyline runs them end-to-end) vs real water: lake **0.001**, SE river **0.163** — the component-level guard (`ROAD_OVERLAP_MAX 0.45`) separates them deterministically. Full-res inspection of the largest contested ribbon @ (3978, 4706) confirmed **road** (uniform width, asphalt tone, junctions) — i.e. several `.2.5/.2.5.1`-era "rivers" were themselves road FPs; this slice removes them correctly.
- Pixel-level road exclusion was tried and REJECTED (fragmented true rivers that run beside roads); the shipped mechanism is the component-level `roadFrac` guard.

## Mask recipe (all engine data)

| Layer | Source |
|---|---|
| Ocean | DEM ≤ 0 m (UNCHANGED) + depth ramp |
| Compact bodies (lake/marsh/lagoon) | supertexture water appearance + `.2.5.1` compact rules (flatFrac ≤ 0.12 …) |
| Grey-river linear | `.2.5.1` rules + **roadFrac ≤ 0.45 guard** |
| Wet-channel linear (streams/gullies) | RELAXED (operator call — carved watercourses read as water even when seasonally dry): valleyFrac ≥ 0.6 (was 0.7), meanSat ≤ 0.18 (was 0.16), meanLum ≤ 0.31 (was 0.28), area ≥ 1000 m² + roadFrac guard |
| Road subtraction | `.topo` corridors, half-widths px@4 m: type0=3, 1=2, 2=2, 3=1, 5=1 |

Result: **153 accepted bodies** (6 compact + 24 grey-river + 123 wet-channel), montage-audited; 46 `.2.5.1` bodies dropped — all road-riding (roadFrac > 0.45) or road-guard adjacents; **operator FP sites (incl. the (4617, 8711) viewport and (4754, 8870) body): zero accepted bodies**.

**Forbidden-methods attestation:** no hand-paint, no AI rivers, no solid rectangles. New lever = the engine's own `.topo` road vector network; everything else = engine supertexture appearance + engine DEM.

## One-button pipeline (operator requirement)

**`make map-water-everon`** = restore pre-water ortho → strip meta → mask build (spike JSON refresh) → composite → unified bundle → **manifest `unified.bytes` auto-patch** → lossless pyramid → verify-sap-ortho + verify-unified-satellite + EXPECT_LOSSLESS verify-tile-pyramid. **This ship ran through the button end-to-end (exit 0).** Terrain-parameterized: `decode-topo.mjs` `TOPO_TERRAINS` config (Arland row pre-seeded); new terrain = config entry + one-time DEM export (T-091.0 flow).

## Automated gates (all exit 0)

```
$ make map-water-everon   (button run — tail)
verify-sap-ortho OK          (orientation guard AE ratio 0.003 < 0.2)
verify-unified-satellite: OK everon — 12800x12800, 14 mips, 17 VP8L blocks, 152.7 MB
verify-tile-pyramid: OK everon — levels [0,1,2,3,4,5,6], 5461 tiles, 256px, 5461 VP8L lossless

$ make verify-terrain
verify-terrain-alignment: OK   (maxDeltaM 0.204, anchors unchanged)

$ cd apps/website/frontend && npm run build && npm run lint
build + lint clean
```

## Byte-identity (R-REG3)

```
land px: 50,967,731   changed on land: 0   changed in water: 112,872,269
R-REG3 PASS — land byte-identical (inward-only feather)
```

## Manual acceptance (self-check; operator to confirm)

| ID | Check | Self-check |
|----|-------|-----------|
| **R-FP viewport** | (4617, 8711) viewport — no blue on roads/paths | **PASS** — [`t090_1_2_5_2_crop_operator_viewport.png`](t090_1_2_5_2_crop_operator_viewport.png): every grey ribbon (all `.topo`-traced roads) stays grey |
| **R-FN gullies** | carved channels gain water | **PASS** — [`t090_1_2_5_2_crop_gully_west.png`](t090_1_2_5_2_crop_gully_west.png) + [`t090_1_2_5_2_crop_gully_se.png`](t090_1_2_5_2_crop_gully_se.png): valley watercourses blue, adjacent roads grey; 123 wet-channel bodies vs 108 in `.2.5.1` at stricter quality |
| **R-REG lake** | central lake intact | **PASS** — [`t090_1_2_5_2_crop_lake.png`](t090_1_2_5_2_crop_lake.png), 19.3 ha, roadFrac 0.001 |
| **R-REG ocean** | unchanged | **PASS** — ocean algorithm untouched; guard AE 0.003 |
| **R-REG land** | byte-identity | **PASS** — 0 / 50,967,731 changed |

**Expected visual change vs `.2.5.1` (heads-up for operator):** several long blue ribbons from earlier ships are now GREY — the `.topo` decode + full-res inspection proved them to be roads (the exact class the FP report flagged). Everon's mapped water = lake + valley stream/gully network + coastal wetlands; there is no long lowland river in the engine's own road-vs-terrain data that we can substantiate. If BI's official map shows named rivers the composite still misses, the exact-geometry escalation path is `Eden.ent` water entities via Workbench export (T-090.8) — `.topo` definitively does not carry them.

## LFS churn note

Same class as prior slices: `everon-sat.tbd-sat` rewritten (152.7 MB) + 5,462 lossless WebP tiles.
