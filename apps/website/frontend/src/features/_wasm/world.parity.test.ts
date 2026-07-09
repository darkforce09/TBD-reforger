// T-151.2 (W2) — differential parity: the Rust `WorldStore` world parser vs the JS
// `worldObjectsCore` oracle, over the real Everon object export.
//   - Class R: chunk SoA columns byte-identical (`f32BytesEqual` / integer array equality).
//   - Class S: per-render-class `rowsByClass` index sets equal.
//   - Class T: OBB corners + road centerline within 1 ULP of the TS.
// Census totals are asserted against the pinned inventory (391 / 508 291 / 275 / 888 / 36 / 625).
//
// The JS master arrays are length `instances.length` but only `[0, count)` is valid, so the
// oracle columns are sliced to `count` before comparison; the wasm columns are already truncated.
import { readFileSync, readdirSync } from 'node:fs'
import { resolve } from 'node:path'
import { gunzipSync } from 'node:zlib'

import { describe, expect, it } from 'vitest'

import {
  buildPrefabMaps,
  extractRoadCenterline,
  narrowPrefabRows,
  obbCorners,
  parseChunkOracle,
  RENDER_CLASS_CODES,
} from '@/features/_wasm/oracles/jsWorldChunkOracle'
import * as wasm from '@/wasm/pkg/map_engine_wasm'

import { f32BytesEqual, intArrayEqual, ulpDistanceF64 } from './parity'

// cwd = apps/website/frontend (same as worldObjectsCore.test.ts). 3× up → repo root.
const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const EVERON = `${MAP_ASSETS}/everon`
const OBJECTS = `${EVERON}/objects`

const readGz = (path: string): unknown => JSON.parse(gunzipSync(readFileSync(path)).toString('utf8'))
const bytes = (path: string): Uint8Array => new Uint8Array(readFileSync(path))

// Shared setup (runs once at import): load the manifest + prefab table into both sides.
const manifestJson = readFileSync(`${EVERON}/manifest.json`, 'utf8')
const prefabById = buildPrefabMaps(narrowPrefabRows(readGz(`${OBJECTS}/prefabs.json.gz`))).byId

const store = new wasm.WorldStore()
store.load_manifest_json(manifestJson)
const wasmPrefabCount = store.load_prefabs_gz(bytes(`${OBJECTS}/prefabs.json.gz`))

const chunkFiles = readdirSync(`${OBJECTS}/chunks`)
  .filter((f) => f.endsWith('.json.gz'))
  .sort()

describe('world.parity — Rust WorldStore vs JS worldObjectsCore oracle (T-151.2 W2)', () => {
  it('prefab table: 391 (wasm == JS oracle == manifest)', () => {
    expect(wasmPrefabCount).toBe(391)
    expect(prefabById.size).toBe(391)
  })

  it('chunk file census: 275', () => {
    expect(chunkFiles.length).toBe(275)
  })

  it('all 275 chunks byte-exact — Class R columns + Class S row sets; ΣInstances = 508 291', () => {
    let totalInstances = 0
    for (const file of chunkFiles) {
      const id = file.replace('.json.gz', '')
      const oracle = parseChunkOracle(id, readGz(`${OBJECTS}/chunks/${file}`), prefabById)
      const count = store.parse_chunk_gz(id, bytes(`${OBJECTS}/chunks/${file}`))

      if (!oracle) {
        expect(count).toBe(0)
        continue
      }
      expect(count).toBe(oracle.count)
      totalInstances += oracle.count
      const n = oracle.count

      // Class R — raw f32 bytes (positions/rotations/z) + integer columns (prefabIdx/clsCodes).
      expect(f32BytesEqual(store.chunk_positions(), oracle.positions.subarray(0, 2 * n)), `${id} pos`).toBe(true)
      expect(f32BytesEqual(store.chunk_rotations(), oracle.rotations.subarray(0, n)), `${id} rot`).toBe(true)
      expect(f32BytesEqual(store.chunk_z(), oracle.z.subarray(0, n)), `${id} z`).toBe(true)
      expect(intArrayEqual(store.chunk_prefab_idx(), oracle.prefabIdx.subarray(0, n)), `${id} pid`).toBe(true)
      expect(intArrayEqual(store.chunk_cls_codes(), oracle.clsCodes.subarray(0, n)), `${id} cls`).toBe(true)

      // Class S — per-render-class row-index sets.
      for (let code = 0; code < RENDER_CLASS_CODES.length; code++) {
        const oracleRows = oracle.rowsByClass[RENDER_CLASS_CODES[code]] ?? new Uint32Array(0)
        expect(intArrayEqual(store.chunk_rows_for_class(code), oracleRows), `${id} rows[${code}]`).toBe(true)
      }
    }
    expect(totalInstances).toBe(508291)
  }, 180_000)

  it('roads: 888 centerlined segments', () => {
    expect(store.load_roads_gz(bytes(`${OBJECTS}/roads.json.gz`))).toBe(888)
  })

  it('forest regions: 36', () => {
    expect(store.load_forest_regions_gz(bytes(`${OBJECTS}/forest-regions.json.gz`))).toBe(36)
  })

  it('density grids: 625 TBDD files (decode smoke on 3)', () => {
    const bins = readdirSync(`${OBJECTS}/density`)
      .filter((f) => f.endsWith('.bin'))
      .sort()
    expect(bins.length).toBe(625)
    for (const f of bins.slice(0, 3)) {
      const grid = wasm.decode_tbdd(bytes(`${OBJECTS}/density/${f}`))
      expect(grid.cols).toBeGreaterThan(0)
      expect(grid.rows).toBeGreaterThan(0)
    }
  })

  it('stats() reflects the loaded totals (after the sweep + roads + regions)', () => {
    const s = JSON.parse(store.stats()) as {
      prefab_count: number
      instance_count_total: number
      chunk_count_loaded: number
      road_segment_count: number
      forest_region_count: number
      has_oversized: boolean
    }
    expect(s.prefab_count).toBe(391)
    expect(s.instance_count_total).toBe(508291)
    expect(s.chunk_count_loaded).toBe(275)
    expect(s.road_segment_count).toBe(888)
    expect(s.forest_region_count).toBe(36)
    expect(typeof s.has_oversized).toBe('boolean')
  })
})

describe('world.parity — Class T (≤ 1 ULP vs TS)', () => {
  it('obb_corners matches obbCorners on the pinned cases', () => {
    const cases: [number, number, number, number, number][] = [
      [100, 200, 5, 3, 0],
      [100, 200, 5, 3, 90],
      [0, 0, 4, 2, 37],
      [0, 0, 4, 2, 360],
      [5120.04, 5518.09, 8.5, 4.25, 94.55],
    ]
    for (const [x, y, hx, hy, rot] of cases) {
      const w = wasm.obb_corners(x, y, hx, hy, rot)
      const ts = obbCorners(x, y, hx, hy, rot).flatMap((c) => c)
      expect(w.length).toBe(ts.length)
      for (let i = 0; i < ts.length; i++) {
        expect(ulpDistanceF64(w[i], ts[i]), `case ${rot}° idx ${i}`).toBeLessThanOrEqual(1)
      }
    }
  })

  it('road_centerline matches extractRoadCenterline (path vertices + width) within 1 ULP', () => {
    const quadSoup: [number, number][] = [
      [-2, 0],
      [2, 0],
      [2, 10],
      [-2, 10],
      [-2, 10],
      [2, 10],
      [2, 20],
      [-2, 20],
    ]
    const flare: [number, number][] = [
      [-2, 0],
      [2, 0],
      [2, 10],
      [-2, 10],
      [-6, 20],
      [6, 20],
    ]
    for (const pts of [quadSoup, flare]) {
      const ts = extractRoadCenterline(pts)
      if (!ts) throw new Error('extractRoadCenterline unexpectedly null')
      const w = wasm.road_centerline(new Float64Array(pts.flatMap((p) => p)))
      // wasm layout: [width, x0, y0, x1, y1, …]
      expect(w.length).toBe(1 + ts.path.length * 2)
      expect(ulpDistanceF64(w[0], ts.widthM)).toBeLessThanOrEqual(1)
      for (let k = 0; k < ts.path.length; k++) {
        expect(ulpDistanceF64(w[1 + 2 * k], ts.path[k][0]), `vx ${k}`).toBeLessThanOrEqual(1)
        expect(ulpDistanceF64(w[2 + 2 * k], ts.path[k][1]), `vy ${k}`).toBeLessThanOrEqual(1)
      }
    }
  })
})
