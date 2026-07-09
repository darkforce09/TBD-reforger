// T-151.9 — wasm-only Class S residency golden (Deck chunkStore retired from this harness).
// Drives `wasm.WorldResidency` over the same 22-step Everon viewport script as T-151.3 W3,
// asserting missingIds / residentIds / pinned_building_count against a frozen golden JSON.
//
// Regenerate goldens:
//   T151_CAPTURE_RESIDENCY=1 npx vitest run src/features/_wasm/world.residency.parity.test.ts

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import * as wasm from '@/wasm/pkg/map_engine_wasm'

const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const EVERON = `${MAP_ASSETS}/everon`
const OBJECTS = `${EVERON}/objects`
const bytes = (path: string): Uint8Array => new Uint8Array(readFileSync(path))
const chunkPath = (id: string): string => `${OBJECTS}/chunks/${id}.json.gz`

const GOLDEN_PATH = resolve(
  process.cwd(),
  'src/features/_wasm/oracles/goldens/residency_everon_v1.json',
)
const CAPTURE = process.env.T151_CAPTURE_RESIDENCY === '1'

type Bbox = [number, number, number, number]

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

interface GoldenStep {
  i: number
  missingIds: string[]
  residentIds: string[]
  pinnedBuildingCount: number
}

interface GoldenFile {
  baseline: string
  steps: GoldenStep[]
}

const asc = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0)

function loadManifestInto(residency: InstanceType<typeof wasm.WorldResidency>): void {
  residency.load_manifest_json(readFileSync(`${EVERON}/manifest.json`, 'utf8'))
  residency.load_prefabs_gz(bytes(`${OBJECTS}/prefabs.json.gz`))
  residency.load_chunk_index_json(readFileSync(`${OBJECTS}/chunks/manifest.json`, 'utf8'))
}

describe('world.residency.parity — wasm-only Class S (T-151.9)', () => {
  it('22-step script matches residency_everon_v1 golden', () => {
    expect(SCRIPT.length).toBe(22)

    const residency = new wasm.WorldResidency()
    loadManifestInto(residency)

    const captured: GoldenStep[] = []
    let golden: GoldenFile | null = null
    if (!CAPTURE) {
      golden = JSON.parse(readFileSync(GOLDEN_PATH, 'utf8')) as GoldenFile
      expect(golden.baseline).toBe('ec59d10e')
      expect(golden.steps.length).toBe(22)
    }

    for (let i = 0; i < SCRIPT.length; i++) {
      const { bbox, zoom } = SCRIPT[i]
      const missingIds = Array.from(residency.set_viewport(bbox[0], bbox[1], bbox[2], bbox[3], zoom))
      for (const id of missingIds) residency.ingest_chunk_gz(id, bytes(chunkPath(id)))
      residency.end_apply_frame(0)

      const residentIds = [...residency.resident_chunk_ids()].sort(asc)
      const pinnedBuildingCount = residency.pinned_building_count
      const step: GoldenStep = { i, missingIds, residentIds, pinnedBuildingCount }
      captured.push(step)

      if (golden) {
        const g = golden.steps[i]
        expect(g.i).toBe(i)
        expect(missingIds, `step ${i}: missingIds`).toEqual(g.missingIds)
        expect(residentIds, `step ${i}: residentIds`).toEqual(g.residentIds)
        expect(pinnedBuildingCount, `step ${i}: pinnedBuildingCount`).toBe(g.pinnedBuildingCount)
      }
    }

    if (CAPTURE) {
      mkdirSync(dirname(GOLDEN_PATH), { recursive: true })
      const out: GoldenFile = { baseline: 'ec59d10e', steps: captured }
      writeFileSync(GOLDEN_PATH, `${JSON.stringify(out, null, 2)}\n`)
      expect(captured.length).toBe(22)
    }

    expect(residency.eviction_log().length, 'eviction occurred').toBeGreaterThan(0)
  }, 120_000)

  it('in-flight chunks are not re-requested by an overlapping viewport (inflight dedup)', () => {
    const residency = new wasm.WorldResidency()
    loadManifestInto(residency)

    // set_viewport A marks its chunks in-flight (not yet ingested); an overlapping B must exclude
    // them (mirror of chunkStore.test.ts's in-flight-dedup case).
    const a = Array.from(residency.set_viewport(6000, 6000, 6400, 6400, -2))
    expect(a.length).toBeGreaterThan(0)
    const b = Array.from(residency.set_viewport(6300, 6000, 6800, 6400, -2))
    expect(b.length, 'B requests genuinely-new chunks').toBeGreaterThan(0)
    expect(b.every((id) => !a.includes(id)), 'B excludes A in-flight ids').toBe(true)
  })
})
