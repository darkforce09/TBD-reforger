// T-151.3 W3 (P7/P8, Class S) — the Rust `WorldSpatialIndex` returns the same id set as the JS
// rbush `worldSpatialIndex` over a fixed multi-chunk Everon fixture, across ≥10k random probes with
// class masks. Both sides get IDENTICAL f32-quantized coords (the chunk `positions` f32 values), so
// `dx²+dy²` is bit-identical f64.
//
// Determinism (no probabilistic argument): `pick_rect` is a SET (tie-immune). For `pick_nearest`
// each probe ASSERTS the brute-force minimum squared distance among in-radius class-matched
// candidates is UNIQUE (strict `<` the runner-up) before comparing ids — a fixed seed that passes
// proves no exact-distance tie occurred, so the first-encountered iteration-order difference between
// the grid and the rbush cannot matter. A tie would fail loudly (bump the seed).

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { gunzipSync } from 'node:zlib'

import { describe, expect, it } from 'vitest'

import {
  buildPrefabMaps,
  narrowPrefabRows,
  parseChunkOracle,
  RENDER_CLASS_CODES,
} from '@/features/tactical-map/workers/worldObjectsCore'
import { createWorldSpatialIndex } from '@/features/tactical-map/state/worldSpatialIndex'
import * as wasm from '@/wasm/pkg/map_engine_wasm'

const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const OBJECTS = `${MAP_ASSETS}/everon/objects`
const readGz = (path: string): unknown => JSON.parse(gunzipSync(readFileSync(path)).toString('utf8'))

const NO_CLASS = 255
const prefabById = buildPrefabMaps(narrowPrefabRows(readGz(`${OBJECTS}/prefabs.json.gz`))).byId

// Fixed multi-chunk fixture — a spread of real chunks with buildings + trees (class diversity for
// the mask filter). `12_11` etc. are populated Everon town/forest cells.
const FIXTURE_CHUNKS = ['12_11', '12_12', '13_11', '13_12', '2_10', '2_11']

const rustIdx = new wasm.WorldSpatialIndex()
const oracle = createWorldSpatialIndex()
const entryById = new Map<string, { x: number; y: number; cls: string }>()
let buildingCount = 0

for (const id of FIXTURE_CHUNKS) {
  const parsed = parseChunkOracle(id, readGz(`${OBJECTS}/chunks/${id}.json.gz`), prefabById)
  if (!parsed) continue
  const n = parsed.count
  const xs = new Float32Array(n)
  const ys = new Float32Array(n)
  for (let i = 0; i < n; i++) {
    xs[i] = parsed.positions[2 * i]
    ys[i] = parsed.positions[2 * i + 1]
  }
  // Rust: raw SoA columns (skips NO_CLASS internally).
  rustIdx.insert_chunk(id, xs, ys, parsed.clsCodes.subarray(0, n))
  // rbush oracle: pre-filtered entries (matches worldObjectsCore.indexChunk id form + skip).
  const entries: { id: string; x: number; y: number; cls: string }[] = []
  for (let i = 0; i < n; i++) {
    const code = parsed.clsCodes[i]
    if (code === NO_CLASS) continue
    const e = { id: `${id}:${i}`, x: xs[i], y: ys[i], cls: RENDER_CLASS_CODES[code] }
    entries.push(e)
    entryById.set(e.id, { x: e.x, y: e.y, cls: e.cls })
    if (code === 0) buildingCount++
  }
  oracle.insertChunk(id, entries)
}

const asc = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0)

/** The 4 mask/filter pairs the probe battery cycles through. */
const MASKS: { mask: number | undefined; filter: ((cls: string) => boolean) | undefined; label: string }[] = [
  { mask: undefined, filter: undefined, label: 'all' },
  { mask: 1 << 0, filter: (c) => c === 'building', label: 'building' },
  { mask: 1 << 1, filter: (c) => c === 'tree', label: 'tree' },
  { mask: (1 << 0) | (1 << 1), filter: (c) => c === 'building' || c === 'tree', label: 'building|tree' },
]

describe('world.pick.parity — Rust WorldSpatialIndex vs rbush worldSpatialIndex (Class S)', () => {
  it('fixture loaded with class diversity (both sides equal size)', () => {
    expect(rustIdx.size).toBe(oracle.size())
    expect(rustIdx.size).toBeGreaterThan(1000) // dense trees
    expect(buildingCount).toBeGreaterThan(0) // buildings present for the building-mask probes
  })

  it('pick_rect + pick_nearest set/id equality over 10k probes (checked tie-free)', () => {
    const PROBES = 10_000
    let s = 0x151_00003 >>> 0
    const rnd = (): number => {
      s = (Math.imul(s, 1103515245) + 12345) >>> 0
      return s / 0x100000000
    }
    let ties = 0
    for (let p = 0; p < PROBES; p++) {
      const x = rnd() * 12800
      const y = rnd() * 12800
      const r = 1 + rnd() * 300
      const { mask, filter } = MASKS[p % MASKS.length]

      // pick_rect — sorted id-set equality (tie-immune).
      const rustRect = Array.from(rustIdx.pick_rect(x - r, y - r, x + r, y + r, mask)).sort(asc)
      const oracleRect = oracle.pickRect([x - r, y - r, x + r, y + r], filter).sort(asc)
      expect(rustRect, `pick_rect probe ${p}`).toEqual(oracleRect)

      // pick_nearest — verify the min is unique among in-radius class-matched candidates.
      const r2 = r * r
      let min = Infinity
      let minCount = 0
      for (const id of oracle.pickRect([x - r, y - r, x + r, y + r], filter)) {
        const e = entryById.get(id)
        if (!e) continue
        const dx = e.x - x
        const dy = e.y - y
        const d2 = dx * dx + dy * dy
        if (d2 > r2) continue
        if (d2 < min) {
          min = d2
          minCount = 1
        } else if (d2 === min) {
          minCount++
        }
      }
      if (minCount >= 2) {
        ties++
        continue // exact-distance tie — id choice is iteration-order-dependent; skip (see below)
      }
      const rustId = rustIdx.pick_nearest(x, y, r, mask) ?? null
      const oracleId = oracle.pickNearest(x, y, r, filter)
      expect(rustId, `pick_nearest probe ${p}`).toBe(oracleId)
    }
    // No exact-distance ties for this fixed seed → every pick_nearest was compared, none skipped.
    expect(ties, 'exact-distance ties (bump the seed if > 0)').toBe(0)
  })
})
