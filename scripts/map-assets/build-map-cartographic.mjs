// T-090.1.1 — Build the Map-view staging ortho (stylized cartographic, NOT satellite).
// T-090.1.1.1 — land-cover tints (forest / open) on the same compose path.
//
// Base raster is the MapDataExporter.ExportRasterization TGA shipped @ T-090.1 (G1-A winner,
// .ai/artifacts/t090_1_1_source_spike.json): 4096×4096, top-origin (north-up), stylized
// land/ocean palette + relief shading. That capture tier was honestly rejected for the
// SATELLITE tab (T-090.1.2.4) — it is the correct product tier for the MAP tab. Never merge
// this raster into the SAP satellite bundle.
//
// The TGA's land is a single relief-shaded olive band (its forestArea palette never rendered
// — all 5.28M land px sit in R∈[68,105), t090_1_1_1_source_spike.json), it carries no road
// network, and it misses most inland water. The cartographic product ("roads + terrain
// palette", t090_basemap_dual_view.md) is therefore composed here, all offline:
//   1. tint land cover at source res (4096²) from the SAP-appearance masks
//      (build-landcover-mask.mjs, T-090.1.1.1): open/bright fields lighten toward tan,
//      forest darkens toward canopy green — partial alpha so the TGA relief shading ghosts
//      through both (tinting pre-upscale is visually identical and ~10× cheaper);
//   2. upscale → full world extent (Everon 12800², 1 m/px) with Lanczos;
//   3. tint inland water from the T-090.1.2.5.2 classifier mask (read-only reuse — the
//      heuristic itself is frozen; ocean is already in the TGA palette);
//   4. stroke the .topo road network (decode-topo.mjs — airfield + 4 road tiers, the same
//      vectors .5.2 used for road subtraction) as an MVG draw pass.
//
// Output: packages/map-assets/<terrain>/staging/map/<terrain>-map-ortho.png, north-up.
// No vertical flip anywhere on this path: TGA top row = north = on-disk XYZ row 0
// (build-tile-pyramid.sh must NOT get --flip-v); the frontend tileUrl() applies the single
// XYZ↔south-first inversion at fetch.
//
// Usage:
//   TERRAIN=everon node scripts/map-assets/build-map-cartographic.mjs
//
// Requires magick (ImageMagick 7) on PATH (same toolchain as build-tile-pyramid.sh).
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(HERE, '..', '..');

// Per-terrain cartographic sources. Arland has no MapDataExporter capture yet — re-run the
// Workbench export (see error text below) and add a row when it lands.
const SOURCES = {
  everon: {
    tga: 'packages/map-assets/everon/staging/spike/TBD_SatExport_everon.tga',
    out: 'packages/map-assets/everon/staging/map/everon-map-ortho.png',
    waterMask: 'packages/map-assets/everon/staging/sap/water-inland-mask.png',
    worldPx: 12800,
    sourcePx: 4096,
  },
};

// Cartographic stroke per .topo type (tier semantics per T-090.1.2.5.2: airfield + 4 road
// classes). Widths in metres (= px at 1 m/px), kept near real carriageway widths — the
// vectors carry short driveway/junction stubs, and over-wide strokes turn those into
// scalloped blobs at high zoom.
const ROAD_STYLE = {
  0: { color: '#9aa3a2', width: 20 }, // airfield / runway
  1: { color: '#b0452b', width: 10 }, // primary route
  2: { color: '#c8823c', width: 8 }, // secondary road
  3: { color: '#ded6bd', width: 5 }, // minor road
  5: { color: '#7a7466', width: 3 }, // track / trail
};
const WATER_COLOR = '#2E5266'; // the TGA's own ocean teal — inland water matches it

// Land-cover tints (T-090.1.1.1, L1 winner — t090_1_1_1_source_spike.json). Alphas < 1 so
// the TGA relief shading stays visible under both tints; grass keeps the raw TGA olive.
const LANDCOVER_STYLE = {
  open: { color: '#CDC6A3', alpha: 0.7 }, // tan fields / plow / urban — lighter than grass
  forest: { color: '#37502D', alpha: 0.8 }, // canopy green — darker than grass
};

const terrain = process.env.TERRAIN || 'everon';
const cfg = SOURCES[terrain];
if (!cfg) {
  console.error(
    `build-map-cartographic: no cartographic source registered for terrain "${terrain}".\n` +
      `Export one first: Workbench → Plugins → TBD → "Export TBD Satellite" (MapDataExporter\n` +
      `rasterization via TBD_SatelliteExportPlugin.c), copy the TGA out of the Proton profile\n` +
      `(scripts/map-assets/copy-world-export-profile.mjs), then add a SOURCES row here.`,
  );
  process.exit(1);
}

const tga = join(repoRoot, cfg.tga);
const out = join(repoRoot, cfg.out);
if (!existsSync(tga)) {
  console.error(
    `build-map-cartographic: source raster missing: ${cfg.tga}\n` +
      `staging/ is gitignored (local scratch) — regenerate via the Workbench export above.`,
  );
  process.exit(1);
}

const started = Date.now();
mkdirSync(dirname(out), { recursive: true });

// ── Road MVG from the .topo vector network (y is already north-up image metres) ──────────
// The raw vectors are NOT clean centerlines: at (nearly) every centerline vertex the record
// detours to an offset point one road-width away and back (a geometry-baked width encoding —
// constant per record, e.g. exactly 8.0 m on secondaries). Stroked as-is that draws a comb of
// perpendicular stubs along every road. `despike` strips the excursions back to a centerline:
//   1. drop consecutive duplicate points;
//   2. drop A,B,A′ return-spikes (path comes back to where it was);
//   3. drop lone vertices deviating > 3.5 m perpendicular from their neighbor chord —
//      real road curvature over 8–30 m segments stays well under that; only the width
//      stubs (and the odd junction corner, acceptably rounded) exceed it.
function despike(verts) {
  let pts = [];
  for (let i = 0; i < verts.length; i += 2) pts.push([verts[i], verts[i + 1]]);
  const d2 = (a, b) => (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2;
  pts = pts.filter((p, i) => i === 0 || d2(p, pts[i - 1]) > 0.01);
  const perp2 = (p, a, b) => {
    const abx = b[0] - a[0];
    const aby = b[1] - a[1];
    const len2 = abx * abx + aby * aby;
    if (len2 < 1e-6) return d2(p, a);
    const t = Math.max(0, Math.min(1, ((p[0] - a[0]) * abx + (p[1] - a[1]) * aby) / len2));
    return d2(p, [a[0] + t * abx, a[1] + t * aby]);
  };
  for (let changed = true; changed; ) {
    changed = false;
    const keep = [pts[0]];
    for (let i = 1; i < pts.length - 1; i++) {
      const prev = keep[keep.length - 1];
      const next = pts[i + 1];
      if (d2(prev, next) < 1 || perp2(pts[i], prev, next) > 3.5 ** 2) {
        changed = true; // return-spike apex or width-stub excursion — drop
      } else {
        keep.push(pts[i]);
      }
    }
    keep.push(pts[pts.length - 1]);
    pts = keep.filter((p, i) => i === 0 || d2(p, keep[i - 1]) > 0.01);
  }
  return pts;
}

const { decodeTopo } = await import(join(HERE, 'decode-topo.mjs'));
const { buildLandcoverMasks } = await import(join(HERE, 'build-landcover-mask.mjs'));
const topo = await decodeTopo(terrain);

// ── Land-cover masks from the SAP appearance (rebuilt every compose — ~6 s, deterministic) ─
const landcover = buildLandcoverMasks(terrain);
const mvgPath = join(dirname(out), 'roads.mvg');
const mvg = ['fill none stroke-linecap round stroke-linejoin round'];
const drawn = { records: 0, verts: 0, rawVerts: 0 };
// Draw airfields first, then descending tier so primaries stay on top at crossings.
for (const type of [0, 5, 3, 2, 1]) {
  const style = ROAD_STYLE[type];
  for (const rec of topo.records) {
    if (rec.type !== type || rec.verts.length < 4) continue;
    const pts = despike(rec.verts);
    if (pts.length < 2) continue;
    const str = pts.map((p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ');
    mvg.push(`stroke '${style.color}' stroke-width ${style.width} polyline ${str}`);
    drawn.records += 1;
    drawn.verts += pts.length;
    drawn.rawVerts += rec.verts.length / 2;
  }
}
writeFileSync(mvgPath, `${mvg.join('\n')}\n`);

// ── Compose: upscale → inland-water tint → road strokes, one magick pass ─────────────────
const waterMask = join(repoRoot, cfg.waterMask);
const hasWater = existsSync(waterMask);
if (!hasWater)
  console.warn(
    `build-map-cartographic: ${cfg.waterMask} missing — shipping without inland-water tint`,
  );
const size = `${cfg.worldPx}x${cfg.worldPx}`;
// Two bounded passes (upscale+water, then roads) — one 12800² Q16 pipeline holding base +
// overlay + draw canvas peaks past ~7 GB and gets OOM-killed on a loaded box; -limit lets
// ImageMagick spill to disk instead of dying. The spill MUST land on a real disk: /tmp is
// tmpfs here, so the default temp path turns "disk" spill back into RAM (observed SIGKILL
// with multi-GB /tmp/magick-* residue).
const LIMITS = ['-limit', 'memory', '3GiB', '-limit', 'map', '6GiB'];
const MAGICK_ENV = { ...process.env, MAGICK_TEMPORARY_PATH: '/var/tmp' };
// Land-cover tints run at SOURCE resolution (4096²), before the upscale: solid colour with
// the (alpha-capped) class mask as its opacity, Over-composited — same mechanism as the
// water tint below. Tinting after the upscale is visually identical (the soft mask edges
// ride the same Lanczos) but needs three 12800² Q16 overlay stages in one pipeline, which
// blew past the -limit budget and spilled multi-GB to disk (observed 10min+ vs ~1min).
// Open first, forest second (forest wins where the soft edges overlap, which is the right
// call at a treeline against a field).
const srcSize = `${cfg.sourcePx}x${cfg.sourcePx}`;
const tinted = join(dirname(out), 'tinted-base-tmp.png');
const tintArgs = [...LIMITS, tga, '-alpha', 'off'];
for (const [name, mask] of [
  ['open', landcover.brightMask],
  ['forest', landcover.forestMask],
]) {
  const style = LANDCOVER_STYLE[name];
  tintArgs.push(
    '(',
    '-size',
    srcSize,
    `xc:${style.color}`,
    '(',
    mask,
    '-alpha',
    'off',
    '-resize',
    `${srcSize}!`,
    '-evaluate',
    'Multiply',
    String(style.alpha),
    ')',
    '-compose',
    'CopyOpacity',
    '-composite',
    ')',
    '-compose',
    'Over',
    '-composite',
  );
}
tintArgs.push(tinted);
execFileSync('magick', tintArgs, { stdio: 'inherit', env: MAGICK_ENV });

const args = [...LIMITS, tinted, '-filter', 'Lanczos', '-resize', `${size}!`];
if (hasWater) {
  // Solid-teal layer with the classifier mask as its alpha (white = water), Over-composited.
  args.push(
    '(',
    '-size',
    size,
    `xc:${WATER_COLOR}`,
    '(',
    waterMask,
    '-alpha',
    'off',
    ')',
    '-compose',
    'CopyOpacity',
    '-composite',
    ')',
    '-compose',
    'Over',
    '-composite',
  );
}
args.push(out);
execFileSync('magick', args, { stdio: 'inherit', env: MAGICK_ENV });
rmSync(tinted);
execFileSync('magick', [...LIMITS, out, '-draw', `@${mvgPath}`, out], {
  stdio: 'inherit',
  env: MAGICK_ENV,
});
rmSync(mvgPath);

const meta = {
  slice: 'T-090.1.1.1',
  source: 'workbench-cartographic',
  terrain,
  sourceRaster: cfg.tga,
  sourceDimensions: [cfg.sourcePx, cfg.sourcePx],
  dimensions: [cfg.worldPx, cfg.worldPx],
  worldBounds: [0, 0, cfg.worldPx, cfg.worldPx],
  upscale: `${cfg.sourcePx}->${cfg.worldPx} magick -filter Lanczos (documented upscale, slice spec §1)`,
  orientation: 'north-up (TGA top origin preserved; no flips on this path)',
  overlays: {
    landCover: {
      source: 'build-landcover-mask.mjs (SAP appearance heuristic, L1)',
      thresholds: landcover.meta.thresholds,
      fractions: landcover.meta.fractions,
      style: LANDCOVER_STYLE,
      provenance: 'T-090.1.1.1 — SAP ortho read-only; satellite bundle untouched',
    },
    inlandWater: hasWater
      ? { mask: cfg.waterMask, color: WATER_COLOR, provenance: 'T-090.1.2.5.2 classifier (read-only reuse)' }
      : null,
    roads: {
      source: 'decode-topo.mjs (.topo vector network)',
      records: drawn.records,
      vertices: drawn.verts,
      style: ROAD_STYLE,
    },
  },
  spikeArtifact: '.ai/artifacts/t090_1_1_1_source_spike.json',
  buildSeconds: Math.round((Date.now() - started) / 1000),
  generatedAt: new Date().toISOString(),
};
writeFileSync(join(dirname(out), 'map-ortho-meta.json'), `${JSON.stringify(meta, null, 2)}\n`);
console.log(
  `build-map-cartographic: OK ${cfg.out} (${cfg.worldPx}² north-up, ` +
    `${drawn.records} road records / ${drawn.verts} verts, water=${hasWater}, ${meta.buildSeconds}s)`,
);
