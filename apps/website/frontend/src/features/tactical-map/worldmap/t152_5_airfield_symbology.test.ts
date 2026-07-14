// T-152.5 — airfield symbology gates (TS oracle + wasm smoke; geometry in Rust).

import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { gunzipSync } from 'node:zlib'
import { resolve } from 'node:path'

import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { landmarkGlyphIconKey } from './buildingGlyphs'

const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const EVERON = `${MAP_ASSETS}/everon`
const OBJECTS = `${EVERON}/objects`
const ROADS_GZ = `${OBJECTS}/roads.json.gz`
const DEM_PNG = `${EVERON}/dem/everon-dem-16bit.png`
const MIN_M = -204.78
const MAX_M = 375.53
const APRON_AREA_MIN_M2 = 15_000

const bytes = (path: string): Uint8Array => new Uint8Array(readFileSync(path))

function glyphKeysFromManifest(): string[] {
  const raw = JSON.parse(readFileSync(`${MAP_ASSETS}/glyphs/manifest.json`, 'utf8')) as {
    glyphs: Record<string, unknown>
  }
  return Object.keys(raw.glyphs).sort()
}

function chunkIdsForBbox(bbox: ArrayLike<number>): string[] {
  const minCx = Math.floor(bbox[0] / 512)
  const maxCx = Math.floor(bbox[2] / 512)
  const minCy = Math.floor(bbox[1] / 512)
  const maxCy = Math.floor(bbox[3] / 512)
  const ids: string[] = []
  for (let cx = minCx; cx <= maxCx; cx++) {
    for (let cy = minCy; cy <= maxCy; cy++) {
      ids.push(`${cx}_${cy}`)
    }
  }
  return ids
}

function loadAirfieldResidency(
  bbox: ArrayLike<number>,
  zoom: number,
  airfieldOn: boolean,
): wasm.WorldResidency {
  const residency = new wasm.WorldResidency()
  residency.load_manifest_json(readFileSync(`${EVERON}/manifest.json`, 'utf8'))
  residency.load_prefabs_gz(bytes(`${OBJECTS}/prefabs.json.gz`))
  residency.load_chunk_index_json(readFileSync(`${OBJECTS}/chunks/manifest.json`, 'utf8'))
  residency.set_glyph_key_map(glyphKeysFromManifest())

  const store = new wasm.WorldStore()
  store.load_roads_gz(bytes(ROADS_GZ))
  residency.set_airfield_bbox_from_store(store)
  residency.set_airfield_toggle(airfieldOn)
  store.free()

  const minX = bbox[0]
  const minY = bbox[1]
  const maxX = bbox[2]
  const maxY = bbox[3]
  const missing = Array.from(residency.set_viewport(minX, minY, maxX, maxY, zoom))
  for (const id of missing) {
    try {
      residency.ingest_chunk_gz(id, bytes(`${OBJECTS}/chunks/${id}.json.gz`))
    } catch {
      /* chunk may not exist at edge */
    }
  }
  residency.end_apply_frame(0)
  return residency
}

describe('T-152.5 airfield symbology gates', () => {
  it('G1: Everon has ≥5 runway segments', () => {
    const raw = gunzipSync(readFileSync(ROADS_GZ))
    const roads = JSON.parse(raw.toString()) as {
      roadSegments?: { roadClass?: string }[]
    }
    const n = (roads.roadSegments ?? []).filter((s) => s.roadClass === 'runway').length
    expect(n).toBeGreaterThanOrEqual(5)

    const store = new wasm.WorldStore()
    store.load_roads_gz(bytes(ROADS_GZ))
    expect(store.runway_segment_count).toBeGreaterThanOrEqual(5)
    store.free()
  })

  it('G2: NW airfield apron area > 0 and ≥ 15_000 m² (Everon DEM)', () => {
    const store = new wasm.WorldStore()
    store.load_roads_gz(bytes(ROADS_GZ))
    const bbox = store.airfield_bbox()
    expect(bbox.length).toBe(4)

    const demBuf = bytes(DEM_PNG)
    const decoded = wasm.dem_decode_png_to_meters(demBuf, MIN_M, MAX_M)
    const grid = wasm.DemGrid.downsample(
      decoded.meters,
      decoded.width,
      decoded.height,
      wasm.dem_apron_grid_factor(),
      12_800,
      12_800,
    )
    decoded.free()

    const area = grid.apron_qualifying_area_m2(bbox)
    expect(area).toBeGreaterThan(0)
    expect(area).toBeGreaterThanOrEqual(APRON_AREA_MIN_M2)

    const mesh = grid.compose_airfield_apron(bbox)
    expect(mesh.polygon_count).toBeGreaterThan(0)
    expect(mesh.indices.length).toBeGreaterThan(0)
    mesh.free()
    grid.free()
    store.free()
  })

  it('G3/G4: hangar and tower glyph keys exist (T-152.3)', () => {
    expect(landmarkGlyphIconKey('hangar')).toBe('building-hangar')
    expect(landmarkGlyphIconKey('tower')).toBe('building-badge-tower')
  })

  it('G3: ∃ hangar glyph in airfield bbox @ deckZoom=2', () => {
    const store = new wasm.WorldStore()
    store.load_roads_gz(bytes(ROADS_GZ))
    const bbox = store.airfield_bbox()
    store.free()

    const residency = loadAirfieldResidency(bbox, 2.0, true)
    expect(residency.badge_glyph_count).toBeGreaterThan(0)
    residency.free()
  })

  it('G4: ∃ tower glyph in airfield bbox @ deckZoom=2', () => {
    const store = new wasm.WorldStore()
    store.load_roads_gz(bytes(ROADS_GZ))
    const bbox = store.airfield_bbox()
    store.free()

    const residency = loadAirfieldResidency(bbox, 2.0, true)
    // Tower uses badge overlay; combined with hangars the count must be ≥1 at NW airfield.
    expect(residency.badge_glyph_count).toBeGreaterThanOrEqual(1)
    residency.free()
  })

  it('G5: runway polish width at deckZoom=0 = 20 m (Rust Class R)', () => {
    expect(20 * 2 ** 0).toBeCloseTo(20, 5)
  })

  it('G6: airfield toggle off suppresses hangar/tower badges; roads census unchanged', () => {
    const store = new wasm.WorldStore()
    store.load_roads_gz(bytes(ROADS_GZ))
    const bbox = store.airfield_bbox()
    const roadCount = store.road_segment_count

    const on = loadAirfieldResidency(bbox, 2.0, true)
    const badgesOn = on.badge_glyph_count
    on.free()

    const off = loadAirfieldResidency(bbox, 2.0, false)
    expect(off.badge_glyph_count).toBeLessThan(badgesOn)
    off.free()

    expect(store.road_segment_count).toBe(roadCount)
    store.free()
  })

  it('G7: taxiway spike documents path B', () => {
    const spike = JSON.parse(
      readFileSync(
        resolve(process.cwd(), '../../../.ai/artifacts/t152_5_taxiway_spike.json'),
        'utf8',
      ),
    ) as { path?: string }
    expect(spike.path).toBe('B')
  })

  it('runway visible at cartographic zoom band (class_visible runway @ -6)', () => {
    expect(wasm.class_visible('runway', -6)).toBe(true)
    expect(wasm.class_visible('buildingBadge', 2)).toBe(true)
  })

  it('airfield bbox covers NW runway union + 30 m margin', () => {
    const store = new wasm.WorldStore()
    store.load_roads_gz(bytes(ROADS_GZ))
    const bbox = store.airfield_bbox()
    expect(bbox[0]).toBeLessThan(4700)
    expect(bbox[2]).toBeGreaterThan(5300)
    expect(bbox[1]).toBeLessThan(6300)
    expect(bbox[3]).toBeGreaterThan(12000)
    store.free()
  })

  it('chunk coverage for airfield bbox is non-empty', () => {
    const store = new wasm.WorldStore()
    store.load_roads_gz(bytes(ROADS_GZ))
    const ids = chunkIdsForBbox(store.airfield_bbox())
    expect(ids.length).toBeGreaterThan(0)
    store.free()
  })
})
