// Self-contained base map: a vector tactical grid drawn as a LineLayer (Ultra Plan
// §4.3 layer #1, base-map decision: procedural grid). A handful of 1 km gridlines +
// a terrain border is a few dozen segments — crisp at every zoom and effectively
// free, versus a tiled bitmap that floods the view with sub-layers. Zero network.

import { useMemo } from 'react'
import { LineLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import type { TerrainDef } from '../coords/terrains'

const GRID_STEP = 1000 // meters between gridlines (1 km)

// Aegis primary #adc6ff = rgb(173,198,255); faint minor, brighter every 5 km. Two palettes:
// the faint one reads on the dark background; the boosted one keeps the grid legible over the
// ~40%-gray hillshade overlay (T-091.2 follow-up).
const MINOR: [number, number, number, number] = [173, 198, 255, 28]
const MAJOR: [number, number, number, number] = [173, 198, 255, 60]
const BORDER: [number, number, number, number] = [173, 198, 255, 90]
const MINOR_HS: [number, number, number, number] = [173, 198, 255, 80]
const MAJOR_HS: [number, number, number, number] = [173, 198, 255, 150]
const BORDER_HS: [number, number, number, number] = [173, 198, 255, 210]

interface GridLine {
  source: [number, number]
  target: [number, number]
  color: [number, number, number, number]
}

// `visible` toggles the grid via deck's prop rather than adding/removing the layer from the
// array — removing a layer finalizes it, and re-adding the same memoized instance leaves it
// disposed (renders nothing). Always keep it in the layers array; flip `visible` instead.
export function useBaseMapLayer(
  terrain: TerrainDef,
  visible: boolean,
  overHillshade: boolean,
): LineLayer<GridLine> {
  return useMemo(() => {
    const { width, height } = terrain
    const minor = overHillshade ? MINOR_HS : MINOR
    const major = overHillshade ? MAJOR_HS : MAJOR
    const border = overHillshade ? BORDER_HS : BORDER
    const lines: GridLine[] = []

    for (let x = 0; x <= width; x += GRID_STEP) {
      const onBorder = x === 0 || x >= width
      lines.push({
        source: [x, 0],
        target: [x, height],
        color: onBorder ? border : x % 5000 === 0 ? major : minor,
      })
    }
    for (let y = 0; y <= height; y += GRID_STEP) {
      const onBorder = y === 0 || y >= height
      lines.push({
        source: [0, y],
        target: [width, y],
        color: onBorder ? border : y % 5000 === 0 ? major : minor,
      })
    }

    return new LineLayer<GridLine>({
      id: 'base-map-grid',
      coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
      visible,
      data: lines,
      getSourcePosition: (d) => d.source,
      getTargetPosition: (d) => d.target,
      getColor: (d) => d.color,
      getWidth: 1,
      widthUnits: 'pixels',
    })
  }, [terrain, visible, overHillshade])
}
