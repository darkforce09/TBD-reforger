// T-151.1 L2 — pure basemap resolve + LOD helpers, extracted verbatim from useTerrainBasemapLayer.ts
// so BOTH the Deck hook (useTerrainBasemapLayer.ts) and the wgpu controller (wgpu/wgpuBasemap.ts)
// share ONE proven implementation of the manifest resolve chain (unified → pyramid → single → none)
// and the pyramid LOD math. No React, no Deck, no luma.gl — node-testable (basemapLod.test.ts, a
// Class-S oracle). Behavior is byte-identical to the T-090 Deck path; the extraction is guarded by
// the existing basemap tests + basemapLod.test.ts.

import { loadTerrainManifest, probeUrl, type TileSource } from '../coords/terrainManifest'
import type { TerrainDef } from '../coords/terrains'
import type { BasemapView } from '../state/basemapView'
import type { MapViewState } from '../types'

/** Hard cap on simultaneously-mounted basemap tiles (110% patch #3). */
export const MAX_VISIBLE_BASEMAP_TILES = 64
const TILE_PX = 256

export type BasemapRenderMode = 'loading' | 'unified' | 'single-bitmap' | 'pyramid' | 'none'

export interface Resolved {
  mode: BasemapRenderMode
  /** single-bitmap ortho URL; in unified mode: the instant full.webp preview (optional). */
  image?: string
  template?: string
  unifiedUrl?: string
  minZoom: number
  maxZoom: number
}

/** Zoom bounds shared by every resolved mode. */
interface ZoomRange {
  minZoom: number
  maxZoom: number
}

/**
 * Unified-bundle branch of the resolve (T-090.1.2.8): manifest flags `delivery: "unified"`,
 * the bundle probes OK → unified mode, with the `full.webp` sibling probed in parallel as
 * the instant loading preview. `null` → caller falls through to the pyramid chain.
 */
export async function resolveUnifiedMode(
  sat: TileSource | undefined,
  fullUrl: string | undefined,
  zoom: ZoomRange,
): Promise<Resolved | null> {
  const unifiedUrl = sat?.delivery === 'unified' ? sat.unified?.url : undefined
  if (!unifiedUrl) return null
  const [bundleOk, previewOk] = await Promise.all([
    probeUrl(unifiedUrl),
    fullUrl ? probeUrl(fullUrl) : Promise.resolve(false),
  ])
  if (!bundleOk) return null
  return { mode: 'unified', unifiedUrl, image: previewOk ? fullUrl : undefined, ...zoom }
}

/** Pull one view's manifest fields (nullable manifest → safe defaults). */
function viewFields(
  m: Awaited<ReturnType<typeof loadTerrainManifest>>,
  view: BasemapView,
): {
  zoom: ZoomRange
  src?: TileSource
  tmpl?: string
  fullUrl?: string
} {
  const tiles = m?.tiles
  const src = view === 'map' ? tiles?.map : tiles?.satellite
  // The legacy top-level urlTemplate aliases to SATELLITE only (dual-view manifest contract).
  const tmpl = src?.urlTemplate ?? (view === 'satellite' ? tiles?.urlTemplate : undefined)
  return {
    zoom: { minZoom: tiles?.minZoom ?? 0, maxZoom: tiles?.maxZoom ?? 5 },
    src,
    tmpl,
    fullUrl: tmpl?.replace('{z}/{x}/{y}', 'full'),
  }
}

/**
 * Resolve one basemap view's render mode from the terrain manifest. **Prefers the unified
 * bundle** (`delivery === "unified"`, T-090.1.2.8 — satellite only; `tiles.map` never carries
 * `delivery`, so the Map view falls straight through). Otherwise (or with `forcePyramid`,
 * the runtime fallback after a failed unified load) probe the pyramid (z0 `0/0/0`, which
 * exists regardless of the Y-flip since tmsY(0,0)=0), then the full-res single ortho.
 * `none` → grid-only + degraded toast.
 */
export async function resolveBasemapMode(
  manifestUrl: string | undefined,
  view: BasemapView,
  forcePyramid = false,
): Promise<Resolved> {
  const { zoom, src, tmpl, fullUrl } = viewFields(await loadTerrainManifest(manifestUrl), view)
  if (!forcePyramid) {
    const unified = await resolveUnifiedMode(src, fullUrl, zoom)
    if (unified) return unified
  }
  if (!tmpl) return { mode: 'none', ...zoom }
  const z0 = tmpl.replace('{z}', '0').replace('{x}', '0').replace('{y}', '0')
  if (await probeUrl(z0)) return { mode: 'pyramid', template: tmpl, ...zoom }
  if (fullUrl && (await probeUrl(fullUrl)))
    return { mode: 'single-bitmap', image: fullUrl, ...zoom }
  return { mode: 'none', ...zoom }
}

const clampInt = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v))

/** Style-driven dimming touches satellite-view layers only (T-090.5.1): in `mapStyle:'map'`
 *  the Map pyramid IS the basemap and always draws at 1. */
export const satelliteOpacity = (view: BasemapView, opacity: number) =>
  view === 'satellite' ? opacity : 1

export type Lod =
  | { kind: 'none' }
  | { kind: 'single'; image: string }
  | {
      kind: 'pyramid'
      z: number
      txMin: number
      txMax: number
      tyMin: number
      tyMax: number
      template: string
    }

/**
 * Choose the pyramid level from the deck zoom, then cull tiles to the visible world AABB and drop
 * coarser until the visible count fits the cap. Cheap (runs every render); the heavy layer build is
 * memoized downstream on the discrete result. Unified mode never reaches this — one full-extent
 * texture needs no LOD selection (the GPU picks the mip per fragment).
 */
export function computeLod(
  resolved: Resolved,
  viewState: MapViewState,
  viewBounds: [number, number, number, number] | null,
  terrain: TerrainDef,
  visible: boolean,
): Lod {
  if (
    !visible ||
    resolved.mode === 'loading' ||
    resolved.mode === 'unified' ||
    resolved.mode === 'none'
  )
    return { kind: 'none' }
  if (resolved.mode === 'single-bitmap' && resolved.image)
    return { kind: 'single', image: resolved.image }
  if (resolved.mode !== 'pyramid' || !resolved.template) return { kind: 'none' }

  const { width: w, height: h } = terrain
  const { minZoom, maxZoom, template } = resolved
  // metersPerScreenPx = 2**(-zoom); want tile texel ≤ screen px → z ≈ log2(worldSpan/tileSize)+zoom.
  let z = clampInt(Math.ceil(Math.log2(w / TILE_PX) + viewState.zoom), minZoom, maxZoom)
  const [bx0, by0, bx1, by1] = viewBounds ?? [0, 0, w, h]

  const rangeAt = (lvl: number) => {
    const n = 2 ** lvl
    const twx = w / n
    const twy = h / n
    const txMin = clampInt(Math.floor(bx0 / twx), 0, n - 1)
    const txMax = clampInt(Math.floor(bx1 / twx), 0, n - 1)
    const tyMin = clampInt(Math.floor(by0 / twy), 0, n - 1)
    const tyMax = clampInt(Math.floor(by1 / twy), 0, n - 1)
    return { txMin, txMax, tyMin, tyMax }
  }
  let r = rangeAt(z)
  const count = () => (r.txMax - r.txMin + 1) * (r.tyMax - r.tyMin + 1)
  while (count() > MAX_VISIBLE_BASEMAP_TILES && z > minZoom) {
    z -= 1
    r = rangeAt(z)
  }
  return { kind: 'pyramid', z, ...r, template }
}
