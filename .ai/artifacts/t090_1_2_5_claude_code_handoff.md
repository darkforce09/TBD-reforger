# T-090.1.2.5 — Claude Code handoff (satellite water composite)

**Slice:** T-090.1.2.5 · **Executor:** claude-code  
**Depends on:** T-090.1.2.2 shipped (seam-fixed ortho preferred) · T-090.1.2.1 @ `19bc785`  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_5_satellite_water_composite.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_5_satellite_water_composite.md)  
**Resume guide:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md)

## Problem

**No readable water** on Satellite basemap today:
- **Ocean:** grey SAP seabed (not blue water)
- **Inland:** lakes/rivers look like dry ground

Interim T-090.1 raster had **blue ocean only** — inland was still missing. Operator needs both.

## P0 — Water source spike (gate)

Create `analyze-water-sources.mjs` or spike doc → `.ai/artifacts/t090_1_2_5_water_source_spike.json`

Evaluate (pick one primary + optional refine):
- MapDataExporter / engine hydrology mask
- DEM height ≤ sea level (+ river width heuristic)
- Future waterBody export (T-090.8) — note if blocked

**Forbidden:** hand-painted lakes, AI rivers.

## P1 — Composite ortho

Script `composite-water-ortho.mjs`:
- Input: seam-fixed `everon-sap-ortho.png` + aligned water mask (north-up)
- Output: composited PNG → staging
- Ocean + inland may use different treatment; land SAP preserved outside mask

## P2 — Rebuild unified bundle (T-090.1.2.8 primary)

After composited ortho lands in staging:

```bash
node scripts/map-assets/build-unified-satellite.mjs \
  --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out packages/map-assets/everon/satellite/everon-sat.tbd-sat \
  --terrain everon
node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
```

Optional pyramid fallback (legacy delivery):

```bash
scripts/map-assets/build-tile-pyramid.sh \
  --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
```

## P3 — Verify + ship gates

```bash
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make verify-terrain && make ci-local-frontend
```

Update `map_export_everon.json` with water mask source.

Manual **W1–W4** in spec (coast blue, inland bodies, alignment).

Tag **`T-090.1.2.5`**. Return **"Ready for Cursor doc sync."**

## Do not

- Edit docs/registry
- Skip spike — must document mask provenance
