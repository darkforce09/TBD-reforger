// T-091.0 — DEM vs GetSurfaceY anchor alignment gate.
// Usage: node scripts/verify-terrain-alignment.mjs [--terrain everon] [--strict]
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

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
    console.warn(`WARN  Stub DEM — delta check deferred (T-091.0)`);
    if (strict) {
      console.error('FAIL  --strict requires exported DEM');
      process.exit(1);
    }
  }

  let anchorsFile = anchorsPath;
  if (!existsSync(anchorsPath)) {
    if (existsSync(examplePath)) {
      console.warn('WARN  Using verification.example.json');
      anchorsFile = examplePath;
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
    console.error(`FAIL  --strict requires ≥${MIN_ANCHORS_STRICT} anchors`);
    process.exit(1);
  }

  if (stubDem) {
    console.log('\nverify-terrain-alignment: OK (stub schema only)');
    process.exit(0);
  }

  let failures = 0;
  let maxDelta = 0;
  for (const a of anchors) {
    if (!Number.isFinite(a.demYM) || !Number.isFinite(a.surfaceYM)) {
      if (strict) {
        console.error(`FAIL  ${a.id}: missing demYM or surfaceYM`);
        failures++;
      }
      continue;
    }
    const delta = Math.abs(a.demYM - a.surfaceYM);
    maxDelta = Math.max(maxDelta, delta);
    if (delta > threshold) {
      console.error(`FAIL  ${a.id}: delta ${delta.toFixed(3)} m > ${threshold} m`);
      failures++;
    } else {
      console.log(`PASS  ${a.id}: delta ${delta.toFixed(3)} m`);
    }
  }

  if (failures) process.exit(1);
  console.log(`\nverify-terrain-alignment: OK (max delta ${maxDelta.toFixed(3)} m)`);
}

main();
