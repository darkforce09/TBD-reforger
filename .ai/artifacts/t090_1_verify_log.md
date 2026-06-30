# T-090.1 — Satellite basemap verify log

**Slice:** T-090.1 · **Executor:** claude-code · **Date:** 2026-06-30 · **Status:** ✅ COMPLETE — K3 closed, all automated gates green.

---

## Phase 0 — satellite raster (method + result)

No one-call Workbench export exists (spike S3). The clean, scriptable path = the engine's own map
rasterizer, **`MapDataExporter.ExportRasterization`** (the same call the WE *Export Map Data →
Rasterization* tool / `SCR_WorldMapExportTool` drives). **Capture method id = 2** (plugin → engine API).

- Plugin: `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_SatelliteExportPlugin.c` (menu `Plugins,TBD,Export TBD Satellite`).
- worldPath = `worlds/Eden/Eden.ent` (Everon base world; clean terrain + towns/forests).
- **Path quirk solved:** `ExportRasterization` does NOT resolve the `$profile:` VFS (FileIO does → rc=32 "Could not open output file"). It needs a real OS path; under Proton that's a Windows path in the prefix. Passing the profile **directory** `C:/Users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/` → **rc=0**; the engine names the output after the world → **`Eden.tga`**.
- Output: **4096×4096 RGBA TGA**, 67,108,882 B (= 4096²·4 + 18-byte header). Resolution is engine-fixed (3.125 m/px for Everon), **not** the 6400² the plan assumed — corrected; the pyramid build resizes per level. **B2:** `MapDataExporter` exposes no output-size param → 4096² is engine-locked; true meter-detail needs the SAP super-textures (`worlds/Eden/Eden/.Data/Eden_*_supertexture.edds`, `.edds` decode — separate effort).
- **Orientation (T-090.1.1 fix):** the **raw** export is north-up for Deck (magick respects the TGA origin bit; Deck maps the top scanline → north), so tile with **no `--flip-v`**. The initial ship used `--flip-v` and rendered north-at-bottom — my validation baseline was the DEM **PNG**, which is itself stored **south-up** (`useDemLayer` row-flips it at runtime). Re-confirmed `asis` (raw) = north-up against the operator's real-Everon reference + an orientation contact sheet: SE mountain peninsula bottom-right; E-W correct (DEM `axisFlip.x=false` + anchor verify).
- EnfScript notes (fixed during bring-up): **no ternary `? :` operator** (use if/else); engine names export by world, not by given filename.

### Phase 0 exit table

| Field | Value |
|-------|-------|
| File | `packages/map-assets/everon/staging/spike/TBD_SatExport_everon.tga` (from `$profile:/Eden.tga`) |
| File size (bytes) | 67,108,882 |
| Dimensions (px) | **4096 × 4096** (engine-fixed; 3.125 m/px) |
| SW origin / north-up | ✓ **raw, no flip** (Deck top scanline → north) |
| H2b landmark @ ~(4839,6620) | **north not mirror-flipped** ✓ (raw `asis` matches the real-Everon reference; SE peninsula bottom-right) |
| Capture method id | 2 (plugin → `MapDataExporter.ExportRasterization`) |

---

## Ship mode (T-090.1.1)

- **`basemapRenderMode: pyramid`** — the frontend prefers the z0–5 pyramid with **viewport LOD**: level
  chosen from the deck zoom (`z = clamp(ceil(log2(width/256) + zoom), minZoom, maxZoom)`), culled to the
  visible world AABB, capped at **`MAX_VISIBLE_BASEMAP_TILES = 64`** (drop a level until it fits). Zooming
  in loads deeper/sharper tiles instead of stretching one image (the blur fix). `full.webp` is the
  single-bitmap **fallback** + H-test surface only.
- Detail ceiling is the 4096² source; z5 tiles are 8192-equiv (≈2× upsampled) → sharper than the
  single-bitmap stretch, but true meter-detail awaits a higher-res SAP source (B2).

## Alignment (H1/H2/H2b)

Judged on the full-res ortho (not the 256² stub), via the north-up DEM ground truth:
- **H1** — ortho covers exactly world `[0,0,12800,12800]`; BitmapLayer `bounds` = same → world (0,0)=SW corner by construction.
- **H2** — landmass/coastline matches the DEM land mask 1:1 at full extent; landmark region @ ~(4839,6620) lands on the same coast as the DEM (≤50 m @ default editor zoom, single-bitmap criterion).
- **H2b** — **north not mirror-flipped** ✓ (flipped ortho == DEM; as-is is the mirror).
- **H3** — BitmapLayer is clipped to world bounds (no draw past 12800 m).

Browser M1–M5 (visual confirmation in `/missions/:id/edit`) remain a manual pass; alignment is already proven against the DEM here.

---

## Gates — GREEN

| Gate | Result |
|------|--------|
| `test -f …/tiles/satellite/0/0/0.webp` (K3 file gate) | **PASS** |
| `verify-tile-pyramid.mjs TERRAIN=everon` | **PASS** — levels [0–5], 1365 tiles, 256px |
| `verify-spike-ops-log.mjs TERRAIN=everon` | **PASS** — K3 gate↔artifact |
| `make verify-terrain` | **PASS** (maxDeltaM 0.204 ≤ 1.0) |
| frontend lint / build / vitest | **PASS** (26/26, incl. `tileUrl` Y-flip ×5) / format clean |
| LFS | `*.webp` tracked (`git check-attr` = lfs); tiles **5.5 MB** total (shaded map compresses well) |
| manifest `tiles.satellite`/`map` | schema-valid (dual-tiles golden shape) |

**Note:** `make schema-validate` remains red **only** on the pre-existing `verify-t090-specs` gate 8/10 —
docs say "T-090.1 active" but the gate hardcodes "active = T-090.3.0". None of those files were touched
here; it's the Cursor doc/registry-sync boundary (T-090.3.0 → T-090.1 advance), not code.

## Render contract (recap)

Cartesian only (no Web Mercator, no `@deck.gl/geo-layers`) · single Y-inversion in `layers/tileUrl.ts`
(`tmsY = 2**z-1-y`, unit-tested) · render-mode auto-select (full.webp → single-bitmap; else pyramid
≤64-tile grid; else grid + toast) · layer order satellite → hillshade → grid → icons · basemap-view pref
`localStorage tbd-mc-basemap-view` (Satellite; Map disabled until T-090.1.1).
