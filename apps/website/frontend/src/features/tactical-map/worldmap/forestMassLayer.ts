// T-090.8.1 — Forest mass render layers (Map Engine v2 slot 8, ids `world-forest` +
// `world-forest-outline`, drawn after buildings per the t090_10 layer stack). Thin builders
// over the forestMassStore composite: geometry stays the binary typed arrays the worker
// shipped (SolidPolygonLayer/LineLayer binary-attribute form — zero per-instance JS objects,
// plan R1/R9), styling comes from forestMass.forestFillAlpha (N3 α ladder) and the locked
// rgba(34,120,60,α) fill. Visibility booleans come from lodGates.classVisible — sole
// authority (LOD5); `visible:false` keeps the buffers on the GPU across band crossings.
// Mass layers — never pickable (N4: picking is worker-owned).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { LineLayer, SolidPolygonLayer } from '@deck.gl/layers'
import { FOREST_FILL_RGB, forestFillAlpha } from './forestMass'
import type { ForestMassComposite } from './forestMassStore'

/** Outline stroke: darker forest green, 1 px hairline (N3 "outline" column). */
export const FOREST_OUTLINE_RGBA: [number, number, number, number] = [24, 90, 45, 230]

/** Fill color at a zoom: locked RGB + the N3 α ladder as an 8-bit alpha. Deck re-evaluates
 *  constant accessors on value change, so band crossings restyle without updateTriggers. */
export function forestFillColor(deckZoom: number): [number, number, number, number] {
  return [...FOREST_FILL_RGB, Math.round(255 * forestFillAlpha(deckZoom))]
}

export interface BuildForestLayersOpts {
  mass: ForestMassComposite
  /** forestMass.forestFillAlpha(deckZoom) — caller-derived so the hook memo keys on the α
   *  BAND, not raw zoom (continuous zoom must not rebuild layers — T-090.5.2 memo rule). */
  fillAlpha: number
  /** lodGates.classVisible('forestFill', deckZoom) — caller-derived (memo key). */
  fillVisible: boolean
  /** lodGates.classVisible('forestOutline', deckZoom) — caller-derived (memo key). */
  outlineVisible: boolean
}

/** Build the slot-8 forest layers: `world-forest` (marching-squares fill, closed rings in
 *  binary form — `_normalize:false`) and `world-forest-outline` (iso contour segments,
 *  stride-interleaved src/tgt over one buffer). Layers exist whenever geometry exists;
 *  gates drive `visible` so pan/zoom never re-uploads buffers. */
export function buildForestLayers(opts: BuildForestLayersOpts): (SolidPolygonLayer | LineLayer)[] {
  const { mass } = opts
  const layers: (SolidPolygonLayer | LineLayer)[] = []
  if (mass.polygonCount > 0) {
    layers.push(
      new SolidPolygonLayer({
        id: 'world-forest',
        data: {
          length: mass.polygonCount,
          startIndices: mass.fillStartIndices,
          attributes: {
            getPolygon: { value: mass.fillPositions, size: 2 },
          },
        },
        _normalize: false,
        visible: opts.fillVisible && opts.fillAlpha > 0,
        coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
        getFillColor: [...FOREST_FILL_RGB, Math.round(255 * opts.fillAlpha)],
        pickable: false,
      }),
    )
  }
  if (mass.segmentCount > 0) {
    layers.push(
      new LineLayer({
        id: 'world-forest-outline',
        data: {
          length: mass.segmentCount,
          attributes: {
            // Interleaved [x0,y0,x1,y1] per segment: src/tgt view the same buffer with a
            // 16-byte stride (4 f32) and an 8-byte target offset — no repack on delivery.
            getSourcePosition: { value: mass.outlineSegments, size: 2, stride: 16, offset: 0 },
            getTargetPosition: { value: mass.outlineSegments, size: 2, stride: 16, offset: 8 },
          },
        },
        visible: opts.outlineVisible,
        coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
        getColor: FOREST_OUTLINE_RGBA,
        getWidth: 1,
        widthUnits: 'pixels',
        widthMinPixels: 1,
        pickable: false,
      }),
    )
  }
  return layers
}
