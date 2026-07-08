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
//
// T-151.1 L2: the manifest resolve chain + pyramid LOD math moved to basemapResolve.ts (shared,
// byte-identically, with the wgpu editor path). This hook is the Deck consumer of those helpers.

import { useEffect, useMemo, useState } from 'react'
import { BitmapLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import type { Device, Texture } from '@luma.gl/core'
import type { TerrainDef } from '../coords/terrains'
import { tileUrl } from './tileUrl'
import { loadUnifiedSatTexture } from './satelliteUnified'
import type { BasemapView } from '../state/basemapView'
import type { MapViewState } from '../types'
import {
  computeLod,
  resolveBasemapMode,
  satelliteOpacity,
  type Resolved,
} from './basemapResolve'

// Re-exported for back-compat with the T-090 public surface (moved to basemapResolve.ts, T-151.1).
export { MAX_VISIBLE_BASEMAP_TILES } from './basemapResolve'
export type { BasemapRenderMode } from './basemapResolve'

export function useTerrainBasemapLayer({
  terrain,
  basemapView,
  visible,
  viewState,
  viewBounds,
  device,
  opacity = 1,
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
  /** Satellite-field opacity from the mapStyle (T-090.5.1, plan §4.3): 1 satellite / 0.55
   *  hybrid. Applies to SATELLITE-view layers only — Map-view pyramid tiles always draw at 1
   *  (in `mapStyle:'map'` they ARE the basemap; sat is simply not the active view there). */
  opacity?: number
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

  const satOpacity = satelliteOpacity(basemapView, opacity)

  return useMemo(() => {
    const { width: w, height: h } = terrain
    if (unifiedTexture) {
      return [
        new BitmapLayer({
          id: 'basemap-satellite-unified',
          coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
          bounds: [0, 0, w, h],
          image: unifiedTexture,
          opacity: satOpacity,
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
          opacity: satOpacity,
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
          opacity: satOpacity,
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
              opacity: satOpacity,
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
    satOpacity,
  ])
}
