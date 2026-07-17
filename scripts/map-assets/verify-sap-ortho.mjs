// T-090.1.2 — verify the stitched SAP satellite ortho (post-build ship gate).
//
// Asserts the build artifacts produced by `world sap-catalog` + stitch-sap-ortho.mjs
// (under packages/map-assets/everon/staging/sap/, gitignored) plus the committed
// satellite z0 tile. HARD requirements:
//   - cell catalog has exactly 2500 cells
//   - stitch meta: source=sap-supertexture-stitch, cellsDecoded===2500, 12800x12800,
//     metersPerPixel===1, worldBounds [0,0,12800,12800]
//   - ortho PNG is 12800x12800 and NOT flat (global stddev above a floor) — i.e. real
//     decoded ground, not a placeholder/grey fill
//   - ORIENTATION: ortho land-mask matches the DEM land-mask rendered north-up (catches a
//     vertical N/S flip — the upside-down bug from the post-render review). Non-circular:
//     the DEM is independently anchored to real GetTerrainSurfaceY.
//   - committed satellite/0/0/0.webp exists (>0 bytes)
//
// Uses `magick` (already a hard pipeline prerequisite of build-tile-pyramid.sh). Run AFTER stitch.
//
//   node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "../..");

const terrain =
  (process.argv.find((a) => a.startsWith("TERRAIN=")) || "TERRAIN=everon").split("=")[1];
if (terrain !== "everon") {
  console.error(`only everon supported this slice (got ${terrain})`);
  process.exit(1);
}

const SAP = join(REPO, "packages/map-assets/everon/staging/sap");
const catalogPath = join(SAP, "cell-catalog.json");
const metaPath = join(SAP, "TBD_SatExport_meta.json");
const orthoPath = join(SAP, "everon-sap-ortho.png");
const manifestPath = join(REPO, "packages/map-assets/everon/manifest.json");
const z000 = join(REPO, "packages/map-assets/everon/tiles/satellite/0/0/0.webp");

const EXPECT_CELLS = 2500;
const EXPECT_DIM = 12800;
const MIN_STDDEV = 0.02; // normalized [0,1]; a flat/grey placeholder is ~0, real ground >> this
const ORIENT_MAX = 0.2; // max land-mask AE ratio vs north-up DEM (flip ⇒ ~0.35 fail, match ⇒ ~0.08 pass)

const errors = [];
const ok = (m) => console.log(`  ok: ${m}`);

function eqArr(a, b) {
  return Array.isArray(a) && a.length === b.length && a.every((v, i) => v === b[i]);
}

// 1) catalog
if (!existsSync(catalogPath)) {
  errors.push(`missing ${catalogPath} — run `cargo run -q -p tbd-tools --bin world -- sap-catalog` first`);
} else {
  const cat = JSON.parse(readFileSync(catalogPath, "utf8"));
  if (cat.cellCount !== EXPECT_CELLS || cat.cells?.length !== EXPECT_CELLS) {
    errors.push(`catalog cellCount ${cat.cellCount}/${cat.cells?.length} != ${EXPECT_CELLS}`);
  } else ok(`catalog ${EXPECT_CELLS} cells`);
}

// 2) stitch meta
if (!existsSync(metaPath)) {
  errors.push(`missing ${metaPath} — run stitch-sap-ortho.mjs first`);
} else {
  const m = JSON.parse(readFileSync(metaPath, "utf8"));
  if (m.source !== "sap-supertexture-stitch") errors.push(`meta.source=${m.source}`);
  if (m.cellsDecoded !== EXPECT_CELLS) errors.push(`meta.cellsDecoded ${m.cellsDecoded} != ${EXPECT_CELLS}`);
  if (!eqArr(m.dimensions, [EXPECT_DIM, EXPECT_DIM])) errors.push(`meta.dimensions ${m.dimensions}`);
  if (m.metersPerPixel !== 1) errors.push(`meta.metersPerPixel ${m.metersPerPixel} != 1`);
  if (!eqArr(m.worldBounds, [0, 0, EXPECT_DIM, EXPECT_DIM])) errors.push(`meta.worldBounds ${m.worldBounds}`);
  if (errors.length === 0) ok(`meta source/cells/dims/mpp/bounds`);
}

// 3) ortho dims + variance (magick identify)
if (!existsSync(orthoPath)) {
  errors.push(`missing ${orthoPath} — run stitch-sap-ortho.mjs first`);
} else {
  let out;
  try {
    out = execFileSync(
      "magick",
      ["identify", "-format", "%w %h %[fx:standard_deviation]", orthoPath],
      { encoding: "utf8" }
    ).trim();
  } catch (e) {
    errors.push(`magick identify failed: ${e.message}`);
  }
  if (out) {
    const [w, h, sd] = out.split(/\s+/);
    if (+w !== EXPECT_DIM || +h !== EXPECT_DIM) errors.push(`ortho ${w}x${h} != ${EXPECT_DIM}^2`);
    else ok(`ortho ${w}x${h}`);
    if (!(+sd > MIN_STDDEV)) errors.push(`ortho stddev ${sd} <= ${MIN_STDDEV} (flat?)`);
    else ok(`ortho stddev ${(+sd).toFixed(4)} (> ${MIN_STDDEV})`);
  }
}

// 5) orientation guard — ortho land-mask vs DEM land-mask rendered north-up.
// The DEM raster row 0 = world Z=0 (south); useDemLayer flips it for north-up display, so we
// flip the DEM mask here. A vertically-flipped ortho fails this (AE ratio ~0.35 vs ~0.08).
//
// Two ortho classifiers (meta.waterComposite switches):
//   legacy (raw SAP): land = HSL saturation > 12 % (sea = desaturated grey seabed)
//   water-composited (T-090.1.2.5): the ocean is now saturated BLUE, so saturation would
//     misread ~68 % of the frame — instead land = NOT(water-blue hue window). Not circular:
//     the composite applied the DEM mask in ortho pixel space, so a vertical flip anywhere in
//     the pipeline leaves the blue coastline mirrored against the independently-anchored DEM
//     coast and the AE ratio blows past the same threshold.
if (existsSync(orthoPath) && existsSync(manifestPath)) {
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  const demPath = join(REPO, "packages/map-assets/everon", manifest.dem.path);
  const { heightRangeMinM: lo, heightRangeMaxM: hi } = manifest.dem;
  if (!existsSync(demPath)) {
    errors.push(`orientation guard: DEM missing at ${demPath}`);
  } else {
    const seaPct = ((0 - lo) / (hi - lo)) * 100; // DEM value mapping to sea level (land = above)
    const waterComposited = existsSync(metaPath) &&
      !!JSON.parse(readFileSync(metaPath, "utf8")).waterComposite;
    const tmp = mkdtempSync(join(tmpdir(), "sap-orient-"));
    const sapMask = join(tmp, "sap.png");
    const demMask = join(tmp, "dem.png");
    try {
      if (waterComposited) {
        // land = NOT(blue-water): hue window around the ocean palette (~203° ≈ 0.56 in HSL
        // hue scale) with a small saturation floor. -fx emits 1 (white) for land.
        execFileSync("magick", [orthoPath, "-resize", "512x512!", "-colorspace", "HSL",
          "-fx", "(u.r>=0.50 && u.r<=0.68 && u.g>0.05) ? 0 : 1", sapMask]);
      } else {
        // ortho land = saturated (colored) ground; sea = desaturated grey. Stored north-up.
        execFileSync("magick", [orthoPath, "-resize", "512x512!", "-colorspace", "HSL",
          "-channel", "G", "-separate", "+channel", "-threshold", "12%", sapMask]);
      }
      // DEM land = elevation above sea; flip raster (row0=south) to north-up to match the ortho.
      execFileSync("magick", [demPath, "-resize", "512x512!", "-threshold",
        `${seaPct.toFixed(2)}%`, "-flip", demMask]);
      // magick compare prints the metric to stderr and exits 1 when images differ — use
      // spawnSync so a non-zero exit isn't thrown and stderr is captured (not printed).
      const cmp = spawnSync("magick", ["compare", "-metric", "AE", sapMask, demMask, "null:"],
        { encoding: "utf8" });
      const aeRaw = (cmp.stderr || cmp.stdout || "").toString();
      const ae = Number(String(aeRaw).trim().split(/\s+/)[0]);
      const ratio = ae / (512 * 512);
      if (!Number.isFinite(ratio)) {
        errors.push(`orientation guard: could not parse AE ("${String(aeRaw).trim()}")`);
      } else if (!(ratio < ORIENT_MAX)) {
        errors.push(`orientation guard: ortho vs north-up DEM AE ratio ${ratio.toFixed(3)} >= ${ORIENT_MAX} (basemap upside-down?)`);
      } else {
        ok(`orientation guard: ortho matches north-up DEM (AE ratio ${ratio.toFixed(3)} < ${ORIENT_MAX})`);
      }
    } catch (e) {
      errors.push(`orientation guard failed: ${e.message}`);
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  }
}

// 4) committed z0 tile
if (!existsSync(z000) || statSync(z000).size === 0) {
  errors.push(`missing/empty committed tile ${z000}`);
} else ok(`committed satellite/0/0/0.webp (${statSync(z000).size} B)`);

if (errors.length) {
  console.error(`\nverify-sap-ortho FAIL (${errors.length}):`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
console.log("\nverify-sap-ortho OK");
