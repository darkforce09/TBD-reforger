// T-090.1 — Terrain manifest loader (shared with the DEM path, T-091.1). Fetches the
// per-terrain manifest.json referenced by `TerrainDef.manifestUrl` and exposes the tile
// pyramid descriptors. Parsing is defensive: a missing/404 manifest resolves to `null`
// (the caller falls back to a grid-only basemap), never throws into render.

/** A single basemap pyramid (satellite or map). `urlTemplate` uses `{z}/{x}/{y}`. The
 *  full-extent single ortho is a `full.webp` sibling of the pyramid (convention, not a
 *  manifest field — the schema keeps `tiles.satellite` closed). */
export interface TileSource {
  path?: string
  urlTemplate?: string
  source?: string
}

/** The `tiles` block of a terrain manifest (dual pyramids — T-090.1/.1.1). */
export interface ManifestTiles {
  type?: string
  indexOrder?: 'xyz' | 'tms'
  urlTemplate?: string
  tileSizePx?: number
  minZoom?: number
  maxZoom?: number
  bounds?: [number, number, number, number]
  alignmentOrigin?: [number, number]
  satellite?: TileSource
  map?: TileSource
}

export interface TerrainManifest {
  terrainId: string
  worldBounds: [number, number, number, number]
  tiles?: ManifestTiles
}

const cache = new Map<string, Promise<TerrainManifest | null>>()

/** Fetch + parse the manifest at `url`, cached per-url. `null` on any failure. */
export function loadTerrainManifest(url: string | undefined): Promise<TerrainManifest | null> {
  if (!url) return Promise.resolve(null)
  const hit = cache.get(url)
  if (hit) return hit
  const p = fetch(url)
    .then((r) => (r.ok ? (r.json() as Promise<TerrainManifest>) : null))
    .catch(() => null)
  cache.set(url, p)
  return p
}

/** Probe whether a tile URL resolves (HEAD; used to pick pyramid vs single-bitmap mode). */
export function probeUrl(url: string): Promise<boolean> {
  return fetch(url, { method: 'HEAD' })
    .then((r) => r.ok)
    .catch(() => false)
}

/** Clear the manifest cache (tests). */
export function _resetManifestCache(): void {
  cache.clear()
}
