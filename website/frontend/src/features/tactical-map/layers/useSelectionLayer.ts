// Marquee box layer (Ultra Plan §4.3 layer #7; Phase 7b). Renders the live left-drag
// box-select rectangle as a translucent Aegis-primary polygon in world meters, so it pans
// and zooms with the map. null when no marquee is active; the actual hit-test on release is
// a GPU rectangle pick (deck.pickObjects) in TacticalMap, not this layer.

import { useMemo } from 'react'
import { PolygonLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { useMapStore } from '../state/useMapStore'

const FILL: [number, number, number, number] = [173, 198, 255, 40] // Aegis primary @ ~16%
const LINE: [number, number, number, number] = [173, 198, 255, 200]

export function useSelectionLayer(): PolygonLayer | null {
  const marquee = useMapStore((s) => s.marquee)

  return useMemo(() => {
    if (!marquee) return null
    const { x0, y0, x1, y1 } = marquee
    const polygon = [
      [x0, y0],
      [x1, y0],
      [x1, y1],
      [x0, y1],
    ]
    return new PolygonLayer({
      id: 'marquee',
      coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
      data: [{ polygon }],
      getPolygon: (d: { polygon: number[][] }) => d.polygon,
      getFillColor: FILL,
      getLineColor: LINE,
      getLineWidth: 1,
      lineWidthUnits: 'pixels',
      stroked: true,
      filled: true,
      pickable: false,
    })
  }, [marquee])
}
