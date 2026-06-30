// T-090.1 — Aligned satellite basemap, drawn UNDER the procedural grid. Cartesian only
// (COORDINATE_SYSTEM.CARTESIAN, world meters, origin bottom-left) — never Web Mercator, and
// never @deck.gl/geo-layers TileLayer (which assumes geospatial tiling). We hand-roll the
// raster with BitmapLayers, exactly like the DEM hillshade (useDemLayer.ts), so the basemap
// stays in the same flat world space as the slots.
//
// Render modes (logged as `basemapRenderMode`):
//   • pyramid (primary)   — real viewport LOD: pick the pyramid level from the deck zoom, cull
//     to the visible world AABB, cap MAX_VISIBLE_BASEMAP_TILES, and mount only those tiles as
//     BitmapLayers (each fetched through `tileUrl`, the single XYZ↔south-first Y inversion).
//     Zooming in loads deeper, sharper tiles instead of stretching one image (T-090.1.1 blur fix).
//   • single-bitmap (fallback) — one full-extent BitmapLayer from `full.webp` when no pyramid is
//     on disk. Top scanline → north (maxY), no Y flip.
//
// The layer array is memoized on the *discrete* level+tile-range, so a micro-pan within the same
// tiles is a no-op and Deck reuses textures (stable ids) — pan/zoom holds the T-057 ≥55 fps bar.
// 404 / no tiles → no layers + onDegraded() (host shows a grid-only toast).

import { useEffect, useMemo, useState } from 'react'
import { BitmapLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import type { TerrainDef } from '../coords/terrains'
import { loadTerrainManifest, probeUrl } from '../coords/terrainManifest'
import { tileUrl } from './tileUrl'
import type { BasemapView } from '../state/basemapView'
import type { MapViewState } from '../types'

/** Hard cap on simultaneously-mounted basemap tiles (110% patch #3). */
export const MAX_VISIBLE_BASEMAP_TILES = 64
const TILE_PX = 256

export type BasemapRenderMode = 'loading' | 'single-bitmap' | 'pyramid' | 'none'

interface Resolved {
  mode: BasemapRenderMode
  image?: string
  template?: string
  minZoom: number
  maxZoom: number
}

/**
 * Resolve the satellite render mode from the terrain manifest. **Prefers the pyramid** (probe z0
 * `0/0/0`, which exists regardless of the Y-flip since tmsY(0,0)=0) so zoomed-in detail is sharp;
 * falls back to the full-res single ortho (`full.webp`) only when no pyramid is committed.
 * `none` → grid-only + degraded toast.
 */
async function resolveSatelliteMode(manifestUrl: string | undefined): Promise<Resolved> {
  const m = await loadTerrainManifest(manifestUrl)
  const minZoom = m?.tiles?.minZoom ?? 0
  const maxZoom = m?.tiles?.maxZoom ?? 5
  const tmpl = m?.tiles?.satellite?.urlTemplate
  if (!tmpl) return { mode: 'none', minZoom, maxZoom }
  const z0 = tmpl.replace('{z}', '0').replace('{x}', '0').replace('{y}', '0')
  if (await probeUrl(z0)) return { mode: 'pyramid', template: tmpl, minZoom, maxZoom }
  const fullUrl = tmpl.replace('{z}/{x}/{y}', 'full')
  if (await probeUrl(fullUrl)) return { mode: 'single-bitmap', image: fullUrl, minZoom, maxZoom }
  return { mode: 'none', minZoom, maxZoom }
}

const clampInt = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v))

type Lod =
  | { kind: 'none' }
  | { kind: 'single'; image: string }
  | { kind: 'pyramid'; z: number; txMin: number; txMax: number; tyMin: number; tyMax: number; template: string }

/**
 * Choose the pyramid level from the deck zoom, then cull tiles to the visible world AABB and drop
 * coarser until the visible count fits the cap. Cheap (runs every render); the heavy layer build is
 * memoized downstream on the discrete result.
 */
function computeLod(
  resolved: Resolved,
  viewState: MapViewState,
  viewBounds: [number, number, number, number] | null,
  terrain: TerrainDef,
  visible: boolean,
  basemapView: BasemapView,
): Lod {
  if (basemapView !== 'satellite' || !visible || resolved.mode === 'loading' || resolved.mode === 'none')
    return { kind: 'none' }
  if (resolved.mode === 'single-bitmap' && resolved.image) return { kind: 'single', image: resolved.image }
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
  onDegraded,
}: {
  terrain: TerrainDef
  basemapView: BasemapView
  visible: boolean
  viewState: MapViewState
  viewBounds: [number, number, number, number] | null
  onDegraded?: () => void
}): BitmapLayer[] {
  // `<TacticalMap>` is keyed on terrain id (it remounts on terrain switch), so this resets to
  // `loading` per terrain — no synchronous reset needed and no stale tiles cross terrains.
  const [resolved, setResolved] = useState<Resolved>({ mode: 'loading', minZoom: 0, maxZoom: 5 })

  useEffect(() => {
    if (basemapView !== 'satellite') return
    let alive = true
    void resolveSatelliteMode(terrain.manifestUrl).then((r) => {
      if (!alive) return
      setResolved(r)
      if (r.mode === 'none') onDegraded?.()
    })
    return () => {
      alive = false
    }
    // onDegraded is a stable host callback; manifestUrl + view are the real inputs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terrain.manifestUrl, basemapView])

  // Cheap every-render LOD selection; the layer build below memoizes on its discrete result so a
  // pan within the same tiles doesn't rebuild.
  const lod = computeLod(resolved, viewState, viewBounds, terrain, visible, basemapView)
  const kind = lod.kind
  const image = lod.kind === 'single' ? lod.image : undefined
  const z = lod.kind === 'pyramid' ? lod.z : -1
  const txMin = lod.kind === 'pyramid' ? lod.txMin : 0
  const txMax = lod.kind === 'pyramid' ? lod.txMax : 0
  const tyMin = lod.kind === 'pyramid' ? lod.tyMin : 0
  const tyMax = lod.kind === 'pyramid' ? lod.tyMax : 0
  const template = lod.kind === 'pyramid' ? lod.template : undefined

  return useMemo(() => {
    const { width: w, height: h } = terrain
    if (kind === 'single' && image) {
      return [
        new BitmapLayer({
          id: 'basemap-satellite',
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
              id: `basemap-sat-${z}-${tx}-${ty}`,
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
  }, [kind, image, z, txMin, txMax, tyMin, tyMax, template, terrain])
}
