// T-151.3 W3 (P2/P3/P6, Class S) — the Rust `WorldResidency` and the shipped Deck `createChunkStore`
// evolve identically. Both are driven through ONE viewport script over the real Everon export (Rust
// ingests real `.json.gz` bytes; the JS fake client serves the same chunks parsed via
// `parseChunkOracle`), with an identical fake clock.
//
// Completeness theorem (P3): with identical deliveries, `missing = pinned − cache − inflight` is a
// pure function of the cache, which evolves only by applies (identical order both sides) and
// evictions (under test). So if the REQUESTED-ID SEQUENCE matches at every step over a script that
// crosses the LRU cap (step 0 pins all 275 chunks; cap then drops to 64) AND revisits evicted
// regions, eviction matched too. Eviction ORDER itself is separately pinned by the native
// `residency.rs` ascending-`last_used` test; here we prove the observable consequence.
//   P2: Rust set_viewport missing-ids === the ids the chunkStore fake client recorded (ordered).
//   P6: Rust pinned_building_count === chunkStore getWorldBuildings().length (u16 lookup parity).

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { gunzipSync } from 'node:zlib'

import { describe, expect, it } from 'vitest'

import { createChunkStore, type WorldStreamClient } from '@/features/tactical-map/worldmap/chunkStore'
import { type Bbox } from '@/features/tactical-map/worldmap/chunkMath'
import { TERRAINS } from '@/features/tactical-map/coords/terrains'
import {
  buildPrefabMaps,
  narrowPrefabRows,
  parseChunkOracle,
  type ChunkClassGroup,
  type ChunkLoadResult,
  type ChunkPayload,
  type LoadChunksOpts,
  type ParsedChunk,
  type WorldChunkCell,
  type WorldManifestLite,
} from '@/features/tactical-map/workers/worldObjectsCore'
import * as wasm from '@/wasm/pkg/map_engine_wasm'

const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const EVERON = `${MAP_ASSETS}/everon`
const OBJECTS = `${EVERON}/objects`
const readGz = (path: string): unknown => JSON.parse(gunzipSync(readFileSync(path)).toString('utf8'))
const bytes = (path: string): Uint8Array => new Uint8Array(readFileSync(path))
const chunkPath = (id: string): string => `${OBJECTS}/chunks/${id}.json.gz`

const EV = TERRAINS.everon
const manifestJson = readFileSync(`${EVERON}/manifest.json`, 'utf8')
const prefabsRaw = readGz(`${OBJECTS}/prefabs.json.gz`)
const prefabRows = narrowPrefabRows(prefabsRaw)
const { byId: prefabById, hasOversized } = buildPrefabMaps(prefabRows)
const indexRaw = JSON.parse(readFileSync(`${OBJECTS}/chunks/manifest.json`, 'utf8')) as {
  cells: { cx: number; cy: number; path: string; instanceCount?: number }[]
}
const cells: WorldChunkCell[] = indexRaw.cells.map((c) => ({
  id: `${c.cx}_${c.cy}`,
  cx: c.cx,
  cy: c.cy,
  path: c.path,
  instanceCount: c.instanceCount,
}))

/** Slice the building-class rows of a parsed chunk into a ChunkClassGroup (mirror of the worker's
 *  private `sliceGroup`). */
function sliceBuilding(chunk: ParsedChunk): ChunkClassGroup | undefined {
  const rows = chunk.rowsByClass.building
  if (!rows || rows.length === 0) return undefined
  const n = rows.length
  const positions = new Float32Array(2 * n)
  const prefabIdx = new Uint16Array(n)
  const rotations = new Float32Array(n)
  const z = new Float32Array(n)
  for (let k = 0; k < n; k++) {
    const i = rows[k]
    positions[2 * k] = chunk.positions[2 * i]
    positions[2 * k + 1] = chunk.positions[2 * i + 1]
    prefabIdx[k] = chunk.prefabIdx[i]
    rotations[k] = chunk.rotations[i]
    z[k] = chunk.z[i]
  }
  return { count: n, positions, prefabIdx, rotations, z }
}

const payloadCache = new Map<string, ChunkPayload | null>()
function payloadFor(id: string): ChunkPayload | null {
  const cached = payloadCache.get(id)
  if (cached !== undefined) return cached
  const parsed = parseChunkOracle(id, readGz(chunkPath(id)), prefabById)
  let payload: ChunkPayload | null = null
  if (parsed) {
    const building = sliceBuilding(parsed)
    payload = {
      id,
      cx: parsed.cx,
      cy: parsed.cy,
      totalInstances: parsed.count,
      groups: building ? { building } : {},
    }
  }
  payloadCache.set(id, payload)
  return payload
}

interface Harness {
  store: ReturnType<typeof createChunkStore>
  requestedIds: string[][]
  flush: () => Promise<void>
  pump: () => void
}

function makeHarness(): Harness {
  const requestedIds: string[][] = []
  const scheduled: (() => void)[] = []
  const manifest: WorldManifestLite = {
    terrainId: 'everon',
    chunkSizeM: 512,
    cells,
    prefabRows,
    roadsPath: null,
    densityPath: null,
    instanceCount: null,
    hasOversized,
  }
  const client: WorldStreamClient = {
    async loadManifest(): Promise<WorldManifestLite> {
      return manifest
    },
    async loadChunksInBbox(_bbox: Bbox, _m: number, opts: LoadChunksOpts): Promise<ChunkLoadResult> {
      const ids = opts.ids ?? []
      requestedIds.push([...ids])
      const chunks: ChunkPayload[] = []
      for (const id of ids) {
        const p = payloadFor(id)
        if (p) chunks.push(p)
      }
      return { chunkSizeM: 512, chunks }
    },
    unload(): Promise<void> {
      return Promise.resolve()
    },
  }
  // Fast clock (0 ms/call) → the whole delivered queue applies in one frame (matches the wgpu
  // driveStep, which ingests all missing synchronously per step).
  const store = createChunkStore({ client, now: () => 0, schedule: (cb) => scheduled.push(cb) })
  return {
    store,
    requestedIds,
    flush: async () => {
      await Promise.resolve()
      await Promise.resolve()
      await Promise.resolve()
    },
    pump: () => {
      for (let i = 0; i < 100 && scheduled.length > 0; i++) (scheduled.shift() as () => void)()
    },
  }
}

// ── Viewport script (≥20 steps): full-island (pin all 275, > cap 64) → small stops (evict) →
// unchanged → gate-closed → reopen → revisits of evicted regions. ────────────────────────────────
const FULL: Bbox = [0, 0, 12800, 12800]
function small(cx: number, cy: number): Bbox {
  const x = cx * 512 + 256
  const y = cy * 512 + 256
  return [x - 150, y - 150, x + 150, y + 150]
}
interface Step {
  bbox: Bbox
  zoom: number
}
const SCRIPT: Step[] = [
  { bbox: FULL, zoom: -2 }, // 0: pin all 275
  // 1..12: small stops marching across the island (each cap=64 → evicts the aged full-island set)
  { bbox: small(12, 11), zoom: -2 },
  { bbox: small(13, 12), zoom: -2 },
  { bbox: small(3, 20), zoom: -2 },
  { bbox: small(20, 5), zoom: -2 },
  { bbox: small(7, 14), zoom: -2 },
  { bbox: small(18, 18), zoom: -2 },
  { bbox: small(2, 9), zoom: -2 },
  { bbox: small(15, 3), zoom: -2 },
  { bbox: small(9, 22), zoom: -2 },
  { bbox: small(22, 12), zoom: -2 },
  { bbox: small(5, 5), zoom: -2 },
  { bbox: small(11, 8), zoom: -2 },
  { bbox: small(11, 8), zoom: -2 }, // 13: unchanged (same rect) → no request
  { bbox: small(11, 8), zoom: -3 }, // 14: gate closed → release pins, no request
  { bbox: small(11, 8), zoom: -2 }, // 15: reopen (freshly pinned last → cached)
  // 16..21: revisit earlier regions — many were evicted → re-request (P2 verifies both agree)
  { bbox: small(12, 11), zoom: -2 },
  { bbox: small(3, 20), zoom: -2 },
  { bbox: small(20, 5), zoom: -2 },
  { bbox: FULL, zoom: -2 }, // 19: re-pin the whole island (re-requests every evicted chunk)
  { bbox: small(7, 14), zoom: -2 },
  { bbox: small(2, 9), zoom: -2 },
]

describe('world.residency.parity — Rust WorldResidency vs Deck chunkStore (T-151.3 W3, Class S)', () => {
  it('requested-id sequence + building counts match at every step; eviction exercised', async () => {
    const residency = new wasm.WorldResidency()
    residency.load_manifest_json(manifestJson)
    residency.load_prefabs_gz(bytes(`${OBJECTS}/prefabs.json.gz`))
    residency.load_chunk_index_json(readFileSync(`${OBJECTS}/chunks/manifest.json`, 'utf8'))

    const h = makeHarness()
    h.store.ensureWorldStream(EV)
    await h.flush()

    let postSweepRequests = 0
    for (let step = 0; step < SCRIPT.length; step++) {
      const { bbox, zoom } = SCRIPT[step]
      // Rust: set_viewport → ingest every missing chunk synchronously → end the apply frame.
      const rustMissing = Array.from(residency.set_viewport(bbox[0], bbox[1], bbox[2], bbox[3], zoom))
      for (const id of rustMissing) residency.ingest_chunk_gz(id, bytes(chunkPath(id)))
      if (rustMissing.length > 0) residency.end_apply_frame(0)

      // JS chunkStore: same viewport, then flush+pump so the delivered queue fully applies.
      const before = h.requestedIds.length
      h.store.setWorldViewport(bbox, zoom)
      await h.flush()
      h.pump()
      const after = h.requestedIds.length

      // P2 — a request happened iff Rust had missing ids, and the id lists are identical (ordered).
      if (rustMissing.length > 0) {
        expect(after, `step ${step}: one request`).toBe(before + 1)
        expect(h.requestedIds[after - 1], `step ${step}: requested ids`).toEqual(rustMissing)
        if (step > 0) postSweepRequests++
      } else {
        expect(after, `step ${step}: no request`).toBe(before)
      }

      // P6 — pinned building count matches the composite length (u16 building-row selection parity).
      expect(residency.pinned_building_count, `step ${step}: building count`).toBe(
        h.store.getWorldBuildings().length,
      )
    }

    // Eviction was exercised: step 0 pinned all 275 (> cap 64), later small stops evicted, and
    // revisits re-requested evicted chunks (both sides agreed via P2 above).
    expect(residency.eviction_log().length, 'eviction occurred').toBeGreaterThan(0)
    expect(postSweepRequests, 're-request after eviction').toBeGreaterThan(0)
  }, 120_000)

  it('in-flight chunks are not re-requested by an overlapping viewport (inflight dedup)', () => {
    const residency = new wasm.WorldResidency()
    residency.load_manifest_json(manifestJson)
    residency.load_prefabs_gz(bytes(`${OBJECTS}/prefabs.json.gz`))
    residency.load_chunk_index_json(readFileSync(`${OBJECTS}/chunks/manifest.json`, 'utf8'))

    // set_viewport A marks its chunks in-flight (not yet ingested); an overlapping B must exclude
    // them (mirror of chunkStore.test.ts's in-flight-dedup case).
    const a = Array.from(residency.set_viewport(6000, 6000, 6400, 6400, -2))
    expect(a.length).toBeGreaterThan(0)
    const b = Array.from(residency.set_viewport(6300, 6000, 6800, 6400, -2))
    expect(b.length, 'B requests genuinely-new chunks').toBeGreaterThan(0)
    expect(b.every((id) => !a.includes(id)), 'B excludes A in-flight ids').toBe(true)
  })
})
