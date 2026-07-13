// T-152.3 — wasm smoke: landmark badges compose @ BUILDING_BADGE_MIN_ZOOM (Class R in Rust tests).
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import * as wasm from '@/wasm/pkg/map_engine_wasm'

const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const EVERON = `${MAP_ASSETS}/everon`
const OBJECTS = `${EVERON}/objects`
const FIXTURE_CHUNK = '2_12'

const bytes = (path: string): Uint8Array => new Uint8Array(readFileSync(path))

function glyphKeysFromManifest(): string[] {
  const raw = JSON.parse(readFileSync(`${MAP_ASSETS}/glyphs/manifest.json`, 'utf8')) as {
    glyphs: Record<string, unknown>
  }
  return Object.keys(raw.glyphs).sort()
}

function chunkBbox(chunkId: string): [number, number, number, number] {
  const [cx, cy] = chunkId.split('_').map(Number)
  const minX = cx * 512
  const minY = cy * 512
  return [minX, minY, minX + 512, minY + 512]
}

describe('world.landmark-glyphs.parity — wasm smoke (T-152.3)', () => {
  it('fixture 2_12: importanceZoom landmarks on @ z=0.9, all badges @ z=2 (T-152.21)', () => {
    const residency = new wasm.WorldResidency()
    residency.load_manifest_json(readFileSync(`${EVERON}/manifest.json`, 'utf8'))
    residency.load_prefabs_gz(bytes(`${OBJECTS}/prefabs.json.gz`))
    residency.load_chunk_index_json(
      readFileSync(`${OBJECTS}/chunks/manifest.json`, 'utf8'),
    )
    residency.set_glyph_key_map(glyphKeysFromManifest())

    const bbox = chunkBbox(FIXTURE_CHUNK)

    // z=0.9 — below BUILDING_BADGE_MIN_ZOOM: T-152.21 wires `render.importanceZoom` (−4), so the
    // importance landmarks (lighthouse/castle) surface early while ordinary buildings stay off
    // (was: ALL badges off — the P1 "white rectangles at default zoom" behavior).
    const missingLow = Array.from(
      residency.set_viewport(bbox[0], bbox[1], bbox[2], bbox[3], 0.9),
    )
    for (const id of missingLow) {
      residency.ingest_chunk_gz(id, bytes(`${OBJECTS}/chunks/${id}.json.gz`))
    }
    residency.end_apply_frame(0)
    const lowBadges = residency.badge_glyph_count
    expect(lowBadges).toBeGreaterThan(0)

    // z=2 — class gate open: every building emits its badge / footprint glyph.
    const missingHigh = Array.from(
      residency.set_viewport(bbox[0], bbox[1], bbox[2], bbox[3], 2.0),
    )
    for (const id of missingHigh) {
      residency.ingest_chunk_gz(id, bytes(`${OBJECTS}/chunks/${id}.json.gz`))
    }
    residency.end_apply_frame(0)
    expect(residency.badge_glyph_count).toBeGreaterThanOrEqual(69)
    // The class gate at z≥1 adds the ordinary buildings that are correctly dark at z=0.9.
    expect(residency.badge_glyph_count).toBeGreaterThan(lowBadges)
    expect(residency.chunks_draw).toBeGreaterThan(0)
  })
})
