import { describe, it, expect, beforeAll, afterEach, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { PNG } from 'pngjs'
import { rasterFromPngjs, type DemRaster } from './DemTexture'
import { sampleElevationMeters } from './sampleElevation'
import type { TerrainManifest } from './terrainManifest'
import {
  loadDemForTerrain,
  isDemReady,
  isDemDegraded,
  sampleElevation,
  subscribeDem,
  getDemVersion,
  _resetForTest,
} from './DemController'

// Toast is mocked so degraded-mode tests can assert the Retry action without a DOM.
vi.mock('sonner', () => ({ toast: { error: vi.fn() } }))
import { toast } from 'sonner'

// vitest runs from apps/website/frontend, so 3× up reaches the repo root (matches the
// vitest.config.ts `map-assets` alias).
const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const EVERON_MANIFEST = resolve(MAP_ASSETS, 'everon/manifest.json')
const EVERON_PNG = resolve(MAP_ASSETS, 'everon/dem/everon-dem-16bit.png')
const ARLAND_MANIFEST = resolve(MAP_ASSETS, 'arland/manifest.json')

function jsonResponse(path: string): Response {
  const text = readFileSync(path, 'utf8')
  return {
    ok: true,
    status: 200,
    json: async () => JSON.parse(text),
  } as unknown as Response
}

function bufferResponse(path: string): Response {
  const buf = readFileSync(path)
  return {
    ok: true,
    status: 200,
    arrayBuffer: async () => buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength),
  } as unknown as Response
}

// ── Pure-math anchors against the real committed PNG (T-091.0 @ 6d96339) ──────────────
describe('sampleElevationMeters — Everon anchors (±0.01 m)', () => {
  let manifest: TerrainManifest
  let dem: DemRaster

  beforeAll(() => {
    manifest = JSON.parse(readFileSync(EVERON_MANIFEST, 'utf8'))
    const png = PNG.sync.read(readFileSync(EVERON_PNG), { skipRescale: true })
    dem = rasterFromPngjs(png)
  })

  // [id, x, z(=editor y), expected demY meters]
  const anchors: Array<[string, number, number, number]> = [
    ['bridgehead-sl', 4839.2, 6620.8, 121.784],
    ['bridgehead-tl0', 4836.9, 6626.5, 123.328],
    ['bridgehead-tl1', 4831.2, 6628.8, 123.602],
    ['coast-w', 1000, 6400, 0.054],
    ['valley-inland', 5000, 5000, 80.871],
    ['hill-north', 9600, 3200, 221.652],
    ['peak-central', 6400, 6400, 157.882],
    ['coast-sw', 2000, 2000, -7.408],
    ['seabed-e', 11000, 6400, -84.86],
    ['shelf-ne', 8000, 8000, -18.314],
    ['mid-s', 3200, 9600, -47.743],
  ]

  for (const [id, x, z, expected] of anchors) {
    it(`${id} (${x}, ${z}) → ${expected} m`, () => {
      const got = sampleElevationMeters(x, z, manifest, dem.raster, dem.width, dem.height)
      expect(Math.abs(got - expected)).toBeLessThanOrEqual(0.01)
    })
  }
})

// ── Synthetic 2×2 — bilinear center vs hand calc ──────────────────────────────────────
describe('sampleElevationMeters — synthetic 2×2', () => {
  it('center bilinear = mean of corners', () => {
    // min 0 / max 65535 → uint16ToMeters is identity, so meters == raster values.
    const manifest = {
      terrainId: 'synthetic',
      schemaVersion: 1,
      worldBounds: [0, 0, 1, 1],
      metersPerPixel: 1,
      dem: {
        path: 'x.png',
        widthPx: 2,
        heightPx: 2,
        encoding: 'uint16-linear',
        heightRangeMinM: 0,
        heightRangeMaxM: 65535,
        source: 'test',
        axisFlip: { x: false, z: false },
      },
      precision: { storageDecimals: 3 },
    } as TerrainManifest
    const raster = new Float64Array([0, 100, 200, 300]) // row-major 2×2
    // center (0.5, 0.5) → px=0.5, py=0.5 → (0+100+200+300)/4 = 150
    expect(sampleElevationMeters(0.5, 0.5, manifest, raster, 2, 2)).toBeCloseTo(150, 6)
    // corner (0,0) → px=0,py=0 → 0
    expect(sampleElevationMeters(0, 0, manifest, raster, 2, 2)).toBeCloseTo(0, 6)
  })
})

// ── DemController lifecycle ───────────────────────────────────────────────────────────
describe('DemController', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    vi.clearAllMocks()
    _resetForTest()
  })

  it('S10: sampleElevation returns 0 before load (not NaN)', () => {
    _resetForTest()
    const v = sampleElevation(5000, 5000)
    expect(v).toBe(0)
    expect(Number.isNaN(v)).toBe(false)
  })

  it('S8: Arland stub → degraded, no PNG fetch, toast with Retry', async () => {
    _resetForTest()
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
      const url = String(input)
      if (url.endsWith('arland/manifest.json')) return jsonResponse(ARLAND_MANIFEST)
      throw new Error(`unexpected fetch ${url}`)
    })
    await loadDemForTerrain('arland')
    expect(isDemDegraded()).toBe(true)
    expect(isDemReady()).toBe(false)
    expect(fetchSpy).toHaveBeenCalledTimes(1) // manifest only — no PNG
    const errMock = vi.mocked(toast.error)
    expect(errMock).toHaveBeenCalledTimes(1)
    expect(errMock.mock.calls[0][1]?.action).toMatchObject({ label: 'Retry' })
    expect(sampleElevation(5000, 5000)).toBe(0)
  })

  it('S9: Everon load → ready + four-edge clamp', async () => {
    _resetForTest()
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
      const url = String(input)
      if (url.endsWith('everon/manifest.json')) return jsonResponse(EVERON_MANIFEST)
      if (url.endsWith('everon-dem-16bit.png')) return bufferResponse(EVERON_PNG)
      throw new Error(`unexpected fetch ${url}`)
    })
    await loadDemForTerrain('everon')
    expect(isDemReady()).toBe(true)
    expect(isDemDegraded()).toBe(false)

    // Clamp to terrain bounds [0,12800]² before sampling — OOB never throws.
    expect(sampleElevation(-100, 5000)).toBe(sampleElevation(0, 5000))
    expect(sampleElevation(12900, 5000)).toBe(sampleElevation(12800, 5000))
    expect(sampleElevation(5000, -100)).toBe(sampleElevation(5000, 0))
    expect(sampleElevation(5000, 12900)).toBe(sampleElevation(5000, 12800))

    // In-bounds anchor still resolves through the runtime cache (rounded to 3 dp).
    const peak = sampleElevation(6400, 6400)
    expect(Math.abs(peak - 157.882)).toBeLessThanOrEqual(0.01)
  })

  it('subscribeDem notifies + getDemVersion bumps across a load', async () => {
    _resetForTest()
    const cb = vi.fn()
    const unsub = subscribeDem(cb)
    const v0 = getDemVersion()
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
      const url = String(input)
      if (url.endsWith('everon/manifest.json')) return jsonResponse(EVERON_MANIFEST)
      if (url.endsWith('everon-dem-16bit.png')) return bufferResponse(EVERON_PNG)
      throw new Error(`unexpected fetch ${url}`)
    })
    await loadDemForTerrain('everon') // loading + ready → ≥2 notifies
    expect(cb).toHaveBeenCalled()
    expect(getDemVersion()).toBeGreaterThan(v0)
    expect(isDemReady()).toBe(true)

    // Unsubscribe stops further calls.
    unsub()
    const calls = cb.mock.calls.length
    _resetForTest() // would notify if still subscribed
    expect(cb.mock.calls.length).toBe(calls)
  })
})
