// T-090.5.4 — Contour render layer (Map Engine v2 slot 5, id `world-contours`). Thin builder
// over the demVectorStore contour composite: iso segments stay the interleaved [x0,y0,x1,y1]
// buffer the worker shipped (LineLayer binary form, src/tgt view the same buffer at stride 16 /
// target offset 8 — the world-forest-outline pattern, no repack). Drawn after world-landcover,
// before world-roads. Mass layer — never pickable (N4).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { LineLayer } from '@deck.gl/layers'
import type { ContourComposite } from './demVectorStore'

/** Contour stroke: muted brown hairline (provisional — operator visual pass tunes). */
export const CONTOUR_RGBA: [number, number, number, number] = [120, 96, 64, 200]

export interface BuildContourLayerOpts {
  contours: ContourComposite
  /** lodGates.classVisible('contour', deckZoom) — caller-derived (memo key). */
  visible: boolean
}

/** Build the slot-5 contour layer, or null when there are no segments. */
export function buildContourLayer(opts: BuildContourLayerOpts): LineLayer | null {
  const { contours } = opts
  if (contours.segmentCount === 0) return null
  return new LineLayer({
    id: 'world-contours',
    data: {
      length: contours.segmentCount,
      attributes: {
        getSourcePosition: { value: contours.segments, size: 2, stride: 16, offset: 0 },
        getTargetPosition: { value: contours.segments, size: 2, stride: 16, offset: 8 },
      },
    },
    visible: opts.visible,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    getColor: CONTOUR_RGBA,
    getWidth: 1,
    widthUnits: 'pixels',
    widthMinPixels: 1,
    pickable: false,
  })
}
