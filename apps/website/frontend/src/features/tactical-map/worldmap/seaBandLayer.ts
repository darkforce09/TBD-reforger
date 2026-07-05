// T-090.5.4 — Sea-band render layer (Map Engine v2 slot 2, id `world-sea`). Thin builder over
// the demVectorStore sea composite: geometry stays the binary typed arrays the worker shipped
// (SolidPolygonLayer binary-attribute form — zero per-instance JS objects). Per-vertex RGBA
// carries the nested hypsometric colours (baked opaque); the layer `opacity` is the N3 fade
// ladder (seaFillAlpha) so deeper bands stack-darken while the whole band fades out past +3.
// Rendered ABOVE the satellite basemap but BELOW dem-hillshade (TacticalMap splices it there).
// Mass layer — never pickable (N4).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { SolidPolygonLayer } from '@deck.gl/layers'
import type { SeaBandGeometry } from './seaBand'

export interface BuildSeaBandLayerOpts {
  geometry: SeaBandGeometry
  /** lodGates.classVisible('sea', deckZoom) — caller-derived (memo key). */
  visible: boolean
  /** seaBand.seaFillAlpha(deckZoom) — caller-derived so the memo keys on the α BAND, not raw
   *  zoom (T-057). Drives layer opacity; α 0 → visible:false (buffers stay on the GPU). */
  fillAlpha: number
}

/** Build the slot-2 sea layer (marching-squares + RLE fills, closed rings in binary form —
 *  `_normalize:false`), or null when there is no geometry. Per-vertex `getFillColor` from the
 *  hypsometric palette; `opacity` from the fade ladder. */
export function buildSeaBandLayer(opts: BuildSeaBandLayerOpts): SolidPolygonLayer | null {
  const { geometry } = opts
  if (geometry.polygonCount === 0) return null
  return new SolidPolygonLayer({
    id: 'world-sea',
    data: {
      length: geometry.polygonCount,
      startIndices: geometry.fillStartIndices,
      attributes: {
        getPolygon: { value: geometry.fillPositions, size: 2 },
        getFillColor: { value: geometry.fillColors, size: 4 },
      },
    },
    _normalize: false,
    visible: opts.visible && opts.fillAlpha > 0,
    opacity: opts.fillAlpha,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    pickable: false,
  })
}
