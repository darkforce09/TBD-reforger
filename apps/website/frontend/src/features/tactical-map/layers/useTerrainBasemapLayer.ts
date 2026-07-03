// T-090.1 — Aligned basemap (Satellite | Map views), drawn UNDER the procedural grid.
// Cartesian only (COORDINATE_SYSTEM.CARTESIAN, world meters, origin bottom-left) — never Web
// Mercator, and never @deck.gl/geo-layers TileLayer (which assumes geospatial tiling). We
// hand-roll the raster with BitmapLayers, exactly like the DEM hillshade (useDemLayer.ts), so
// the basemap stays in the same flat world space as the slots.
//
// Render modes (logged as `basemapRenderMode`):
//   • unified (satellite primary, T-090.1.2.8) — ONE tbd-sat bundle fetch → one mipmapped GPU
//     texture (satelliteUnified.ts) on a single full-extent BitmapLayer. Zoom samples GPU mips
//     (trilinear), pan never mounts/unmounts layers — no tile pop-in. While the bundle
//     loads, the capped full.webp ortho renders as an instant preview; the swap to the
//     sharp texture is the only layer change. Falls back to the pyramid on any
//     fetch/parse/decode failure.
//   • pyramid (satellite fallback; Map primary, T-090.1.1) — real viewport LOD: pick the pyramid
//     level from the deck zoom, cull to the visible world AABB, cap MAX_VISIBLE_BASEMAP_TILES,
//     and mount only those tiles as BitmapLayers (each fetched through `tileUrl`, the single
//     XYZ↔south-first Y inversion). Zooming in loads deeper, sharper tiles instead of
//     stretching one image (T-090.1.1 blur fix).
//   • single-bitmap (fallback) — one full-extent BitmapLayer from `full.webp` when no pyramid is
//     on disk. Top scanline → north (maxY), no Y flip.
//
// The two views resolve independently (per-view state) so switching Satellite ↔ Map never
// destroys the loaded unified texture — a round trip back to Satellite is instant. The Map
// view is pyramid LOD only by contract (tiles.map has no unified delivery, dual-view spec).
//
// The pyramid layer array is memoized on the *discrete* level+tile-range, so a micro-pan within
// the same tiles is a no-op and Deck reuses textures (stable ids) — pan/zoom holds the T-057
// ≥55 fps bar. 404 / no tiles → no layers + onDegraded(view) (host shows a grid-only toast).

import { useEffect, useMemo, useState } from 'react'
import { BitmapLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import type { Device, Texture } from '@luma.gl/core'
import type { TerrainDef } from '../coords/terrains'
import { loadTerrainManifest, probeUrl, type TileSource } from '../coords/terrainManifest'
import { tileUrl } from './tileUrl'
import { loadUnifiedSatTexture } from './satelliteUnified'
import type { BasemapView } from '../state/basemapView'
import type { MapViewState } from '../types'

/** Hard cap on simultaneously-mounted basemap tiles (110% patch #3). */
export const MAX_VISIBLE_BASEMAP_TILES = 64
const TILE_PX = 256

export type BasemapRenderMode = 'loading' | 'unified' | 'single-bitmap' | 'pyramid' | 'none'

interface Resolved {
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
async function resolveUnifiedMode(
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
async function resolveBasemapMode(
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
  if (fullUrl && (await probeUrl(fullUrl))) return { mode: 'single-bitmap', image: fullUrl, ...zoom }
  return { mode: 'none', ...zoom }
}

const clampInt = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v))

type Lod =
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
function computeLod(
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

export function useTerrainBasemapLayer({
  terrain,
  basemapView,
  visible,
  viewState,
  viewBounds,
  device,
  onDegraded,
  onProgress,
}: {
  terrain: TerrainDef
  basemapView: BasemapView
  visible: boolean
  viewState: MapViewState
  viewBounds: [number, number, number, number] | null
  /** luma.gl device from Deck's onDeviceInitialized — required for the unified GPU texture. */
  device: Device | null
  onDegraded?: (view: BasemapView) => void
  /** Unified bundle load progress 0..1 (1 = texture live); null = load abandoned. */
  onProgress?: (fraction: number | null) => void
}): BitmapLayer[] {
  // `<TacticalMap>` is keyed on terrain id (it remounts on terrain switch), so these reset to
  // `loading` per terrain — no synchronous reset needed and no stale tiles cross terrains.
  // Satellite and Map resolve into SEPARATE states so a view switch never tears down the
  // satellite lifecycle (in particular the unified GPU texture below stays alive on Map).
  const [resolved, setResolved] = useState<Resolved>({ mode: 'loading', minZoom: 0, maxZoom: 5 })
  const [mapResolved, setMapResolved] = useState<Resolved>({
    mode: 'loading',
    minZoom: 0,
    maxZoom: 5,
  })
  const [texture, setTexture] = useState<Texture | null>(null)

  useEffect(() => {
    if (basemapView !== 'satellite') return
    let alive = true
    void resolveBasemapMode(terrain.manifestUrl, 'satellite').then((r) => {
      if (!alive) return
      setResolved(r)
      if (r.mode === 'none') onDegraded?.('satellite')
    })
    return () => {
      alive = false
    }
    // onDegraded is a stable host callback; manifestUrl + view are the real inputs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terrain.manifestUrl, basemapView])

  // Map view (T-090.1.1) — pyramid LOD only by contract; resolves lazily on first switch.
  useEffect(() => {
    if (basemapView !== 'map') return
    let alive = true
    void resolveBasemapMode(terrain.manifestUrl, 'map').then((r) => {
      if (!alive) return
      setMapResolved(r)
      if (r.mode === 'none') onDegraded?.('map')
    })
    return () => {
      alive = false
    }
    // onDegraded is a stable host callback; manifestUrl + view are the real inputs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terrain.manifestUrl, basemapView])

  // Unified bundle → one GPU texture (T-090.1.2.8). Runs once the Deck device exists; abort +
  // destroy on unmount/terrain switch so VRAM (~873 MB at full base) never outlives the editor.
  // Any failure logs, tells the host the load is abandoned (null progress), and re-resolves with
  // the pyramid forced — onDegraded only fires if that fallback is empty too.
  useEffect(() => {
    if (resolved.mode !== 'unified' || !resolved.unifiedUrl || !device) return
    const ac = new AbortController()
    let tex: Texture | null = null
    void loadUnifiedSatTexture(device, resolved.unifiedUrl, {
      onProgress,
      signal: ac.signal,
    }).then(
      (result) => {
        if (ac.signal.aborted) {
          result.texture.destroy()
          return
        }
        tex = result.texture
        setTexture(result.texture)
      },
      (err: unknown) => {
        if (ac.signal.aborted) return
        console.warn('[basemap] unified satellite failed, falling back to pyramid:', err)
        onProgress?.(null)
        void resolveBasemapMode(terrain.manifestUrl, 'satellite', true).then((r) => {
          if (ac.signal.aborted) return
          setResolved(r)
          if (r.mode === 'none') onDegraded?.('satellite')
        })
      },
    )
    return () => {
      ac.abort()
      // Dismiss any in-flight progress toast — abandoning the load (unmount / terrain
      // switch) must not leave the host stuck on "Loading …%".
      onProgress?.(null)
      if (tex) {
        tex.destroy()
        setTexture(null)
      }
    }
    // Host callbacks are stable (useCallback in MissionCreatorPage); real inputs are the
    // resolved bundle URL + device identity.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resolved.mode, resolved.unifiedUrl, device])

  // Cheap every-render LOD selection on the ACTIVE view's resolve; the layer build below
  // memoizes on its discrete result so a pan within the same tiles doesn't rebuild.
  const active = basemapView === 'map' ? mapResolved : resolved
  const lod = computeLod(active, viewState, viewBounds, terrain, visible)
  const kind = lod.kind
  const image = lod.kind === 'single' ? lod.image : undefined
  const z = lod.kind === 'pyramid' ? lod.z : -1
  const txMin = lod.kind === 'pyramid' ? lod.txMin : 0
  const txMax = lod.kind === 'pyramid' ? lod.txMax : 0
  const tyMin = lod.kind === 'pyramid' ? lod.tyMin : 0
  const tyMax = lod.kind === 'pyramid' ? lod.tyMax : 0
  const template = lod.kind === 'pyramid' ? lod.template : undefined

  // Unified render inputs (kept out of computeLod so pyramid memoization is untouched).
  const unifiedActive = basemapView === 'satellite' && visible && resolved.mode === 'unified'
  const previewImage = unifiedActive && !texture ? resolved.image : undefined
  const unifiedTexture = unifiedActive ? texture : null

  return useMemo(() => {
    const { width: w, height: h } = terrain
    if (unifiedTexture) {
      return [
        new BitmapLayer({
          id: 'basemap-satellite-unified',
          coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
          bounds: [0, 0, w, h],
          image: unifiedTexture,
        }),
      ]
    }
    if (previewImage) {
      // Instant context while the bundle loads; same full-extent geometry as single-bitmap.
      return [
        new BitmapLayer({
          id: 'basemap-satellite-preview',
          coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
          bounds: [0, 0, w, h],
          image: previewImage,
        }),
      ]
    }
    // View-scoped layer ids (basemap-map-* vs basemap-sat[ellite]-*) so Deck never diffs a
    // Map tile into a cached Satellite texture (or vice versa) across a view switch.
    const idPrefix = basemapView === 'map' ? 'map' : 'sat'
    if (kind === 'single' && image) {
      return [
        new BitmapLayer({
          id: `basemap-${basemapView}`,
          coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
          bounds: [0, 0, w, h],
          image,
        }),
      ]
    }
    if (kind === 'pyramid' && template) {
      const n = 2 ** z
      const twx = w / n
      const twy = h / n
      const layers: BitmapLayer[] = []
      for (let ty = tyMin; ty <= tyMax; ty++) {
        for (let tx = txMin; tx <= txMax; tx++) {
          // ty is south-first (y=0 = southern edge, +Y north). The world rect uses it directly;
          // tileUrl converts it to the on-disk XYZ (north-first) row.
          layers.push(
            new BitmapLayer({
              id: `basemap-${idPrefix}-${z}-${tx}-${ty}`,
              coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
              bounds: [tx * twx, ty * twy, (tx + 1) * twx, (ty + 1) * twy],
              image: tileUrl(template, z, tx, ty),
            }),
          )
        }
      }
      return layers
    }
    return []
  }, [
    unifiedTexture,
    previewImage,
    kind,
    image,
    z,
    txMin,
    txMax,
    tyMin,
    tyMax,
    template,
    terrain,
    basemapView,
  ])
}
