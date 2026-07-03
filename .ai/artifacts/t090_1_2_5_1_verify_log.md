# T-090.1.2.5.1 — Inland water mask refine: verify log

**Slice:** T-090.1.2.5.1 · **Executor:** claude-code · **Date:** 2026-07-03
**Parent ship:** T-090.1.2.5 @ `6396960f` · Refine spike: [`t090_1_2_5_1_refine_spike.json`](t090_1_2_5_1_refine_spike.json)

## Param table (old → new)

| Param | .2.5 | .2.5.1 | Why |
|-------|------|--------|-----|
| `FLAT_FRAC_MAX` (compact class) | 0.5 | **0.12** | Operator FP town pavement measured flatFrac 0.30–0.46 (engine-graded pads); central lake 0.031, rivers ~0–0.06 |
| Linear class | — (n/a) | **new**: ribbonWidth = 2·area/perimeter ≤ 5 px | Rivers/streams are thin ribbons; compact rules don't fit them |
| — grey-river sub-class | — | area ≥ 800 m², slope ≤ 16°, flatFrac ≤ 0.2, valleyFrac ≥ 0.2 **or** slope ≤ 8° | Engine water appearance (meanSat ≤ 0.115, same trust as .2.5); soft guard keeps hillside grey roads out |
| — wet-channel sub-class | — | area ≥ 1200 m², **valleyFrac ≥ 0.7**, meanSat ≤ 0.16, meanLum ≤ 0.28, slope ≤ 16°, flatFrac ≤ 0.2 | The refine's new reach (mountain streams): dark wet carved channels; hillside road cuts have no symmetric DEM carve; dry dirt tracks are lighter |
| Wet pixel band | — | lum 0.10–0.30, sat < 0.16, slope ≤ 24°, valley-gated px only | Streams are dark wet rock, not seabed grey |
| Valley carve | — | boxBlur(DEM, 48 m) − DEM > 0.8 m | Engine-designed watercourse channels; spec-sanctioned lever |

Ocean mask **A (DEM ≤ 0)** byte-for-byte unchanged (out of slice scope).

## Refine spike results

- Components: 3302 total, **114 accepted** (lake + 25/38 `.2.5` bodies retained + 85 new stream/marsh segments + wet channels).
- **All 13 dropped `.2.5` bodies are pavement-class** (flatFrac 0.20–0.48) — includes every operator FP body; the largest dropped body (4878, 6952) was crop-verified as a village street web, not river.
- **Operator FP sites — 0 accepted bodies within 250 m of all four:** (4514, 9530), (4836, 9224), (4366, 9304), (4776, 9268).
- New mountain water (examples, world coords): grey-river @ (8318, 2388) valleyFrac 0.98; wet-channel @ (9844, 4446) valleyFrac 1.00; wet-channel @ (9676, 5024) valleyFrac 0.97; coastal lagoon channels @ (~9850, 4400).
- Every accepted body audited via 114-crop montage: winding valley ribbons, marsh channel systems, the lake — no pavement shapes.
- **R3 probe (timeboxed ~20 min):** `Eden.topo` coordinate encoding cracked — big-endian float32 world-coord pairs (0–12800 range, shared vertices = closed polylines); record framing/typing undecoded → road-corridor subtraction unavailable this slice. Best offline vector lead for T-090.8.

**Forbidden-methods attestation:** no hand-paint, no AI rivers, no solid rectangles. All refine levers are engine data: DEM exact-flat grading, DEM valley carve, DEM slope, engine-rendered supertexture appearance.

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

$ cd apps/website/frontend && npm run build && npm run lint
build clean (pre-existing chunk-size warning only) · lint clean
```

## Manual acceptance R-FP / R-FN / R-REG (self-check; operator to confirm)

| ID | Check | Self-check |
|----|-------|-----------|
| **R-FP1** | Road corridor @ ~(4776, 9268) grey not blue | **PASS** — [`t090_1_2_5_1_crop_fp_road.png`](t090_1_2_5_1_crop_fp_road.png): town pavement/yards grey; the real river ribbon through the same crop still blue |
| **R-FP2** | ≥2 more flagged pad sites grey | **PASS** — (4514, 9530), (4836, 9224), (4366, 9304) all rejected (spike `comparison.dropped`); 0 accepted bodies within 250 m of any flagged site |
| **R-FN1** | ≥2 hill streams newly water | **PASS** — mountain valley stream chain @ (~8300, 2400) ([`t090_1_2_5_1_crop_stream_valley.png`](t090_1_2_5_1_crop_stream_valley.png)) + coastal lagoon channel system @ (~9850, 4400) ([`t090_1_2_5_1_crop_stream_lagoon.png`](t090_1_2_5_1_crop_stream_lagoon.png)); more in spike `newBodies` (85) |
| **R-REG1** | Central lake still water | **PASS** — [`t090_1_2_5_1_crop_lake.png`](t090_1_2_5_1_crop_lake.png), 18.9 ha @ (4618, 5972) |
| **R-REG2** | Coast/ocean unchanged | **PASS** — ocean mask algorithm untouched; [`t090_1_2_5_1_crop_coast.png`](t090_1_2_5_1_crop_coast.png) matches the .2.5 coast crop |
| **R-REG3** | Land byte-identity | **PASS** — `land px: 51,068,208  changed on land: 0  changed in water: 112,771,792` (full-res pre-water vs composited against exact water mask; feather is inward-only) |

Known trade (documented): 13 `.2.5` bodies dropped — all pavement-class; the flatFrac 0.12 cap also drops flat coastal wetland fragments that rode in with .2.5's looser cap. Operator keep-list (ocean, lake, lowland rivers) fully retained (25/38 + segment fragmentation differences absorbed into the 85 new bodies).

## LFS churn note

Same class as .2.5, expected: `everon-sat.tbd-sat` rewritten (152.8 MB) + `tiles/satellite/**` z0–6 + `full.webp` (5,462 lossless WebP files).
