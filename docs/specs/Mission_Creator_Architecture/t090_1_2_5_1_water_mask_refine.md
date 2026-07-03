# T-090.1.2.5.1 — Inland water mask refine (road exclusion + hill rivers)

**Ticket:** T-090 · **Slice:** T-090.1.2.5.1  
**Status:** **SHIPPED** @ `82488c6f` (tag **T-090.1.2.5.1**) — operator post-ship: mixed (residual FP/FN; see below)  
**Executor:** claude-code  
**Depends on:** **T-090.1.2.5** shipped @ `6396960f` (inland mask **E** + ocean **A**)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · parent [`t090_1_2_5_satellite_water_composite.md`](t090_1_2_5_satellite_water_composite.md)

---

## In one sentence

Tighten the **inland** water mask from T-090.1.2.5 so **paved roads stop reading as blue** and **mountain stream beds gain water**, without changing the ocean mask or hand-painting hydrology.

---

## Operator report (2026-07-03)

First ship (**T-090.1.2.5** @ `6396960f`) is strong overall: ocean depth ramp, central lake, and lowland river network read well. Two gaps remain:

| Gap | Symptom | Example |
|-----|---------|---------|
| **Road false positives (FP)** | Grey asphalt / paved yards composited as inland water | Road corridors through farmland; rectangular pads near settlements (~world 4776, 9268) |
| **Hill river false negatives (FN)** | Stream beds in mountainous terrain stay dry | Narrow channels in SE massif / ridge valleys visible as dark terrain lines without blue tint |

Spike + verify log: [`.ai/artifacts/t090_1_2_5_water_source_spike.json`](../../../.ai/artifacts/t090_1_2_5_water_source_spike.json) · [`.ai/artifacts/t090_1_2_5_verify_log.md`](../../../.ai/artifacts/t090_1_2_5_verify_log.md)

---

## Root cause (locked — do not re-litigate mask family)

Inland mask **E** = SAP pixels matching engine “underwater/seabed” grey appearance ∩ DEM filters. Roads often share that grey signature. Current filters:

- 8 m density opening — removes *thin* grey streaks; **wide/connected pavement** survives
- `minAreaM2` 2000 — merged road networks form large components
- `flatFracMax` 0.5 — rejects runways, not sloped road segments
- `slopeMeanMaxDeg` 8 — rejects steep valley channels (hill rivers)

**Ocean mask A (DEM ≤ 0)** stays unchanged this slice.

---

## Goal

1. **Road FP:** operator-identified road corridors and paved pads **not** in final inland mask (regression crops PASS).
2. **Hill river FN:** at least **two** additional mountain stream segments visible vs `.2.5` ship (named in verify log with world coords + crops).
3. **No regression:** central lake + existing lowland rivers still water; ocean unchanged; land outside mask still byte-identical post-composite.
4. Rebuild ortho → unified bundle → lossless pyramid (same pipeline as `.2.5`).

---

## Approach (P0 spike → implement)

### P0 — Refine spike (mandatory)

Extend `analyze-water-sources.mjs` (or companion `refine-water-mask-spike.mjs`) → `.ai/artifacts/t090_1_2_5_1_refine_spike.json`:

| Track | Direction | Pass criteria |
|-------|-----------|---------------|
| **R1 — Road exclusion** | Per-component **elongation** (bbox aspect ratio), **perimeter/√area**, optional asphalt lum/sat band from known road calibration patches | FP bodies @ operator coords **rejected** in spike enum |
| **R2 — Hill rivers** | Relaxed slope for **linear** components; optional DEM **valley-carve** (local minima / drainage above sea level); lower min-area for high-aspect bodies | ≥2 new FN segments **accepted** without re-accepting FP roads |
| **R3 — .topo/.smap probe** (timebox ~45 min) | Brief header decode of `Eden.topo` / `Eden.smap` for road or drainage hints | Document PASS/FAIL; not required to ship if R1+R2 suffice |

Pick locked params in spike JSON before full-mask regen.

### P1 — Mask regen

1. **Restore** clean SAP ortho: copy `staging/sap/everon-sap-ortho.pre-water.png` → `everon-sap-ortho.png`; remove `waterComposite` block from `TBD_SatExport_meta.json` (composite script refuses double-apply).
2. Re-run refined `analyze-water-sources.mjs` → new `water-inland-mask.png` + updated spike cross-ref.
3. Re-run `composite-water-ortho.mjs` → composited ortho + meta.

### P2 — Rebuild + verify

Same gates as T-090.1.2.5:

```bash
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make verify-terrain
cd apps/website/frontend && npm run build && npm run lint
```

Append operator regression section to `.ai/artifacts/t090_1_2_5_1_verify_log.md`.

---

## Manual acceptance

| ID | Pass |
|----|------|
| **R-FP1** | Road corridor @ ~world (4776, 9268) — **no blue** on pavement (crop in verify log) |
| **R-FP2** | ≥2 additional operator-flagged road/pad sites — grey SAP, not blue |
| **R-FN1** | ≥2 hill stream segments newly blue vs `.2.5` baseline (crops + world coords) |
| **R-REG1** | Central lake @ (4618, 5972) still water |
| **R-REG2** | Coast/ocean unchanged vs `.2.5` |
| **R-REG3** | W3-style byte-identity on land outside mask |

Baseline comparison: diff against `.2.5` committed bundle crops or `pre-water` + old mask enum in spike.

---

## Post-ship operator review (2026-07-03)

Automated gates and **original regression coords** (town pads ~4776/9268, lake 4618/5972, SE hill streams) **PASS** per verify log. Full-map pan review: **mixed** — better in some areas, worse in others.

| Gap | Symptom | Lead coords / notes |
|-----|---------|---------------------|
| **Residual FP** | Linear blue overlay on non-water (field drains, ditches, grey paths) | Operator viewport **~(4617, 8711)** — blue segments on field/forest boundary; wet-channel class (85 new bodies) may over-accept dark linear terrain |
| **Residual FN** | Dry carved channels / gullies still without water tint | Same viewport — adjacent dark linear depressions without overlay; not all hill/lowland streams captured |
| **Trade** | `.2.5` flat wetlands dropped with pavement (flatFrac 0.12) | Documented in verify log — estuaries not on operator keep-list |

**Next levers (not this slice):** decode **Eden.topo** road polylines for corridor subtraction (**T-090.8** / R3 lead); per-body montage audit tightening on wet-channel thresholds; entity hydrology export (**T-090.3** / **T-090.8**). Optional follow-on slice **T-090.1.2.5.2** if operator wants another mask pass before **T-090.1.1**.

Artifacts: [`.ai/artifacts/t090_1_2_5_1_verify_log.md`](../../../.ai/artifacts/t090_1_2_5_1_verify_log.md) · [`.ai/artifacts/t090_1_2_5_1_refine_spike.json`](../../../.ai/artifacts/t090_1_2_5_1_refine_spike.json)

---

## Out of scope

- Hand-painted lakes / AI rivers / solid blue rectangles
- Ocean mask algorithm change
- New mod plugins; Workbench entity export (→ **T-090.8**)
- `layer.edds` palette decode (blocked @ `.2.5` spike)
- Docs/registry edits (Cursor sync after merge)

---

## Ship

Tag **`T-090.1.2.5.1`** · prefix **`T-090.1.2.5.1:`**

Handoff: [`.ai/artifacts/t090_1_2_5_1_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_5_1_claude_code_handoff.md)

---

## Claude Code prompt — T-090.1.2.5.1 (copy-paste)

Extract: `./scripts/ticket prompt T-090 --slice T-090.1.2.5.1`

```
Read CLAUDE.md first.

Implement **T-090.1.2.5.1** — inland water mask refine (road exclusion + hill rivers).

═══ PREFLIGHT ═══
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090
  test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.pre-water.png
  Read .ai/artifacts/t090_1_2_5_water_source_spike.json + t090_1_2_5_verify_log.md (post-ship gaps)

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_1_2_5_1_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_2_5_1_water_mask_refine.md
  3. scripts/map-assets/analyze-water-sources.mjs
  4. scripts/map-assets/composite-water-ortho.mjs

═══ PROBLEM ═══
  T-090.1.2.5 inland mask E marks grey SAP “water appearance” pixels. Roads/pavement false-
  positive (wide connected grey); hill streams false-negative (slope/opening/min-area).
  Refine inland filters only — ocean mask A (DEM<=0) unchanged.

═══ SHIPPED (do not reopen) ═══
  - T-090.1.2.5 @ 6396960f — first water composite (mask E + ocean A)
  - T-090.1.2.8 @ db9057ef — unified delivery path

═══ LOCKED ═══
  - Restore pre-water ortho + strip waterComposite meta before re-composite
  - P0 refine spike JSON with locked params before full regen
  - Forbidden: hand-paint, AI rivers, solid blue rectangle, ocean algorithm change
  - Operator regression coords: road FP ~(4776,9268); audit central lake (4618,5972)
  - No docs/registry edits

═══ DO ═══
  1. P0 — refine spike → .ai/artifacts/t090_1_2_5_1_refine_spike.json
     R1 elongation/perimeter/asphalt band for road FP rejection
     R2 linear-body slope relax + valley-carve or min-area for hill river FN
     R3 optional .topo/.smap probe (45 min timebox)
  2. Restore everon-sap-ortho.pre-water.png; clear meta.waterComposite
  3. Update analyze-water-sources.mjs with locked filters; regen water-inland-mask.png
  4. composite-water-ortho.mjs → ortho + meta
  5. build-unified-satellite.mjs + lossless pyramid rebuild
  6. .ai/artifacts/t090_1_2_5_1_verify_log.md — R-FP1/2, R-FN1, R-REG1–3 + crops
  7. Tag **T-090.1.2.5.1** · prefix **T-090.1.2.5.1:**

═══ DO NOT ═══
  - Edit docs/**, `.ai/tickets/registry.json`, CLAUDE status markers
  - Change ocean mask (DEM<=0) logic
  - Skip P0 spike or double-composite without restore

═══ VERIFY (all exit 0) ═══
  node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
  node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
  EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
  make verify-terrain
  cd apps/website/frontend && npm run build && npm run lint

═══ MANUAL ═══
  R-FP1/2: flagged roads grey not blue
  R-FN1: ≥2 hill streams newly water
  R-REG1–3: lake, ocean, land byte-identity

═══ RETURN ═══
  - Commit SHA + tag T-090.1.2.5.1
  - Refine spike JSON + param table
  - Verify output + regression crops
  - **Ready for Cursor doc sync.**
```
