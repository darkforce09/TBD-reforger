// T-091.0 — DEM vs GetSurfaceY anchor alignment gate (computes demYM from PNG math).
// Usage: node scripts/verify-terrain-alignment.mjs [--terrain everon] [--strict]
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
import { PNG } from 'pngjs';
import {
  rasterFromPngjs,
  sampleElevationMeters,
  worldToPixel,
} from './lib/dem-sample.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = resolve(root, '../..');
const MIN_ANCHORS_STRICT = 10;

function parseArgs(argv) {
  const terrainIdx = argv.indexOf('--terrain');
  const terrain = terrainIdx >= 0 ? argv[terrainIdx + 1] : 'everon';
  const strict = argv.includes('--strict');
  return { terrain, strict };
}

function readJSON(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function loadValidator(name) {
  const ajv = new Ajv({ allErrors: true, strict: true });
  addFormats(ajv);
  return ajv.compile(readJSON(resolve(root, 'schema', name)));
}

function loadDemRaster(manifest) {
  const demPath = resolve(repoRoot, 'packages/map-assets', manifest.terrainId, manifest.dem.path);
  if (!existsSync(demPath)) {
    throw new Error(`DEM file missing: ${demPath}`);
  }
  const png = PNG.sync.read(readFileSync(demPath), { skipRescale: true });
  const { raster, width, height } = rasterFromPngjs(png);
  if (width !== manifest.dem.widthPx || height !== manifest.dem.heightPx) {
    throw new Error(
      `PNG IHDR ${width}×${height} !== manifest ${manifest.dem.widthPx}×${manifest.dem.heightPx}`,
    );
  }
  return { raster, width, height, demPath };
}

function main() {
  const { terrain, strict } = parseArgs(process.argv.slice(2));
  const manifestPath = resolve(repoRoot, `packages/map-assets/${terrain}/manifest.json`);
  const anchorsPath = resolve(repoRoot, `packages/map-assets/${terrain}/anchors/verification.json`);
  const examplePath = resolve(
    repoRoot,
    `packages/map-assets/${terrain}/anchors/verification.example.json`,
  );

  const manifest = readJSON(manifestPath);
  const validateManifest = loadValidator('terrain-manifest.schema.json');
  if (!validateManifest(manifest)) {
    console.error('FAIL  Manifest schema');
    process.exit(1);
  }

  const stubDem = manifest.dem.widthPx === 0 || manifest.dem.heightPx === 0;
  if (stubDem) {
    console.warn('WARN  Stub DEM (widthPx/heightPx=0) — strict anchor math deferred');
    if (strict) {
      console.error('FAIL  --strict requires exported DEM with widthPx/heightPx > 0');
      process.exit(1);
    }
  }

  let anchorsFile = anchorsPath;
  if (!existsSync(anchorsPath)) {
    if (existsSync(examplePath)) {
      console.warn('WARN  Using verification.example.json (not production anchors)');
      anchorsFile = examplePath;
      if (strict) {
        console.error('FAIL  --strict requires packages/map-assets/everon/anchors/verification.json');
        process.exit(1);
      }
    } else {
      console.log('\nverify-terrain-alignment: OK (no anchors file)');
      process.exit(0);
    }
  }

  const anchorsDoc = readJSON(anchorsFile);
  const validateAnchors = loadValidator('terrain-anchors.schema.json');
  if (!validateAnchors(anchorsDoc)) {
    console.error('FAIL  Anchors schema');
    process.exit(1);
  }
  console.log(`PASS  Anchors validate (${anchorsFile})`);

  const threshold = anchorsDoc.thresholdM ?? 1.0;
  const anchors = anchorsDoc.anchors ?? [];

  if (strict && anchors.length < MIN_ANCHORS_STRICT) {
    console.error(`FAIL  --strict requires ≥${MIN_ANCHORS_STRICT} anchors, got ${anchors.length}`);
    process.exit(1);
  }

  if (stubDem) {
    console.log('\nverify-terrain-alignment: OK (stub — schema only)');
    process.exit(0);
  }

  let dem;
  try {
    dem = loadDemRaster(manifest);
    console.log(`PASS  DEM PNG ${dem.width}×${dem.height} @ ${dem.demPath}`);
  } catch (e) {
    console.error(`FAIL  ${e.message}`);
    process.exit(1);
  }

  let failures = 0;
  let maxDelta = 0;
  console.log('\nAnchor elevation verify (|demYM - surfaceYM| ≤ thresholdM):');
  console.log('id\tx\tz\tsurfaceYM\tdemYM\tdeltaM\tPASS');

  for (const a of anchors) {
    if (!Number.isFinite(a.surfaceYM)) {
      console.error(`FAIL  ${a.id}: surfaceYM missing or non-finite`);
      failures++;
      continue;
    }
    let demYM;
    try {
      demYM = sampleElevationMeters(a.x, a.z, manifest, dem.raster, dem.width, dem.height);
    } catch (e) {
      console.error(`FAIL  ${a.id}: ${e.message}`);
      failures++;
      continue;
    }
    const delta = Math.abs(demYM - a.surfaceYM);
    maxDelta = Math.max(maxDelta, delta);
    const pass = delta <= threshold;
    console.log(
      `${a.id}\t${a.x}\t${a.z}\t${a.surfaceYM.toFixed(3)}\t${demYM.toFixed(3)}\t${delta.toFixed(3)}\t${pass ? 'PASS' : 'FAIL'}`,
    );
    if (!pass) failures++;
  }

  // Horizontal sanity: anchors must lie inside worldBounds
  const [, , maxX, maxY] = manifest.worldBounds;
  for (const a of anchors) {
    if (a.x < 0 || a.x > maxX || a.z < 0 || a.z > maxY) {
      console.error(`FAIL  ${a.id}: (${a.x}, ${a.z}) outside worldBounds`);
      failures++;
    }
    const { u, v } = worldToPixel(a.x, a.z, manifest);
    if (u < 0 || u > 1 || v < 0 || v > 1) {
      console.error(`FAIL  ${a.id}: normalized (u,v)=(${u},${v}) outside [0,1]`);
      failures++;
    }
  }

  console.log(`\nmaxDeltaM=${maxDelta.toFixed(3)} thresholdM=${threshold}`);

  if (failures) {
    console.error(`\n${failures} failure(s) — slice FAIL`);
    process.exit(1);
  }
  console.log('\nverify-terrain-alignment: OK');
}

main();
