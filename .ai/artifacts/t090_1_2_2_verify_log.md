# T-090.1.2.2 — SAP cell seam repair · verify log

**Terrain:** everon · **Date:** 2026-07-01 · **Executor:** claude-code · **Parent:** T-090.1.2.1 lossless pyramid @ `19bc785`

Repaired the operator-visible ~256 m grid seams in the stitched SAP satellite ortho and rebuilt the
identical lossless z0–6 pyramid. **Strategy A — apron-bridge feather**, `A-apron-bridge-4px`
(`halfWidthPx:4, anchorOffsetPx:5, interiorSeamsOnly:true`).

## Root cause (P0, proven read-only)

Each Eden `_supertexture.edds` cell carries a baked **constant ~3–4 px apron on all four edges**
(mip0) — confirmed in a standalone decoded cell (`cell-0.png`: horiz/vert gradient 0.00 at cols/rows
0–2 & 252–255, ~1.9–3.9 interior). Tiled edge-to-edge by `stitch-sap-ortho.mjs`, adjacent aprons
stack into an **~8 px dead-flat band at every interior 256 px seam** → a grid of blurry lines +
crosses over sharp terrain at max zoom. **Not** exposure (cross-seam ΔRGB ≈ 0.1 on the broken ortho)
and **not** placement (boundaries land exactly on 256·k; no off-by-one/mirror). Decode/BC7 untouched.

**Fix:** `bridgeSeams` (`blend-sap-seams.mjs`) rewrites the dead band at each interior seam, per line,
as a linear cross-fade between the nearest **detailed** lines on each side (anchors at `c-5` / `c+4`),
V-then-H so 4-cell crosses fill from the V-bridged columns. The apron held no detail, so the bridge
loses nothing; it removes the flat strip and its sharp bounding edges (the visible line). Detail is
**not** invented — a smooth ~8 px transition remains (inherent; forbidden to upscale/inpaint). World
borders (x/y = 0, 12800) are skipped. Runs in final north-up canvas space; the row-flip assembly is
untouched.

## Metric pivot rationale (NIT-2)

The slice brief prescribed a **"grid-edge mean ΔRGB"** gate, which assumes an **exposure step**. P0
proved the artifact is a **baked-apron flat band**, so cross-seam ΔRGB is already ≈ 0.1 on the broken
ortho — that metric cannot see this bug. The gate therefore measures the **flat band** directly:

- **Primary (contrast-invariant): apron removed.** The contiguous flat run (apron) at each textured
  seam must be ≤ 1 line. A linear bridge breaks any flat run regardless of local contrast, so this is
  the robust invariant signal. (baseline apron 4/3 → post-fix **0/0** on all 98 seams.)
- **Secondary (relative recovery):** the band must recover to **≥ REL_FLOOR (0.05)** of the *local*
  interior gradient. A linear bridge yields gradient **∝ local contrast**, so an absolute floor would
  unfairly fail genuinely low-contrast coastal seams; the relative floor is contrast-invariant.
- **Reported (not a hard fail):** absolute `bandMinGrad ≥ FILL_FLOOR (0.25)` — met by **90/98** seams
  (the 8 below are all low-contrast west/east coastal edges, k=1–3 / 47–49; ratio 0.077–0.096).
- **Guards:** cross-seam ΔRGB ≤ **STEP_CAP (6.0)** (no new exposure step); global stddev floor 0.02
  (no whole-map blur); NIT-1 anchor safety; interior control lines stay textured.

Seams in genuinely uniform terrain (interior gradient ≤ **DETAIL_MIN 1.0**, e.g. open water) are not
evaluated — a flat band there is legitimate and invisible.

## Before / after (worst textured seam over all 98 interior seams)

| Metric | Baseline (broken) | Post-fix | Notes |
|--------|-------------------|----------|-------|
| diagnosis | `baked_apron_flat_band` | `clean` | |
| worst apron (contiguous flat run) | **4 / 3** | **0 / 0** | flat strip fully removed on all 98 |
| worst recovery ratio (band/interior) | ~0.00 | **0.08** | ≥ REL_FLOOR 0.05 |
| worst bandMinGrad | **0.00** | **0.12** | abs 0.25 met on 90/98 |
| mean bandMinGrad (textured) | 0.00 | **0.39** | |
| textured seams still flat | **98 / 98** | **0 / 98** | fill failures |
| max cross-seam ΔRGB | 1.23 | 1.24 | ≤ STEP_CAP 6.0 (no new step) |
| global stddev | 0.0534699 | 0.0533914 | unchanged → no blur |
| interior controls bandMinGrad | 2.96 / 2.8 / 3.07 | 2.94 / 2.79 / 3.05 | interior detail intact |

Baseline metrics captured on the pre-fix ortho before re-stitch; post-fix from the shipped ortho
(`.ai/artifacts/t090_1_2_2_seam_analysis.json`).

## Locked thresholds

| Threshold | Value | Justification |
|-----------|-------|---------------|
| `REL_FLOOR` (primary numeric) | **0.05** | band must recover to ≥5% of local interior detail; observed post-fix 0.077–0.164; dead baseline ~0 |
| apron (primary) | **≤ 1** | contiguous flat run removed; baseline 4/3, post-fix 0/0 |
| `FILL_FLOOR` (reported) | **0.25** | absolute band-gradient reference; met by 90/98 (high/mid-contrast) |
| `STEP_CAP` (guard) | **6.0** | cross-seam ΔRGB cap; observed 1.24 |
| `DETAIL_MIN` (evaluate gate) | **1.0** | below this the neighbourhood is uniform (water) → not a seam artifact |
| `HW` (bridge half-width) | **4** | covers observed 3–4 px aprons; band = 8 px (spec "2–8 px") |

## NIT-1 — anchor safety spot-check

Apron width at the extreme seams k=1 / k=49 (both axes) + worst textured seam. Anchors at `c-5` / `c+4`
must clear the apron:

| seam | baseline apron L/R | post-fix apron L/R | anchorSafe |
|------|--------------------|--------------------|------------|
| v k=1 | 4 / 3 | 0 / 0 | ✅ |
| v k=49 | 4 / 3 | 0 / 0 | ✅ |
| h k=1 | 4 / 3 | 0 / 0 | ✅ |
| h k=49 | 4 / 3 | 0 / 0 | ✅ |

Baseline apron never exceeded 4 anywhere (anchor-unsafe count 0 across all 98 seams) → HW=4 anchors clear
with margin; **no widening / HW reduction needed**.

## Shipped seam-repair meta (written to gitignored staging `TBD_SatExport_meta.json`; identical on the CLI fallback — NIT-4)

```json
"seamRepair": "T-090.1.2.2",
"seamRepairStrategy": "A-apron-bridge-4px",
"seamRepairParams": { "halfWidthPx": 4, "anchorOffsetPx": 5, "interiorSeamsOnly": true }
```

## Automated ship gates — all PASS

### S1 — `node scripts/map-assets/verify-sap-seams.mjs TERRAIN=everon` (exit 0)
```
  ok: ortho 12800x12800
  ok: FILL: flat band removed on all 98 textured seams — worst apron 0 (≤1), worst recovery 0.08 (≥ 0.05); abs bandMinGrad ≥ 0.25 on 90/98
  ok: STEP guard: max cross-seam ΔRGB 1.24 ≤ STEP_CAP 6
  ok: ANCHOR safety: all seams clear (apron never reached bridge anchors)
  ok: control interior lines textured (2.94, 2.79, 3.05)
  ok: global stddev 0.0534 (> 0.02)
verify-sap-seams OK
```

### S2 — `node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon` (exit 0)
```
  ok: catalog 2500 cells
  ok: meta source/cells/dims/mpp/bounds
  ok: ortho 12800x12800
  ok: ortho stddev 0.0534 (> 0.02)
  ok: orientation guard: ortho matches north-up DEM (AE ratio 0.078 < 0.2)
  ok: committed satellite/0/0/0.webp (75638 B)
verify-sap-ortho OK
```

### S3 — `EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon` (exit 0)
```
verify-tile-pyramid: OK everon — levels [0,1,2,3,4,5,6], 5461 tiles, 256px, 5461 VP8L lossless
```
Disk: `tiles/satellite` **302M** (z6 198M, full.webp 17M) — within the 250–450 MB lossless budget.
5266 of 5462 tiles changed (tiles clear of any seam are byte-identical).

### S4 — `make verify-terrain` (exit 0)
```
PASS  Manifest validates against terrain-manifest.schema.json
PASS  Manifest matches terrains.ts for everon
PASS  Anchors validate … DEM PNG 6400×6400
…anchor elevation verify: 11/11 PASS, maxDeltaM=0.204 thresholdM=1
verify-terrain-alignment: OK
```

### S5 — `make ci-local-frontend` (Node 26.4.0) (exit 0)
```
✓ built in 551ms
 Test Files  3 passed (3)
      Tests  26 passed (26)
```
No frontend source changed.

## Manual acceptance (operator) — NIT-3 bar

**Pass bar:** S1/S2 PASS = the **sharp grid line is gone**; a faint ~8 px soft ramp at extreme zoom is
**acceptable** (inherent — detail is not inventable from the source apron). The cross-point radial blend
is escalation only if a **hard line remains at 4-cell crosses** (not built in v1).

| ID | Criterion | Status / evidence |
|----|-----------|-------------------|
| **S1** | Former seam location — no visible line @ max zoom | **BACKED** — apron removed on all 98 seams (worst 0/0); pending operator eyeball |
| **S2** | 3 other intersections (road, forest, coast) | **BACKED** — coast = the low-contrast edge seams (k=1–3/47–49), all healed (apron 0/0, recovery ≥0.077); pending operator eyeball |
| **S3** | North-up + alignment unchanged | **PASS (backed)** — `verify-sap-ortho` orientation AE 0.078 < 0.2; assembly + bounds + tileUrl unchanged |
| **S4** | Pan/zoom ≥55 fps | **BACKED unchanged** — no FE change; identical pyramid structure (5461 tiles, 256px, VP8L); `ci-local-frontend` PASS; pending operator fps confirm |

S1/S2/S4 pixel/fps judgments require a human in the running editor (`make api` + `make web` → dev-login
mission_maker → Mission Creator → Satellite @ max zoom); S1–S3 are backed by the automated gates above.

## Files

- **New:** `scripts/map-assets/blend-sap-seams.mjs`, `scripts/map-assets/analyze-sap-seams.mjs`,
  `scripts/map-assets/verify-sap-seams.mjs`, `scripts/map-assets/lib/sap-seam-metrics.mjs`
- **Edit:** `scripts/map-assets/stitch-sap-ortho.mjs` (import + `bridgeSeams` call + meta bump)
- **Rebuilt (LFS):** `packages/map-assets/everon/tiles/satellite/**` (lossless z0–6, 5266 tiles changed)
- **Artifacts:** `.ai/artifacts/t090_1_2_2_seam_analysis.json` (post-fix), this log
- **Not committed:** staging ortho + `TBD_SatExport_meta.json` (gitignored); `manifest.json` untouched
  (Ajv `strict` schema — provenance lives in the stitch script + staging meta + this log)
