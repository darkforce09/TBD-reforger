// DEM load lifecycle + degraded flat mode (T-091.1). Module singleton; the public API
// (loadDemForTerrain / isDemReady / isDemDegraded / sampleElevation) is consumed by
// T-091.2 (Z on place/move). Out of bounds clamps to terrain bounds before sampling so
// the editor never throws; degraded / not-ready → sampleElevation returns 0 (not NaN).

import { toast } from 'sonner'
import { getTerrain, type TerrainId } from '../coords/terrains'
import {
  fetchTerrainManifest,
  resolveDemUrl,
  type TerrainManifest,
} from './terrainManifest'
import { decodeDemPng, buildMetersCache } from './DemTexture'
import { worldToPixel, bilinearSample } from './sampleElevation'

type DemState = 'idle' | 'loading' | 'ready' | 'degraded'

interface DemData {
  manifest: TerrainManifest
  metersCache: Float32Array
  width: number
  height: number
}

let state: DemState = 'idle'
let loadedTerrainId: TerrainId | null = null
let data: DemData | null = null
let inflight: Promise<void> | null = null

// Tiny external store (T-091.2 follow-up): the DEM loads async (72 MB decode), so consumers
// need to re-render when it becomes ready/degraded — otherwise a hillshade toggled on while the
// DEM is still loading never paints. `version` bumps on every state change; React reads it via
// useSyncExternalStore (see useDemVersion.ts).
const listeners = new Set<() => void>()
let version = 0

function notify(): void {
  version++
  for (const l of listeners) l()
}

function setDemState(next: DemState): void {
  state = next
  notify()
}

/** Subscribe to DEM state changes; returns an unsubscribe. */
export function subscribeDem(cb: () => void): () => void {
  listeners.add(cb)
  return () => listeners.delete(cb)
}

/** Monotonic counter bumped on every DEM state change (useSyncExternalStore snapshot). */
export function getDemVersion(): number {
  return version
}

export function isDemReady(): boolean {
  return state === 'ready'
}

export function isDemDegraded(): boolean {
  return state === 'degraded'
}

function degrade(terrainId: TerrainId, message: string): void {
  data = null
  setDemState('degraded')
  toast.error(message, {
    action: { label: 'Retry', onClick: () => void loadDemForTerrain(terrainId) },
  })
}

/**
 * Start (or restart) the async DEM load for a terrain. Idempotent for the same terrain
 * while ready/loading; resets and reloads when the terrain id changes or after a degrade
 * (Retry). TacticalMap calls this in a useEffect keyed on terrainId.
 */
export function loadDemForTerrain(terrainId: TerrainId): Promise<void> {
  if (terrainId === loadedTerrainId && (state === 'ready' || state === 'loading')) {
    return inflight ?? Promise.resolve()
  }
  loadedTerrainId = terrainId
  data = null
  setDemState('loading')
  inflight = doLoad(terrainId).finally(() => {
    inflight = null
  })
  return inflight
}

async function doLoad(terrainId: TerrainId): Promise<void> {
  const terrain = getTerrain(terrainId)
  if (!terrain.manifestUrl) {
    degrade(terrainId, `No terrain manifest for ${terrain.name}`)
    return
  }
  try {
    const manifest = await fetchTerrainManifest(terrain.manifestUrl)
    // Stub terrain (Arland widthPx:0) — degraded flat mode, no PNG fetch.
    if (manifest.dem.widthPx === 0 || manifest.dem.heightPx === 0) {
      degrade(terrainId, `${terrain.name} terrain has no elevation data yet`)
      return
    }
    const demUrl = resolveDemUrl(terrain.manifestUrl, manifest.dem.path)
    const res = await fetch(demUrl)
    if (!res.ok) {
      degrade(terrainId, `DEM fetch failed (${res.status})`)
      return
    }
    const buf = new Uint8Array(await res.arrayBuffer())
    const { raster, width, height } = decodeDemPng(buf)
    if (width !== manifest.dem.widthPx || height !== manifest.dem.heightPx) {
      degrade(
        terrainId,
        `DEM size ${width}×${height} != manifest ${manifest.dem.widthPx}×${manifest.dem.heightPx}`,
      )
      return
    }
    const metersCache = buildMetersCache(raster, manifest)
    // Guard a terrain switch that landed mid-load — the newer load owns the cache.
    if (loadedTerrainId !== terrainId) return
    data = { manifest, metersCache, width, height }
    setDemState('ready')
  } catch {
    degrade(terrainId, `Could not load ${terrain.name} elevation`)
  }
}

/**
 * Bilinear elevation (meters ASL) at editor x/y. Returns 0 when degraded or not ready.
 * Clamps (x,y) to terrain bounds before worldToPixel (matches slot clamp in ydoc.ts) so
 * the public API never throws; rounds to manifest.precision.storageDecimals.
 */
export function sampleElevation(x: number, y: number): number {
  if (state !== 'ready' || !data) return 0
  const { manifest, metersCache, width, height } = data
  const terrain = getTerrain(loadedTerrainId ?? undefined)
  const cx = Math.min(Math.max(x, 0), terrain.width)
  const cy = Math.min(Math.max(y, 0), terrain.height)
  const { px, py } = worldToPixel(cx, cy, manifest)
  const meters = bilinearSample(metersCache, width, height, px, py)
  const decimals = manifest.precision?.storageDecimals ?? 3
  const f = 10 ** decimals
  return Math.round(meters * f) / f
}

/**
 * Internal overlay accessor (T-091.2 hillshade) — NOT re-exported from the public barrel.
 * Returns the ready meters cache + dims + terrain id/manifest, else null.
 */
export function getDemRasterForOverlay():
  | {
      metersCache: Float32Array
      width: number
      height: number
      terrainId: TerrainId
      manifest: TerrainManifest
    }
  | null {
  if (state !== 'ready' || !data || !loadedTerrainId) return null
  return {
    metersCache: data.metersCache,
    width: data.width,
    height: data.height,
    terrainId: loadedTerrainId,
    manifest: data.manifest,
  }
}

/** Test-only reset of the singleton. Not re-exported from the public barrel. */
export function _resetForTest(): void {
  loadedTerrainId = null
  data = null
  inflight = null
  setDemState('idle')
}
