// T-090.8.1 — Land-cover regions (Map Engine v2 slot 4, id `world-landcover`, drawn UNDER
// roads per the t090_10 layer stack). Data = the T-090.3.2 Path B hulls in
// `objects/forest-regions.json.gz` (map-object-region schema: forest / field / waterBody).
// Role: instant whole-terrain context — one pinned ~43 KB fetch (N11 P2b "region index
// pinned") that reads as light land-cover tint at any zoom and covers the gaps while the
// higher-fidelity marching-squares forest mass (forestMass/forestMassStore, slot 8) streams
// in. Tint α is deliberately low so hull + mass overlap never double-darkens the forest.
//
// Region metadata (treeCount, dominantSpeciesClass, areaHa, coverType) is parsed and kept —
// the T-090.9 hover/inspect slice consumes it; this layer stays pickable:false (N4: picking
// is worker-owned, never Deck GPU).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { PolygonLayer } from '@deck.gl/layers'
import type { TerrainDef } from '../coords/terrains'
import { fetchGzJson } from './worldData'

/** Region kinds in the map-object-region schema (N5 taxonomy). */
export type LandCoverKind = 'forest' | 'field' | 'waterBody'

/** One land-cover region hull (map-object-region row, narrowed). */
export interface LandCoverRegion {
  id: string
  kind: LandCoverKind
  /** One or more rings [[x,y],…] in world meters — first outer, rest holes (Deck complex polygon). */
  polygon: [number, number][][]
  treeCount?: number
  dominantSpeciesClass?: string
  densityPerHa?: number
  areaHa?: number
  coverType?: string
}

type Rgba = [number, number, number, number]

/** Per-kind context tints. Forest sits UNDER the rgba(34,120,60,α) marching-squares mass —
 *  low α by design (overlap ≈ total, so the sum stays inside the N3 ladder). Field/waterBody
 *  are style-ready but unexercised on Everon (the shipped export has forest rows only). */
export const LANDCOVER_FILL: Record<LandCoverKind, Rgba> = {
  forest: [46, 90, 50, 38],
  field: [205, 198, 163, 31],
  waterBody: [90, 140, 185, 89],
}

const isKind = (v: unknown): v is LandCoverKind =>
  v === 'forest' || v === 'field' || v === 'waterBody'

const isPoint = (p: unknown): p is [number, number] =>
  Array.isArray(p) && p.length >= 2 && Number.isFinite(p[0]) && Number.isFinite(p[1])

function narrowRings(polygon: unknown): [number, number][][] | null {
  if (!Array.isArray(polygon) || polygon.length === 0) return null
  const rings: [number, number][][] = []
  for (const ring of polygon) {
    if (!Array.isArray(ring) || ring.length < 3) return null
    for (const p of ring) if (!isPoint(p)) return null
    rings.push(ring as [number, number][])
  }
  return rings
}

/** Narrow the forest-regions payload to render rows; malformed rows drop silently (the
 *  export gates F1/F2 own data quality — the renderer just refuses to crash on drift).
 *  Accepts both artifact shapes: the shipped export wraps rows in `{ regions: [...] }`,
 *  the T-090.2 golden is a bare region array. */
export function parseRegionsPayload(raw: unknown): LandCoverRegion[] {
  const rows = Array.isArray(raw) ? raw : (raw as { regions?: unknown } | null)?.regions
  if (!Array.isArray(rows)) return []
  const out: LandCoverRegion[] = []
  for (const row of rows) {
    const r = row as {
      id?: unknown
      kind?: unknown
      polygon?: unknown
      treeCount?: unknown
      dominantSpeciesClass?: unknown
      densityPerHa?: unknown
      areaHa?: unknown
      coverType?: unknown
    }
    if (typeof r.id !== 'string' || !isKind(r.kind)) continue
    const rings = narrowRings(r.polygon)
    if (!rings) continue
    out.push({
      id: r.id,
      kind: r.kind,
      polygon: rings,
      treeCount: typeof r.treeCount === 'number' ? r.treeCount : undefined,
      dominantSpeciesClass:
        typeof r.dominantSpeciesClass === 'string' ? r.dominantSpeciesClass : undefined,
      densityPerHa: typeof r.densityPerHa === 'number' ? r.densityPerHa : undefined,
      areaHa: typeof r.areaHa === 'number' ? r.areaHa : undefined,
      coverType: typeof r.coverType === 'string' ? r.coverType : undefined,
    })
  }
  return out
}

const regionPromises = new Map<string, Promise<LandCoverRegion[]>>()

async function loadTerrainRegions(terrain: TerrainDef): Promise<LandCoverRegion[]> {
  if (!terrain.manifestUrl) return []
  const manifest = (await fetchGzJson(terrain.manifestUrl)) as {
    objects?: { regionsPath?: string }
  } | null
  const regionsPath = manifest?.objects?.regionsPath
  if (!regionsPath) return []
  const base = terrain.manifestUrl.slice(0, terrain.manifestUrl.lastIndexOf('/'))
  return parseRegionsPayload(await fetchGzJson(`${base}/${regionsPath}`))
}

/** Load (or join the in-flight load of) a terrain's land-cover regions — module-cached per
 *  terrain like loadWorldRoads; failures degrade to the empty set with one warning. */
export function loadLandCoverRegions(terrain: TerrainDef): Promise<LandCoverRegion[]> {
  const key = terrain.id
  let p = regionPromises.get(key)
  if (!p) {
    p = loadTerrainRegions(terrain).catch((e: unknown) => {
      console.warn(
        `[worldmap] land-cover regions load failed for ${key} — landcover off`,
        e instanceof Error ? e.message : e,
      )
      return []
    })
    regionPromises.set(key, p)
  }
  return p
}

/** Build the `world-landcover` PolygonLayer (slot 4 — under roads). `visible` gates via
 *  Deck so the ring data stays on the GPU across band crossings. */
export function buildLandCoverLayer(opts: {
  regions: LandCoverRegion[]
  visible: boolean
}): PolygonLayer<LandCoverRegion> | null {
  if (opts.regions.length === 0) return null
  return new PolygonLayer<LandCoverRegion>({
    id: 'world-landcover',
    data: opts.regions,
    visible: opts.visible,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    getPolygon: (d) => d.polygon,
    filled: true,
    stroked: false,
    getFillColor: (d) => LANDCOVER_FILL[d.kind],
    pickable: false,
  })
}
