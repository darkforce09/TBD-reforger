# T-090.1.2 — SAP supertexture satellite ortho — verify log

**Slice:** T-090.1.2 · **Date:** 2026-06-30 · **Executor:** claude-code
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_sap_supertexture_satellite.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_sap_supertexture_satellite.md)

Replaces the interim T-090.1 rasterization satellite tiles (stylized 4096²/3.125 m/px)
with a true SAP super-texture ortho decoded from `worlds/Eden/Eden/.Data/Eden_<N>_supertexture.edds`.

## Decode contract (proven)

Enfusion EDDS (no `DDS ` magic): `dxgiFormat` u32LE @ `0x48` = **99 BC7_UNORM_SRGB**;
chunk table @ `0x5c` = records `[4B tag][u32LE len]`, tag ∈ {`COPY`,`LZ4 `}; **9 mips**,
mip0 = 256×256 (largest record); `COPY` = raw BC7, `LZ4 ` = `[u32 decompSize][u32 _][raw
LZ4 block]` (stream @ +8). Pure-JS LZ4 verified byte-identical to liblz4. BC7→RGBA via
vendored `bcdec.wasm` (iOrange bcdec v0.97, clang wasm32).

Grid: **row-major N = y·50 + x**; cell **gridY=0 = world Z=0 = south**. Ortho assembled
**north-up** (south at the image bottom — whole-canvas vertical mirror in the stitch copy loop).
50×50 cells × 256 m / 256 px = **1 m/px**, full ortho **12800×12800** (3.1× the interim 4096²).

## Orientation fix (post-render review)

Operator review caught the basemap rendering **upside-down (N/S swapped)** vs real Everon. Root
cause: the first stitch put cell gridY=0 (world Z=0 = south) at the image **top**, but the pipeline
renders the image top at the editor's maxZ/north → south-at-top. The DEM/slots/coords/render were all
correct (`useDemLayer` already flips the DEM raster, whose row 0 = world y=0 = south). **Fix:** stitch
assembles north-up (mirror). Verified: flipped ortho matches the BI Everon map (airfield peninsula
north/top, mountain mass SE/bottom-right) and the new `verify-sap-ortho` orientation guard passes
(AE 0.078 vs 0.350 when flipped).

## P0 decode spike — PASS

`.ai/artifacts/t090_1_2_decode_spike.json`. Cell 0 mip0 → 256² RGBA, sha256
`68fa3c25…a5e2ac`, reproducible (`node scripts/map-assets/decode-edds.mjs 0 | sha256sum`).
Cross-check: RGB **AE = PAE = MAE = 0** vs an independent liblz4 + Pillow oracle.
Full 50×50 mosaic = recognizable Everon (orientation corrected post-review — see Orientation fix below).

## Build

| Step | Result |
|------|--------|
| `vendor/build-bcdec-wasm.sh` | `bcdec.wasm` 10,438 B (clang 22 wasm32 + rust-lld) |
| BC7 unit test (`node --test vendor/`) | **2/2 pass** (varied block vs Pillow-independent expected) |
| `catalog-sap-cells.mjs` | 2500 cells → `staging/sap/cell-catalog.json` |
| `stitch-sap-ortho.mjs` | **12800×12800**, **2500/2500 cells**, **23 s** (decode+stitch); fail-fast, no grey fill |
| `build-tile-pyramid.sh --maxzoom 5` | z0–5 WebP q=80, no `--flip-v` — **1365 tiles** committed (z6 + lossless = follow-up) |

Ortho PNG (staging, gitignored): `everon-sap-ortho.png`, 12800², sRGB.

## Automated gates

| Gate | Result |
|------|--------|
| `test -f .ai/artifacts/t090_1_2_decode_spike.json` | PASS |
| BC7 unit test (`node --test scripts/map-assets/vendor/`) | **2/2 PASS** |
| `node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon` | **PASS** (catalog 2500, meta source/cells/dims/mpp/bounds, ortho 12800² stddev 0.0535, **orientation guard AE 0.078 < 0.2**, z0 tile) |
| `node scripts/map-assets/verify-spike-ops-log.mjs TERRAIN=everon` | **PASS** (K7 + K2/K3/K4) |
| `make verify-terrain` | **PASS** (manifest schema + DEM alignment maxDelta 0.204 m; metersPerPixel 1 does not regress DEM) |
| `node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon` | **PASS** (z0–5, 1365 tiles; manifest `maxZoom` 5 — z6 not built) |
| `make ci-local-frontend` (Node 26) | **PASS** — build OK, **26/26** vitest, lint + format:check clean |

## Manual acceptance (operator to confirm in browser)

| ID | In-session finding |
|----|--------------------|
| D1 | **PASS (visual)** — 100 % crop shows ploughed field, forest canopy, dirt roads, field boundaries: real meter-scale ground, not smooth ramps. |
| D2 | **Fixed; awaiting operator browser confirm.** Corrected north-up ortho matches the BI Everon map (airfield peninsula north/top, mountain mass SE/bottom-right); orientation guard passes. Operator to confirm in-browser (do not tag until then). |
| D3 | alignment preserved — same world bounds `[0,0,12800,12800]`, 12800² ortho; `verify-terrain-alignment` PASS. Operator to re-confirm H1/H2/H2b in browser. |
| D4 | pyramid LOD unchanged; manifest capped at z5 until z6 + lossless rebuild; operator to confirm ≥55 fps. |

## Follow-ups (110% bar — not in this ship)

| Issue | Root cause | Fix |
|-------|------------|-----|
| Basemap vanishes at max zoom | Manifest `maxZoom: 6` while z6 directory empty → LOD requests missing tiles | Ship z0–5 only (`maxZoom: 5`); rebuild z6 when ready |
| Tiles look compressed | `build-tile-pyramid.sh` uses `cwebp -q 80` (lossy) | Rebuild with `cwebp -lossless` or PNG tiles; expect ~2–4× LFS size |

## Notes

- **Ocean = authentic SAP seabed texture** (grey, with ripple detail), not a blue overlay.
  The interim rasterization tinted water blue; the raw SAP is the real ground/seabed — no
  synthetic tint added (would be fake detail). Flag for operator: editor will show seabed,
  not blue water.
- **manifest** `tiles.maxZoom` **5** (z0–5 pyramid shipped; z6 deferred — manifest had briefly advertised 6 while z6 build incomplete, which caused max-zoom basemap 404/disappear), `tiles.satellite.source → sap-supertexture-stitch`,
  top-level `metersPerPixel 2→1` (DEM keeps `dem.widthPx 6400` + `precision.demNativeMetersPerPixel 2`;
  no runtime reads top-level `metersPerPixel`). Schema enum gained `sap-supertexture-stitch`.
- **Incidental:** prettier-formatted `useTerrainBasemapLayer.ts` (pre-existing line-wrap drift
  from T-090.1, unchanged vs HEAD otherwise) so `format:check` passes — zero behavior change.
- LFS: regenerated `tiles/satellite/**` WebP. Vendored `bcdec.{h,wasm}` are normal git blobs.
