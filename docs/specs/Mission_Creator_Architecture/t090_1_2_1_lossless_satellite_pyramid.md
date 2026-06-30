# T-090.1.2.1 — Lossless satellite pyramid (picture-perfect zoom)

**Ticket:** T-090 · **Slice:** T-090.1.2.1  
**Status:** **READY** — blocks crisp Satellite acceptance; run **before** T-090.1.1 Map tiles  
**Executor:** claude-code  
**Depends on:** **T-090.1.2** shipped @ `c2730a3` (12800² SAP ortho + decode/stitch pipeline)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t090_1_2_sap_supertexture_satellite.md`](t090_1_2_sap_supertexture_satellite.md)

---

## In one sentence

Rebuild the Everon **Satellite** tile pyramid from the staged SAP ortho using **lossless WebP** (or PNG) through **z6**, bump manifest `maxZoom` to match, and verify max deck zoom shows **native 1 m/px ground texture with zero visible compression** — not the current blurry q=80 z5 stretch.

---

## Problem (operator-reported)

After T-090.1.2, max zoom **stays visible** (manifest capped at z5) but the basemap is **unacceptably blurry**:

| Cause | Effect |
|-------|--------|
| **`cwebp -q 80`** on every tile | Lossy block artifacts on fields, roads, forest floor — magnified when zoomed in |
| **`maxZoom: 5`** while deck **`MAX_ZOOM = 6`** | At full zoom, LOD picks **z5** (~**1.56 m/px** tile texels) and **stretches** them on screen; native ortho is **1 m/px** → permanent softening even with lossless encode |
| Incomplete z6 commit (partial rows) | Must not ship sparse z6; full **64×64 = 4096** tile level or manifest stays at z5 |

**110% bar:** zoomed-in Satellite must match the clarity of the BI in-game supertexture / the stitched ortho preview — **no visible blur that looks like compression or upscaling**.

---

## Goal

1. **Lossless encode** for all pyramid tiles (no `-q 80`).
2. **Complete z0–6 pyramid** from `packages/map-assets/everon/staging/sap/everon-sap-ortho.png` (12800×12800, north-up — do **not** re-stitch unless verify fails).
3. **Manifest** `tiles.maxZoom: 6`, `metersPerPixel: 1` unchanged; ops log updated.
4. **Automated gates** pass including lossless WebP assertion + full z6 tile count.
5. **Manual:** operator max-zoom pan on a field edge / road junction — **pixel-sharp**, not watercolor.

---

## Resolution math (why z6 is required)

Everon world width **12800 m**; ortho **12800 px** → **1 m/px** source.

Pyramid level **z** (256 px tiles): meters per tile texel = `12800 / (256 × 2^z)`.

| z | m/px in tile | At deck zoom 6 |
|---|--------------|----------------|
| 5 | **1.56** | Selected today → **upsampled → blur** |
| 6 | **0.78** | Matches native band (~1 m/px) → **sharp** |

Frontend LOD (`useTerrainBasemapLayer.ts`):

```ts
z = clampInt(Math.ceil(Math.log2(w / 256) + viewState.zoom), minZoom, maxZoom)
// w=12800 → log2(50)≈5.64; at view zoom 6 → z=12 capped to maxZoom
```

**Ship rule:** `manifest.tiles.maxZoom === 6` **iff** disk has complete z6; never advertise z6 with partial tiles.

---

## Implementation plan

### P1 — Pyramid script (`build-tile-pyramid.sh`)

Add explicit lossless mode (keep lossy path for interim/dev):

| Flag | Behavior |
|------|----------|
| `--lossless` | `cwebp -lossless` instead of `-q N` |
| `--quality N` | Lossy only when `--lossless` omitted (default 80, unchanged for T-090.1) |

**SAP rebuild command (Everon):**

```bash
scripts/map-assets/build-tile-pyramid.sh \
  --input  packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out    packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
```

- **No `--flip-v`** (ortho already north-up per T-090.1.2 orientation fix).
- Script **clears `--out`** before write — removes partial z6 junk from aborted builds.
- `full.webp`: either **lossless** at capped edge (4096) or **omit** if pyramid-only mode is cleaner (frontend probes z0 first; full is fallback only).

**LFS budget (estimate):** z0–6 lossless WebP ≈ **250–450 MB** (vs ~100–150 MB q=80). Accept for 110% bar; document actual `du -sh` in verify log.

### P2 — Verify (`verify-tile-pyramid.mjs`)

Extend gates:

1. **Complete pyramid:** for every `z` in `[minZoom, maxZoom]`, exactly `(2^z)²` tiles exist (no sparse levels).
2. **Lossless WebP:** tiles use **VP8L** chunk (parser already handles VP8L dims); **fail** if VP8 lossy chunk present when `manifest.tiles.satellite.encoding === "webp-lossless"` (new optional manifest field) or when env `EXPECT_LOSSLESS=1`.
3. **maxZoom vs disk:** manifest `maxZoom` must equal highest complete level on disk.

Optional manifest addition (schema + Everon manifest):

```json
"satellite": {
  "encoding": "webp-lossless",
  "source": "sap-supertexture-stitch"
}
```

### P3 — Manifest + ops log

| Field | Value |
|-------|-------|
| `tiles.maxZoom` | **6** |
| `tiles.satellite.encoding` | **`webp-lossless`** (new) |
| `map_export_everon.json` | method notes lossless z0–6, tile count, LFS size |

### P4 — Frontend (only if still soft after P1–P3)

**Default:** no TS change — correct z6 + lossless tiles should suffice.

If operator still sees mush:

- Confirm `computeLod` at max zoom returns **z=6** with **≤64 visible tiles** (viewport cull).
- Consider `BitmapLayer` `textureParameters: { minFilter: 'linear', magFilter: 'linear' }` only after lossless z6 proven — **not** `nearest` (creates blockiness). Do **not** change Y-flip or bounds.

### P5 — Manual acceptance (operator)

| ID | Pass |
|----|------|
| **L1** | Max deck zoom on ploughed field / road — individual texture patches readable; **no watercolor blur** |
| **L2** | Side-by-side mental check vs T-090.1.2 stitched ortho crop @ same world coords — no visible quality regression |
| **L3** | North-up + H1/H2 alignment unchanged |
| **L4** | Pan/zoom ≥55 fps with pyramid LOD (same cap `MAX_VISIBLE_BASEMAP_TILES=64`) |

Log: `.ai/artifacts/t090_1_2_1_verify_log.md`

---

## Verification (automated — all PASS to ship)

```bash
# Ortho still valid (do not skip)
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon

# Full pyramid + lossless
EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon

make verify-terrain
make ci-local-frontend
node scripts/map-assets/verify-spike-ops-log.mjs TERRAIN=everon
```

**Ship:** tag **`T-090.1.2.1`** · commit prefix **`T-090.1.2.1:`** · `active_slice` → **T-090.1.1** (Cursor sync).

---

## Files (expected)

| Action | Path |
|--------|------|
| Edit | `scripts/map-assets/build-tile-pyramid.sh` (`--lossless`) |
| Edit | `scripts/map-assets/verify-tile-pyramid.mjs` (complete levels + VP8L gate) |
| Maybe edit | `packages/tbd-schema/schema/terrain-manifest.schema.json` (`satellite.encoding`) |
| Replace | `packages/map-assets/everon/tiles/satellite/**` (LFS) |
| Edit | `packages/map-assets/everon/manifest.json` (`maxZoom: 6`, encoding) |
| Edit | `.ai/artifacts/map_export_everon.json` |
| Create | `.ai/artifacts/t090_1_2_1_verify_log.md` |

**Do not touch** unless L4 fails: `useTerrainBasemapLayer.ts`, `tileUrl.ts`, stitch/decode scripts.

---

## Out of scope

- Map cartographic tiles (**T-090.1.1**)
- Re-decode SAP cells (ortho already in staging)
- PNG tiles (acceptable alternative if WebP lossless still fails L1 — document switch)
- Arland / other terrains

---

## Related

- Parent: T-090.1.2 @ `c2730a3` — SAP ortho source
- Handoff: [`.ai/artifacts/t090_1_2_1_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_1_claude_code_handoff.md)
