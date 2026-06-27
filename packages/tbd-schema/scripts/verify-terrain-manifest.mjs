// T-090.0 — Cross-check terrain manifest vs terrains.ts contract + schema validate.
// Usage: node scripts/verify-terrain-manifest.mjs [--terrain everon]
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = resolve(root, '../..');

const TERRAINS_TS = {
  everon: {
    width: 12800,
    height: 12800,
    heightRangeMinM: -204.78,
    heightRangeMaxM: 375.53,
  },
  arland: {
    width: 4096,
    height: 4096,
    heightRangeMinM: -163,
    heightRangeMaxM: 148.38,
  },
};

function parseArgs(argv) {
  const i = argv.indexOf('--terrain');
  const terrain = i >= 0 ? argv[i + 1] : 'everon';
  if (!TERRAINS_TS[terrain]) {
    console.error(`Unknown terrain "${terrain}". Use: everon | arland`);
    process.exit(2);
  }
  return { terrain };
}

function readJSON(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function main() {
  const { terrain } = parseArgs(process.argv.slice(2));
  const manifestPath = resolve(repoRoot, `packages/map-assets/${terrain}/manifest.json`);
  const schemaPath = resolve(root, 'schema/terrain-manifest.schema.json');

  let manifest;
  try {
    manifest = readJSON(manifestPath);
  } catch (e) {
    console.error(`FAIL  Cannot read manifest: ${manifestPath}`);
    console.error(e.message);
    process.exit(1);
  }

  const ajv = new Ajv({ allErrors: true, strict: true });
  addFormats(ajv);
  const validate = ajv.compile(readJSON(schemaPath));
  if (!validate(manifest)) {
    console.error('FAIL  Manifest schema validation:');
    for (const err of validate.errors ?? []) {
      console.error(`      ${err.instancePath || '/'} ${err.message}`);
    }
    process.exit(1);
  }
  console.log('PASS  Manifest validates against terrain-manifest.schema.json');

  const expected = TERRAINS_TS[terrain];
  const [minX, minY, maxX, maxY] = manifest.worldBounds;
  const errors = [];

  if (manifest.terrainId !== terrain) errors.push(`terrainId mismatch`);
  if (minX !== 0 || minY !== 0 || maxX !== expected.width || maxY !== expected.height) {
    errors.push(`worldBounds !== [0,0,${expected.width},${expected.height}]`);
  }
  if (Math.abs(manifest.dem.heightRangeMinM - expected.heightRangeMinM) > 0.01) {
    errors.push(`dem.heightRangeMinM !== terrains.ts`);
  }
  if (Math.abs(manifest.dem.heightRangeMaxM - expected.heightRangeMaxM) > 0.01) {
    errors.push(`dem.heightRangeMaxM !== terrains.ts`);
  }
  if (manifest.precision?.storageDecimals !== 3) errors.push('storageDecimals must be 3');
  if (manifest.precision?.spawnAuthority !== 'mod-get-surface-y') {
    errors.push('spawnAuthority must be mod-get-surface-y');
  }

  if (manifest.dem.widthPx === 0 || manifest.dem.heightPx === 0) {
    console.warn('WARN  Stub manifest (widthPx/heightPx=0) — OK for T-090.0');
  } else if (!manifest.dem.exportedAt || !manifest.dem.workbenchVersion) {
    errors.push('exportedAt/workbenchVersion required when DEM dims set');
  }

  if (errors.length) {
    console.error('FAIL  terrains.ts cross-check:');
    for (const e of errors) console.error(`      ${e}`);
    process.exit(1);
  }
  console.log(`PASS  Manifest matches terrains.ts for ${terrain}`);
  console.log('\nverify-terrain-manifest: OK');
}

main();
