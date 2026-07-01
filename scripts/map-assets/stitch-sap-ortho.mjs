// T-090.1.2 — stitch the 2500 Everon SAP cells into one world ortho.
//
// Decodes mip0 (256x256 RGBA) of every Eden_<N>_supertexture.edds and pastes it
// row-major (N = y*50 + x) into a 12800x12800 canvas assembled NORTH-UP: cell gridY=0 is
// world Z=0 (south) and lands at the image bottom (the editor renders image-top at maxZ/north),
// then writes everon-sap-ortho.png (via ImageMagick) + TBD_SatExport_meta.json.
//
// HARD: any cell that is missing / corrupt / wrong-size ABORTS the whole build.
// No grey / placeholder fill — a hole must fail loudly, never be papered over.
//
//   node scripts/map-assets/stitch-sap-ortho.mjs [TERRAIN=everon]
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  CELL_COUNT,
  CELL_M,
  CELL_PX,
  GRID,
  WORLD_M,
  cellGrid,
  decodeCellRgba,
  listEdenCells,
} from "./decode-edds.mjs";
import { SEAM_REPAIR_META, bridgeSeams } from "./blend-sap-seams.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "../..");
const OUT_DIR = join(REPO, "packages/map-assets/everon/staging/sap");

const terrain =
  (process.argv.find((a) => a.startsWith("TERRAIN=")) || "TERRAIN=everon").split("=")[1];
if (terrain !== "everon") {
  console.error(`only everon supported this slice (got ${terrain})`);
  process.exit(1);
}

const ORTHO_PX = GRID * CELL_PX; // 12800
const t0 = Date.now();

const cells = await listEdenCells();
if (cells.length !== CELL_COUNT) {
  console.error(`FAIL: found ${cells.length} Eden cells, expected ${CELL_COUNT} — aborting (no holes)`);
  process.exit(1);
}

// 12800 x 12800 x 4 RGBA canvas (~655 MB).
const stride = ORTHO_PX * 4;
const canvas = Buffer.alloc(ORTHO_PX * ORTHO_PX * 4);

let decoded = 0;
for (const { n } of cells) {
  let cell;
  try {
    cell = await decodeCellRgba(n);
  } catch (e) {
    console.error(`FAIL: cell ${n} decode error: ${e.message} — aborting (no grey fill)`);
    process.exit(1);
  }
  if (cell.side !== CELL_PX || cell.rgba.length !== CELL_PX * CELL_PX * 4) {
    console.error(`FAIL: cell ${n} wrong size (side ${cell.side}) — aborting`);
    process.exit(1);
  }
  const { gridX, gridY } = cellGrid(n);
  const px = gridX * CELL_PX;
  // North-up assembly: cell gridY=0 is world Z=0 (south) and must land at the image
  // BOTTOM (the editor renders the image top at maxZ/north). Mirror the whole canvas
  // vertically in one pass — flip the cell's slot (GRID-1-gridY) AND reverse its interior
  // rows, so destRow = H-1-(gridY*CELL_PX+row) and cells still tile smoothly. (Flipping the
  // slot WITHOUT reversing rows would internally-mirror every 256 px cell = striping.)
  const pyTop = (GRID - 1 - gridY) * CELL_PX;
  const cellStride = CELL_PX * 4;
  for (let row = 0; row < CELL_PX; row++) {
    const dstRow = pyTop + (CELL_PX - 1 - row);
    const dstOff = dstRow * stride + px * 4;
    cell.rgba.copy(canvas, dstOff, row * cellStride, (row + 1) * cellStride);
  }
  decoded++;
  if (decoded % 250 === 0) {
    process.stderr.write(`  decoded ${decoded}/${CELL_COUNT}\n`);
  }
}

if (decoded !== CELL_COUNT) {
  console.error(`FAIL: decoded ${decoded} != ${CELL_COUNT} — aborting`);
  process.exit(1);
}

// T-090.1.2.2 — repair the baked-apron flat band at every interior 256 px cell seam
// (strategy A: apron-bridge feather) in final NORTH-UP canvas space, before PNG write. The
// row-flip assembly above (per-row vertical mirror) is untouched — the bridge only rewrites the
// ~8 px dead band straddling each interior seam, linearly cross-fading between the nearest
// detailed lines. World borders (x/y = 0, 12800) are skipped.
const seamsBridged = bridgeSeams(canvas, { orthoPx: ORTHO_PX });
console.error(`  seam repair: bridged ${seamsBridged} interior seams/axis (apron feather HW=4)`);

mkdirSync(OUT_DIR, { recursive: true });
const rawPath = join(OUT_DIR, ".everon-sap-ortho.rgba");
const pngPath = join(OUT_DIR, "everon-sap-ortho.png");
writeFileSync(rawPath, canvas);
// raw RGBA -> PNG (drop alpha; satellite ortho is opaque RGB)
execFileSync(
  "magick",
  ["-size", `${ORTHO_PX}x${ORTHO_PX}`, "-depth", "8", `rgba:${rawPath}`, "-alpha", "off", pngPath],
  { stdio: "inherit", maxBuffer: 1 << 30 }
);
rmSync(rawPath, { force: true });

const elapsedSec = Math.round((Date.now() - t0) / 1000);
const meta = {
  slice: "T-090.1.2",
  source: "sap-supertexture-stitch",
  captureMethodId: 6,
  terrain,
  dimensions: [ORTHO_PX, ORTHO_PX],
  metersPerPixel: CELL_M / CELL_PX, // 1
  worldBounds: [0, 0, WORLD_M, WORLD_M],
  grid: GRID,
  cellsDecoded: decoded,
  cellPx: CELL_PX,
  cellMeters: CELL_M,
  gridMapping: "row-major N=y*50+x; cell gridY=0 = world Z=0 (south); assembled north-up (south at image bottom)",
  decoder: "decode-edds.mjs + vendor/bcdec.wasm (BC7) + pure-JS LZ4",
  ...SEAM_REPAIR_META,
  pngPath: "packages/map-assets/everon/staging/sap/everon-sap-ortho.png",
  buildSeconds: elapsedSec,
  generatedAt: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
};
writeFileSync(join(OUT_DIR, "TBD_SatExport_meta.json"), JSON.stringify(meta, null, 2) + "\n");
console.log(
  `wrote ${pngPath} (${ORTHO_PX}x${ORTHO_PX}, ${decoded} cells, ${elapsedSec}s)`
);
