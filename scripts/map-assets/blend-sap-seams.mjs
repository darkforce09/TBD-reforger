// T-090.1.2.2 — SAP cell seam repair (strategy A: apron-bridge feather).
//
// Each Eden `_supertexture.edds` cell carries a baked ~3–4 px constant apron on every edge
// (proven in P0). Tiled edge-to-edge those aprons stack into an ~8 px DEAD-FLAT band at every
// interior 256 px seam → a grid of blurry lines/crosses over sharp terrain at max zoom.
//
// `bridgeSeams` replaces that dead band, per line, with a linear cross-fade between the nearest
// DETAILED lines on each side (anchors at c-ANCHOR and c+HW). The apron held no detail, so the
// bridge loses nothing and removes the flat strip AND its sharp bounding edges (the visible
// "line"). Detail is NOT invented (no upscale/inpaint) — a smooth ~8 px transition remains,
// which is the best achievable from this source. Interior seams only (world borders untouched).
//
// Two entry points:
//   * bridgeSeams(canvas, opts)  — pure RGBA-buffer op, called by stitch-sap-ortho.mjs in final
//     (north-up) canvas space, before PNG write. The row-flip assembly is untouched.
//   * CLI `node blend-sap-seams.mjs TERRAIN=everon` — pak-free fallback: bridge the EXISTING
//     ortho PNG in place (magick decode → bridge → encode) and write the identical seamRepair
//     fields into TBD_SatExport_meta.json (NIT-4), so provenance matches the stitch path.
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ANCHOR, HW } from "./lib/sap-seam-metrics.mjs";
import { CELL_PX, GRID } from "./decode-edds.mjs";

/** Meta provenance written by BOTH the stitch path and the CLI fallback (NIT-4). */
export const SEAM_REPAIR_META = {
  seamRepair: "T-090.1.2.2",
  seamRepairStrategy: `A-apron-bridge-${HW}px`,
  seamRepairParams: { halfWidthPx: HW, anchorOffsetPx: ANCHOR, interiorSeamsOnly: true },
};

/**
 * Bridge the dead apron band at every interior cell seam, in place, on an interleaved pixel
 * buffer assembled in final (north-up) canvas space.
 *
 * @param {Buffer} canvas  interleaved pixels, row-major
 * @param {object} opts
 * @param {number} opts.orthoPx   full ortho side in px (e.g. 12800)
 * @param {number} [opts.cellPx]  cell side in px (seam spacing) — default CELL_PX (256)
 * @param {number} [opts.grid]    cells per axis — default GRID (50)
 * @param {number} [opts.halfWidth] band half-width — default HW (4) → 8 px band
 * @param {number} [opts.channels] bytes/pixel — default 4 (RGBA)
 * @returns {number} number of interior seams bridged per axis
 */
export function bridgeSeams(canvas, opts) {
  const { orthoPx } = opts;
  const cellPx = opts.cellPx ?? CELL_PX;
  const grid = opts.grid ?? GRID;
  const hw = opts.halfWidth ?? HW;
  const ch = opts.channels ?? 4;
  const anchor = hw + 1; // anchors sit at c-anchor and c+hw; span = 2*hw+1
  const span = 2 * hw + 1;
  const stride = orthoPx * ch;
  if (canvas.length !== stride * orthoPx) {
    throw new Error(`bridgeSeams: canvas ${canvas.length} B != ${stride * orthoPx} (${orthoPx}²×${ch})`);
  }

  // Interior seam lines only: c = cellPx·k, k = 1..grid-1 (skip 0 and orthoPx world borders).
  const seams = [];
  for (let k = 1; k < grid; k++) seams.push(k * cellPx);

  // ── Vertical pass: bridge columns [c-hw, c+hw-1] using column anchors c-anchor / c+hw. ──
  // Reads only anchor columns (never written — seams are cellPx apart) so in-place is safe.
  for (const c of seams) {
    const aL = c - anchor;
    const aR = c + hw;
    for (let y = 0; y < orthoPx; y++) {
      const row = y * stride;
      const oL = row + aL * ch;
      const oR = row + aR * ch;
      for (let x = c - hw; x <= c + hw - 1; x++) {
        const t = (x - aL) / span;
        const o = row + x * ch;
        for (let k = 0; k < 3; k++) {
          canvas[o + k] = Math.round(canvas[oL + k] + (canvas[oR + k] - canvas[oL + k]) * t);
        }
      }
    }
  }

  // ── Horizontal pass: bridge rows [c-hw, c+hw-1] using row anchors c-anchor / c+hw. ──
  // Reads the vertically-bridged canvas so 4-cell crosses fill from the V-bridged columns;
  // anchor rows are never written by this pass, so in-place is safe.
  for (const c of seams) {
    const aT = c - anchor;
    const aB = c + hw;
    for (let x = 0; x < orthoPx; x++) {
      const col = x * ch;
      const oT = aT * stride + col;
      const oB = aB * stride + col;
      for (let y = c - hw; y <= c + hw - 1; y++) {
        const t = (y - aT) / span;
        const o = y * stride + col;
        for (let k = 0; k < 3; k++) {
          canvas[o + k] = Math.round(canvas[oT + k] + (canvas[oB + k] - canvas[oT + k]) * t);
        }
      }
    }
  }

  return seams.length;
}

// ── CLI fallback: bridge the existing ortho PNG in place (no pak / no re-decode) ─────────────
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const HERE = dirname(fileURLToPath(import.meta.url));
  const REPO = join(HERE, "../..");
  const terrain =
    (process.argv.find((a) => a.startsWith("TERRAIN=")) || "TERRAIN=everon").split("=")[1];
  if (terrain !== "everon") {
    console.error(`only everon supported this slice (got ${terrain})`);
    process.exit(1);
  }
  const SAP = join(REPO, "packages/map-assets/everon/staging/sap");
  const pngPath = join(SAP, "everon-sap-ortho.png");
  const metaPath = join(SAP, "TBD_SatExport_meta.json");
  if (!existsSync(pngPath)) {
    console.error(`FAIL: ${pngPath} missing — run stitch-sap-ortho.mjs first`);
    process.exit(1);
  }
  const orthoPx = GRID * CELL_PX;
  const tmp = mkdtempSync(join(tmpdir(), "blend-sap-"));
  const raw = join(tmp, "ortho.rgba");
  try {
    console.error(`blend-sap-seams (CLI fallback): decoding ${pngPath} …`);
    execFileSync("magick", [pngPath, "-depth", "8", `RGBA:${raw}`], { stdio: "inherit" });
    const canvas = readFileSync(raw);
    const n = bridgeSeams(canvas, { orthoPx, channels: 4 });
    writeFileSync(raw, canvas);
    execFileSync(
      "magick",
      ["-size", `${orthoPx}x${orthoPx}`, "-depth", "8", `rgba:${raw}`, "-alpha", "off", pngPath],
      { stdio: "inherit", maxBuffer: 1 << 30 },
    );
    // NIT-4: same seamRepair provenance as the stitch path.
    if (existsSync(metaPath)) {
      const meta = JSON.parse(readFileSync(metaPath, "utf8"));
      Object.assign(meta, SEAM_REPAIR_META);
      writeFileSync(metaPath, JSON.stringify(meta, null, 2) + "\n");
    }
    console.log(`blend-sap-seams: bridged ${n} interior seams/axis in ${pngPath}`);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}
