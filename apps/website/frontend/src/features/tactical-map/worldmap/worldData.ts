// T-090.5.2 — World-object data loader (interim, main-thread). Loads the committed Map Engine
// v2 export for a terrain once per tab: roads.json.gz one-shot (766 Everon segments) + the P1
// building set distilled from the chunk files (5,606 instances out of 507k mixed rows — trees
// are parsed and immediately discarded, never retained). Deliberately simple at this scale;
// T-090.5.3 replaces the fetch path with the worker + chunkStore LRU streaming (plan §6) and
// this module shrinks to the manifest gate.
//
// Chunk enumeration: the export ships an index at `{chunksPath}/manifest.json`
// ({ chunkSizeM, cells: [{cx, cy, path, instanceCount}] } — 270 Everon cells), so only files
// that exist are fetched. If a terrain export ever lacks the index, we fall back to sweeping
// the full chunk grid (chunkMath) and treating missing files as empty chunks.

import type { TerrainDef } from '../coords/terrains'
import { chunkIdsForRect, chunkRectForBbox } from './chunkMath'
import { parseRoadsPayload, type RoadSegment } from './roadLayer'
import {
  buildingPrefabLookup,
  buildingsFromChunkInstances,
  type BuildingInstance,
} from './buildingLayer'

export interface WorldObjectsData {
  roads: RoadSegment[]
  buildings: BuildingInstance[]
}

/** The manifest `objects` block fields this loader consumes (terrain-manifest schema). */
interface ObjectsBlock {
  roadsPath?: string
  prefabsPath?: string
  chunksPath?: string
  chunkSizeM?: number
}

const EMPTY: WorldObjectsData = { roads: [], buildings: [] }
const CHUNK_FETCH_CONCURRENCY = 12

const loadPromises = new Map<string, Promise<WorldObjectsData>>()
const loaded = new Map<string, WorldObjectsData>()

/** Fetch + parse a (possibly gzipped) JSON asset. Static .gz files are served raw (no
 *  Content-Encoding), so gunzip via DecompressionStream when the gzip magic is present; a
 *  transparently-decompressed body (or plain .json) falls through to direct parse. */
async function fetchGzJson(url: string): Promise<unknown | null> {
  const res = await fetch(url)
  const type = res.headers.get('content-type') ?? ''
  // Vite dev SPA-fallbacks unknown paths to index.html with 200 — treat as missing.
  if (!res.ok || type.includes('text/html')) return null
  const buf = new Uint8Array(await res.arrayBuffer())
  if (buf.length >= 2 && buf[0] === 0x1f && buf[1] === 0x8b) {
    const stream = new Blob([buf]).stream().pipeThrough(new DecompressionStream('gzip'))
    return JSON.parse(await new Response(stream).text()) as unknown
  }
  return JSON.parse(new TextDecoder().decode(buf)) as unknown
}

/** One chunk-index row (`{chunksPath}/manifest.json`, written by the T-090.3 export). */
interface ChunkCell {
  path?: string
}

/** Chunk file URLs to fetch: the export's chunk index when present (cell paths are relative
 *  to the terrain base), else a full-grid sweep where misses read as empty chunks. */
async function chunkUrls(
  base: string,
  objects: ObjectsBlock,
  terrain: TerrainDef,
): Promise<string[]> {
  const index = (await fetchGzJson(`${base}/${objects.chunksPath}/manifest.json`).catch(
    () => null,
  )) as { cells?: ChunkCell[] } | null
  if (Array.isArray(index?.cells)) {
    return index.cells
      .filter((c): c is Required<ChunkCell> => typeof c.path === 'string')
      .map((c) => `${base}/${c.path}`)
  }
  const rect = chunkRectForBbox(
    terrain.bounds,
    { width: terrain.width, height: terrain.height },
    objects.chunkSizeM,
  )
  return chunkIdsForRect(rect).map((id) => `${base}/${objects.chunksPath}/${id}.json.gz`)
}

async function loadTerrainObjects(terrain: TerrainDef): Promise<WorldObjectsData> {
  if (!terrain.manifestUrl) return EMPTY
  const manifest = (await fetchGzJson(terrain.manifestUrl)) as {
    objects?: ObjectsBlock
  } | null
  const objects = manifest?.objects
  // No export for this terrain (Arland/custom) → v2 layers cleanly absent (plan R11).
  if (!objects) return EMPTY
  const base = terrain.manifestUrl.slice(0, terrain.manifestUrl.lastIndexOf('/'))

  const roadsPromise: Promise<RoadSegment[]> = objects.roadsPath
    ? fetchGzJson(`${base}/${objects.roadsPath}`).then(parseRoadsPayload)
    : Promise.resolve([])

  const buildingsPromise: Promise<BuildingInstance[]> = (async () => {
    if (!objects.prefabsPath || !objects.chunksPath) return []
    const lookup = buildingPrefabLookup(await fetchGzJson(`${base}/${objects.prefabsPath}`))
    if (lookup.size === 0) return []
    const urls = await chunkUrls(base, objects, terrain)
    const buildings: BuildingInstance[] = []
    let next = 0
    const worker = async () => {
      while (next < urls.length) {
        const url = urls[next++]
        const chunk = await fetchGzJson(url).catch(
          () => null, // missing chunk file (fallback sweep) or transient failure → empty
        )
        const instances = (chunk as { instances?: unknown } | null)?.instances
        if (instances) buildings.push(...buildingsFromChunkInstances(instances, lookup))
      }
    }
    await Promise.all(Array.from({ length: CHUNK_FETCH_CONCURRENCY }, worker))
    return buildings
  })()

  const [roads, buildings] = await Promise.all([roadsPromise, buildingsPromise])
  return { roads, buildings }
}

/** Load (or join the in-flight load of) a terrain's world objects. Failures degrade to the
 *  empty set with one warning — the editor keeps its sat/hillshade/grid path regardless. */
export function loadWorldObjects(terrain: TerrainDef): Promise<WorldObjectsData> {
  const key = terrain.id
  let p = loadPromises.get(key)
  if (!p) {
    p = loadTerrainObjects(terrain)
      .catch((e: unknown) => {
        console.warn(
          `[worldmap] world-object load failed for ${key} — layers off`,
          e instanceof Error ? e.message : e,
        )
        return EMPTY
      })
      .then((data) => {
        loaded.set(key, data)
        return data
      })
    loadPromises.set(key, p)
  }
  return p
}

/** Synchronous view for layer assembly: resolved data or null while loading/absent. */
export function getLoadedWorldObjects(terrainId: string): WorldObjectsData | null {
  return loaded.get(terrainId) ?? null
}
