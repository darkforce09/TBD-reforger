// T-090.1.2 — enumerate Everon SAP supertexture cells -> cell-catalog.json.
//
// Lists the 2500 `worlds/Eden/Eden/.Data/Eden_<N>_supertexture.edds` cells and
// their grid + world placement (row-major N = y*50 + x; cell gridY=0 = world Z=0 =
// south; ortho assembled north-up; 256 m / 256 px per cell). Hard-fails if count != 2500. Full decode +
// fail-fast happens in stitch-sap-ortho.mjs; this is the fast index/manifest.
//
//   node scripts/map-assets/catalog-sap-cells.mjs [TERRAIN=everon]
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  CELL_COUNT,
  CELL_M,
  CELL_PX,
  GRID,
  WORLD_M,
  cellGrid,
  cellPath,
  listEdenCells,
} from "./decode-edds.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "../..");
const OUT_DIR = join(REPO, "packages/map-assets/everon/staging/sap");
const OUT = join(OUT_DIR, "cell-catalog.json");

const terrain =
  (process.argv.find((a) => a.startsWith("TERRAIN=")) || "TERRAIN=everon").split("=")[1];
if (terrain !== "everon") {
  console.error(`only everon supported this slice (got ${terrain})`);
  process.exit(1);
}

const cells = await listEdenCells();
if (cells.length !== CELL_COUNT) {
  console.error(`FAIL: found ${cells.length} Eden cells, expected ${CELL_COUNT}`);
  process.exit(1);
}

const entries = cells.map(({ n }) => {
  const { gridX, gridY } = cellGrid(n);
  const worldMinX = gridX * CELL_M;
  // cell gridY=0 = world Z=0 (south); world Z increases with gridY toward north.
  const worldMinZ = gridY * CELL_M;
  return {
    id: n,
    eddsPath: cellPath(n),
    gridX,
    gridY,
    widthPx: CELL_PX,
    heightPx: CELL_PX,
    worldMinX,
    worldMinZ,
    // ortho is assembled north-up (world Z=0/south at the image bottom).
    pixelX: gridX * CELL_PX,
    pixelY: (GRID - 1 - gridY) * CELL_PX,
  };
});

const catalog = {
  terrain,
  slice: "T-090.1.2",
  generatedAt: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
  grid: GRID,
  cellCount: entries.length,
  cellMeters: CELL_M,
  cellPx: CELL_PX,
  metersPerPixel: CELL_M / CELL_PX,
  worldBounds: [0, 0, WORLD_M, WORLD_M],
  orthoPx: [GRID * CELL_PX, GRID * CELL_PX],
  gridMapping: "row-major N=y*50+x; cell gridY=0 = world Z=0 (south); ortho north-up (south at image bottom, pixelY=(49-gridY)*256)",
  source: "sap-supertexture-stitch",
  cells: entries,
};

mkdirSync(OUT_DIR, { recursive: true });
writeFileSync(OUT, JSON.stringify(catalog, null, 2) + "\n");
console.log(`wrote ${OUT} (${entries.length} cells, ortho ${catalog.orthoPx.join("x")})`);
