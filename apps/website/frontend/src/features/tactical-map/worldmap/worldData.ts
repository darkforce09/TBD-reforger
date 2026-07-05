// T-090.5.3 — World-object manifest gate + roads one-shot (main thread). The T-090.5.2
// bulk loader (fetch-all chunks, distill buildings) is gone: building/pier instances now
// stream through the worker + chunkStore (plan §6); this module keeps only what is
// deliberately NOT streamed:
//
//  - the manifest gate — no `objects` block ⇒ v2 layers cleanly absent (plan R11), and
//  - roads — 888 Everon segments, one small artifact, parsed once per tab. The parse
//    (extractRoadCenterline over quad-soup) reuses roadLayer's pure exports, which import
//    deck.gl at module scope — exactly why roads stay main-thread instead of riding the
//    worker (the worker bundle must not drag deck.gl in; see worldObjectsCore.ts header).

import type { TerrainDef } from '../coords/terrains'
import { parseRoadsPayload, type RoadSegment } from './roadLayer'

/** Fetch + parse a (possibly gzipped) JSON asset. Static .gz files are served raw (no
 *  Content-Encoding), so gunzip via DecompressionStream when the gzip magic is present; a
 *  transparently-decompressed body (or plain .json) falls through to direct parse.
 *  Shared with the other small main-thread one-shots (landCoverRegions, T-090.8.1). */
export async function fetchGzJson(url: string): Promise<unknown | null> {
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

const roadPromises = new Map<string, Promise<RoadSegment[]>>()
const loadedRoads = new Map<string, RoadSegment[]>()

async function loadTerrainRoads(terrain: TerrainDef): Promise<RoadSegment[]> {
  if (!terrain.manifestUrl) return []
  const manifest = (await fetchGzJson(terrain.manifestUrl)) as {
    objects?: { roadsPath?: string }
  } | null
  const roadsPath = manifest?.objects?.roadsPath
  if (!roadsPath) return []
  const base = terrain.manifestUrl.slice(0, terrain.manifestUrl.lastIndexOf('/'))
  return parseRoadsPayload(await fetchGzJson(`${base}/${roadsPath}`))
}

/** Load (or join the in-flight load of) a terrain's road network. Failures degrade to the
 *  empty set with one warning — the editor keeps its sat/hillshade/grid path regardless. */
export function loadWorldRoads(terrain: TerrainDef): Promise<RoadSegment[]> {
  const key = terrain.id
  let p = roadPromises.get(key)
  if (!p) {
    p = loadTerrainRoads(terrain)
      .catch((e: unknown) => {
        console.warn(
          `[worldmap] road load failed for ${key} — roads off`,
          e instanceof Error ? e.message : e,
        )
        return []
      })
      .then((roads) => {
        loadedRoads.set(key, roads)
        return roads
      })
    roadPromises.set(key, p)
  }
  return p
}

/** Synchronous view for layer assembly: resolved roads or null while loading/absent. */
export function getLoadedWorldRoads(terrainId: string): RoadSegment[] | null {
  return loadedRoads.get(terrainId) ?? null
}
