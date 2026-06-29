// Terrain manifest types + fetch/validate (T-091.1). Mirrors
// packages/tbd-schema/schema/terrain-manifest.schema.json minimally — only the fields
// the DEM loader consumes. The manifest URL lives on TerrainDef.manifestUrl
// (absolute-path style, e.g. /map-assets/everon/manifest.json).

export interface TerrainManifest {
  terrainId: string
  schemaVersion: number
  /** [minX, minY, maxX, maxY] world meters. */
  worldBounds: [number, number, number, number]
  metersPerPixel: number
  dem: {
    path: string
    widthPx: number
    heightPx: number
    encoding: 'uint16-linear'
    heightRangeMinM: number
    heightRangeMaxM: number
    source: string
    axisFlip?: { x?: boolean; z?: boolean }
  }
  precision: { storageDecimals: number }
}

function isFiniteNum(n: unknown): n is number {
  return typeof n === 'number' && Number.isFinite(n)
}

/** Throws on a malformed/unsupported manifest; returns it typed otherwise. */
export function validateManifest(raw: unknown): TerrainManifest {
  const m = raw as Partial<TerrainManifest> | null
  const dem = m?.dem
  if (!m || !dem) throw new Error('terrain manifest missing `dem`')
  if (dem.encoding !== 'uint16-linear') {
    throw new Error(`unsupported dem.encoding: ${String(dem.encoding)}`)
  }
  if (!isFiniteNum(dem.widthPx) || dem.widthPx < 0) {
    throw new Error(`invalid dem.widthPx: ${String(dem.widthPx)}`)
  }
  if (!isFiniteNum(dem.heightPx) || dem.heightPx < 0) {
    throw new Error(`invalid dem.heightPx: ${String(dem.heightPx)}`)
  }
  if (!isFiniteNum(dem.heightRangeMinM) || !isFiniteNum(dem.heightRangeMaxM)) {
    throw new Error('invalid dem heightRange (min/max not finite)')
  }
  return m as TerrainManifest
}

export async function fetchTerrainManifest(manifestUrl: string): Promise<TerrainManifest> {
  const res = await fetch(manifestUrl)
  if (!res.ok) throw new Error(`manifest fetch failed (${res.status})`)
  return validateManifest(await res.json())
}

/**
 * Resolve the absolute DEM PNG URL from the manifest URL + relative dem.path.
 * manifestUrl is absolute-path style (/map-assets/...), so anchor it on the page
 * origin first (window.location.origin in the browser; localhost fallback for Node tests).
 */
export function resolveDemUrl(manifestUrl: string, demPath: string): string {
  const origin =
    (globalThis as { location?: { origin?: string } }).location?.origin ?? 'http://localhost'
  return new URL(demPath, new URL(manifestUrl, origin)).toString()
}
