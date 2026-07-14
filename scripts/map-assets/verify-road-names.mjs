#!/usr/bin/env node
// T-152.9 — mathematical gates G3–G7 for road name labels @ committed road-names.json + roads.json.gz.
import { readFileSync, existsSync } from 'node:fs'
import { gunzipSync } from 'node:zlib'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { MAJOR_EVERON_ROADS } from './lib/road-names.mjs'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')
const terrain = process.env.TERRAIN ?? 'everon'
const zoomMatch = process.argv.find((a) => a.startsWith('--zoom'))
const deckZoom = zoomMatch
  ? Number(zoomMatch.includes('=') ? zoomMatch.split('=')[1] : process.argv[process.argv.indexOf(zoomMatch) + 1])
  : 0

const namesPath = join(repoRoot, 'packages', 'map-assets', terrain, 'road-names.json')
const roadsPath = join(repoRoot, 'packages', 'map-assets', terrain, 'objects', 'roads.json.gz')
const wasmPkg = join(repoRoot, 'apps', 'website', 'frontend', 'src', 'wasm', 'pkg', 'map_engine_wasm.js')

if (!existsSync(namesPath)) {
  console.error(`verify-road-names: missing ${namesPath}`)
  process.exit(1)
}
if (!existsSync(roadsPath)) {
  console.error(`verify-road-names: missing ${roadsPath}`)
  process.exit(1)
}
if (!existsSync(wasmPkg)) {
  console.error('verify-road-names: wasm pkg missing — run make wasm')
  process.exit(1)
}

const namesRaw = readFileSync(namesPath, 'utf8')
const roadsRaw = gunzipSync(readFileSync(roadsPath)).toString('utf8')
const wasm = await import(wasmPkg)

let failures = 0
const pass = (msg) => console.log(`  PASS  ${msg}`)
const fail = (msg) => {
  failures++
  console.log(`  FAIL  ${msg}`)
}

console.log(`verify-road-names (${terrain} @ z=${deckZoom}):`)

const drawn = JSON.parse(wasm.build_road_labels_json(namesRaw, roadsRaw, deckZoom))

const errJson = wasm.verify_road_labels_json(
  namesRaw,
  roadsRaw,
  JSON.stringify(drawn),
  deckZoom,
  JSON.stringify(MAJOR_EVERON_ROADS),
)
const errs = JSON.parse(errJson)
if (errs.length === 0) {
  pass(`G3 MAJOR_EVERON_ROADS (${MAJOR_EVERON_ROADS.length}) ⊆ drawn @ z=${deckZoom}`)
  pass('G4 name.length ≥ 2')
  pass('G5 placement ≤ 12 m perpendicular')
  pass('G6 declutter dist ≥ 60·2^(-z)')
  pass(`G7 |drawn|=${drawn.length} ≤ 24`)
} else {
  for (const e of errs) fail(e)
}

if (wasm.road_declutter_invariant_holds_json(JSON.stringify(drawn), deckZoom)) {
  pass('G6 wasm oracle (redundant check)')
} else {
  fail('G6 road_declutter_invariant_holds_json')
}

const emptyDrawn = JSON.parse(wasm.build_road_labels_json('{"roads":[]}', roadsRaw, deckZoom))
if (emptyDrawn.length === 0) {
  pass('toggle off oracle: empty names → |drawn|=0')
} else {
  fail(`empty names drew ${emptyDrawn.length}`)
}

const bytes = wasm.pack_road_label_bytes(JSON.stringify(drawn), deckZoom)
if (drawn.length === 0 && bytes.length === 0) {
  pass('pack bytes empty when no labels')
} else if (bytes.length > 0 && bytes.length % 20 === 0) {
  pass(`pack ${bytes.length / 20} glyph instances (${bytes.length} B)`)
} else {
  fail(`pack bytes invalid len=${bytes.length}`)
}

if (failures) {
  console.error(`\nverify-road-names: FAIL (${failures})`)
  process.exit(1)
}
console.log('\nverify-road-names: OK')
