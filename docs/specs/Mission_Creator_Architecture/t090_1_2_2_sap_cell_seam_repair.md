# T-090.1.2.2 — SAP supertexture cell seam repair

**Ticket:** T-090 · **Slice:** T-090.1.2.2  
**Status:** **READY** — operator-visible grid seams at 256 m SAP cell boundaries  
**Executor:** claude-code  
**Depends on:** **T-090.1.2.1** shipped @ `19bc785` (lossless z0–6 pyramid)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t090_1_2_sap_supertexture_satellite.md`](t090_1_2_sap_supertexture_satellite.md)

---

## In one sentence

Eliminate visible **vertical/horizontal gaps** where the 50×50 SAP supertexture cells meet in the stitched ortho — then rebuild the lossless pyramid — without re-inventing the decode pipeline or faking detail.

---

## Problem (operator-reported @ T-090.1.2.1)

At max zoom, Satellite shows **clear stitch lines** on a ~**256 m** grid (matches SAP cell size). Example: vertical seam + cross distortion where four cells meet (operator screenshot). This is **not** WebP pyramid tile seams (those are 200 m @ z6) — it is the **ortho composite** from `stitch-sap-ortho.mjs`:

- 2500 cells pasted **hard edge-to-edge** with no overlap blend
- Adjacent BC7-decoded cells can differ in exposure/tint at borders
- Any sub-pixel placement error amplifies as a visible line

Lossless VP8L (T-090.1.2.1) preserved the seam faithfully — compression is not the cause.

---

## What this slice is NOT

| Out of scope | Why |
|--------------|-----|
| **z7+ pyramid** | Interpolates 1 m/px source — fake detail |
| **AI upscaling** | Operator rejected |
| **BC7 “decompression”** | BC7 decode is block-lossy (4×4); cannot recover discarded high frequencies from `.edds` |
| **Pan flicker** | **T-090.1.2.3** (frontend tile cache) |
| **Brightness / tone** | Separate later pass |

---

## Goal

1. **Measure** seam severity on the 256 px grid (automated gate).
2. **Fix** stitch compositing so cell boundaries are **invisible at max zoom** on representative terrain (fields, roads, forest).
3. **Rebuild** lossless z0–6 pyramid from repaired ortho (same command as T-090.1.2.1).
4. **Verify** orientation + alignment gates still PASS (no regression on T-090.1.2 north-up fix).

---

## Investigation (P0 — gate before blind blending)

Script: `scripts/map-assets/analyze-sap-seams.mjs` (new)

| Check | Purpose |
|-------|---------|
| Sample **edge strips** (e.g. 8 px) between adjacent decoded cells | Compare mean ΔRGB / max Δ — quantify mismatch |
| Compare **same-world** edge vs **random** interior edge | Confirm grid-aligned anomaly |
| Optional: inspect higher mips in `.edds` for built-in overlap | If BI stores overlap, use it instead of inventing blend |

Output: `.ai/artifacts/t090_1_2_2_seam_analysis.json`

**STOP** if analysis shows seams are placement bugs (off-by-one row) rather than exposure — fix placement first.

---

## Fix strategies (pick smallest that passes S1)

Apply in `stitch-sap-ortho.mjs` (or companion `blend-sap-seams.mjs` post-pass):

| Strategy | When |
|----------|------|
| **A. Edge feather** | Default — linear blend 2–8 px on interior cell edges (skip map outer border) |
| **B. Per-cell gain/offset** | If edges differ only in brightness — match histogram on 16 px overlap band |
| **C. Overlap discard** | If cells include redundant border texels — crop 1–2 px before paste |
| **D. Placement fix** | If analysis shows row/col mirror bug at specific grid lines |

Forbidden: grey fill, blur-the-whole-map, AI inpainting.

---

## Rebuild + ship

```bash
node scripts/map-assets/stitch-sap-ortho.mjs TERRAIN=everon
node scripts/map-assets/analyze-sap-seams.mjs TERRAIN=everon   # post-fix — must PASS
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon

scripts/map-assets/build-tile-pyramid.sh \
  --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless

EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make verify-terrain && make ci-local-frontend
```

New gate: `verify-sap-seams.mjs` — max edge discontinuity on 256 px grid below threshold (define from P0 baseline).

---

## Manual acceptance

| ID | Pass |
|----|------|
| **S1** | Operator screenshot location — **no visible line** at former seam @ max zoom |
| **S2** | Spot-check 3 other cell intersections (road, forest, coast) |
| **S3** | North-up + H1/H2 alignment unchanged |
| **S4** | Pan/zoom ≥55 fps (pyramid unchanged structurally) |

Log: `.ai/artifacts/t090_1_2_2_verify_log.md`

---

## Files (expected)

| Action | Path |
|--------|------|
| Create | `scripts/map-assets/analyze-sap-seams.mjs` |
| Create | `scripts/map-assets/verify-sap-seams.mjs` |
| Edit | `scripts/map-assets/stitch-sap-ortho.mjs` (or blend post-pass) |
| Replace | `packages/map-assets/everon/tiles/satellite/**` (LFS rebuild) |
| Artifacts | `.ai/artifacts/t090_1_2_2_seam_analysis.json`, verify log |

**Do not touch** unless S4 fails: `useTerrainBasemapLayer.ts`, decode contract in `decode-edds.mjs`.

---

## Ship

Tag **`T-090.1.2.2`** · prefix **`T-090.1.2.2:`** · `active_slice` → **T-090.1.2.3** or **T-090.1.1** (Cursor sync).

---

## Related

Handoff: [`.ai/artifacts/t090_1_2_2_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_2_claude_code_handoff.md) · send-off [`.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md)

Resume: [`t090_1_2_satellite_backlog.md`](t090_1_2_satellite_backlog.md)
- Pan flicker: **T-090.1.2.3**
