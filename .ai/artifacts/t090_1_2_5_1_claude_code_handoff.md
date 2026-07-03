# T-090.1.2.5.1 — Claude Code handoff (inland water mask refine)

**Slice:** T-090.1.2.5.1 · **Executor:** claude-code · **Branch:** `main`  
**Parent:** T-090.1.2.5 @ `6396960f`  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_5_1_water_mask_refine.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_5_1_water_mask_refine.md)

---

## Operator report

- **Road FP:** paved corridors and rectangular pads tinted blue (screenshots ~world 4776, 9268 and similar).
- **Hill river FN:** mountain stream beds visible as dry grey/brown lines — not composited.
- **Keep:** ocean, central lake (4618, 5972), main lowland river network.

Prior spike: [`.ai/artifacts/t090_1_2_5_water_source_spike.json`](t090_1_2_5_water_source_spike.json)  
Post-ship notes: [`.ai/artifacts/t090_1_2_5_verify_log.md`](t090_1_2_5_verify_log.md)

---

## Pipeline

```text
restore pre-water ortho + strip meta.waterComposite
  → P0 refine spike (locked params)
  → analyze-water-sources.mjs (inland filters only)
  → composite-water-ortho.mjs
  → build-unified-satellite.mjs + lossless pyramid
  → verify gates + regression crops
```

---

## Do not

| Forbidden | Why |
|-----------|-----|
| Hand-paint / AI rivers | Spec |
| Change ocean DEM<=0 mask | Out of scope |
| Double-composite without restore | composite-water-ortho.mjs exits |
| Edit docs/registry | Cursor doc sync |

**Do not reopen:** T-090.1.2.5 ship logic for ocean; T-090.1.2.8 unified path.

---

## Restore before re-composite

```bash
SAP=packages/map-assets/everon/staging/sap
cp "$SAP/everon-sap-ortho.pre-water.png" "$SAP/everon-sap-ortho.png"
# Remove waterComposite key from TBD_SatExport_meta.json (jq or node one-liner)
```

---

## Filter ideas (implement after P0 spike locks params)

| Issue | Levers in `analyze-water-sources.mjs` |
|-------|--------------------------------------|
| Road FP | Component bbox aspect ratio; perimeter/√area; stricter lum/sat for asphalt; reject elongated bodies unless lake-like compactness |
| Hill river FN | Higher `slopeMeanMaxDeg` for high-aspect components; DEM valley local-minima mask; lower `minAreaM2` for linear bodies only |

Current constants (baseline @ `.2.5`): `OPEN_R=2`, `MIN_AREA_M2=2000`, `SLOPE_MEAN_MAX_DEG=8`, `FLAT_FRAC_MAX=0.5`, grey detect `satMax=0.12`, `lum 0.2–0.44`.

---

## Key files

| File | Role |
|------|------|
| `scripts/map-assets/analyze-water-sources.mjs` | Inland mask builder — primary edit |
| `scripts/map-assets/composite-water-ortho.mjs` | Blend (unchanged unless meta fields) |
| `scripts/map-assets/build-unified-satellite.mjs` | Bundle rebuild |
| `staging/sap/water-inland-mask.png` | Regenerated mask |
| `staging/sap/everon-sap-ortho.pre-water.png` | Restore source |

---

## Verify

```bash
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make verify-terrain
cd apps/website/frontend && npm run build && npm run lint
```

Log: `.ai/artifacts/t090_1_2_5_1_verify_log.md` — include magick crops for R-FP1, R-FN1, R-REG1.

Tag **`T-090.1.2.5.1`**. Return **"Ready for Cursor doc sync."**
