// DEM hillshade overlay (T-091.2, Ultra Plan §4.3 layer #2). Off by default; visible only
// when meta.environment.showHillshade is on AND the terrain DEM is ready. Built on the CPU
// from the T-091.1 meters cache — a Deck BitmapLayer, not a luma.gl GLSL pass (the GPU path
// is aspirational, see engineering_plan §4.2). The full 6400² grid as RGBA would be ~163 MB,
// so the meters cache is downsampled to a ≤1024 px edge before the Horn hillshade build, and
// the layer is memoized on [terrain, show] so a pan/zoom never rebuilds it — only a terrain
// switch or a hillshade toggle does.

import { useMemo } from 'react'
import { BitmapLayer } from '@deck.gl/layers'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import type { TerrainDef } from '../coords/terrains'
import { getDemRasterForOverlay, isDemReady } from '../dem/DemController'

const MAX_EDGE = 1024 // downsample target — max edge of the hillshade raster
const OPACITY = 0.4 // overlay opacity (~40%), light from NW

// Light direction (NW), standard GIS hillshade.
const AZIMUTH_RAD = (315 * Math.PI) / 180
const ALTITUDE_RAD = (45 * Math.PI) / 180
const ZENITH_RAD = Math.PI / 2 - ALTITUDE_RAD

/**
 * Horn-slope hillshade → row-flipped RGBA ImageData (north up). cellMeters is the world
 * spacing of one downsampled pixel (used as the Horn cell size in both axes).
 */
function buildHillshadeImage(
  meters: Float32Array,
  srcW: number,
  srcH: number,
): { image: ImageData; w: number; h: number } {
  const scale = Math.max(1, Math.ceil(Math.max(srcW, srcH) / MAX_EDGE))
  const w = Math.max(1, Math.floor(srcW / scale))
  const h = Math.max(1, Math.floor(srcH / scale))
  const cellMeters = srcW / w // ≈ scale * metersPerPixel; square cells

  // Downsample (stride) into a small meters grid.
  const ds = new Float32Array(w * h)
  for (let y = 0; y < h; y++) {
    const sy = Math.min(srcH - 1, y * scale)
    for (let x = 0; x < w; x++) {
      const sx = Math.min(srcW - 1, x * scale)
      ds[y * w + x] = meters[sy * srcW + sx]
    }
  }

  const image = new ImageData(w, h)
  const data = image.data
  const at = (x: number, y: number) =>
    ds[Math.min(h - 1, Math.max(0, y)) * w + Math.min(w - 1, Math.max(0, x))]

  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const a = at(x - 1, y - 1)
      const b = at(x, y - 1)
      const c = at(x + 1, y - 1)
      const d = at(x - 1, y)
      const f = at(x + 1, y)
      const g = at(x - 1, y + 1)
      const hh = at(x, y + 1)
      const i = at(x + 1, y + 1)
      const dzdx = (c + 2 * f + i - (a + 2 * d + g)) / (8 * cellMeters)
      const dzdy = (g + 2 * hh + i - (a + 2 * b + c)) / (8 * cellMeters)
      const slope = Math.atan(Math.sqrt(dzdx * dzdx + dzdy * dzdy))
      const aspect = Math.atan2(dzdy, -dzdx)
      let hs =
        Math.cos(ZENITH_RAD) * Math.cos(slope) +
        Math.sin(ZENITH_RAD) * Math.sin(slope) * Math.cos(AZIMUTH_RAD - aspect)
      if (hs < 0) hs = 0
      const gray = Math.round(hs * 255)
      // Flip rows so image row 0 = north (raster row 0 = world y=0 = south).
      const o = ((h - 1 - y) * w + x) * 4
      data[o] = gray
      data[o + 1] = gray
      data[o + 2] = gray
      data[o + 3] = 255
    }
  }
  return { image, w, h }
}

/**
 * Hillshade Deck layer, or null when off / DEM not ready. `version` is the DemController
 * state counter (useDemVersion) — including it in the deps rebuilds the layer the moment the
 * DEM becomes ready, so a hillshade toggled on mid-load (or persisted on across a reload) paints
 * without an extra toggle. Memoized otherwise, so a pan/zoom never rebuilds.
 */
export function useDemLayer({
  terrain,
  show,
  version,
}: {
  terrain: TerrainDef
  show: boolean
  version: number
}): BitmapLayer | null {
  return useMemo(() => {
    if (!show || !isDemReady()) return null
    const dem = getDemRasterForOverlay()
    if (!dem) return null
    const { image } = buildHillshadeImage(dem.metersCache, dem.width, dem.height)
    return new BitmapLayer({
      id: 'dem-hillshade',
      coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
      bounds: [0, 0, terrain.width, terrain.height],
      image,
      opacity: OPACITY,
    })
    // `version` is a deliberate rebuild trigger (DEM ready/degraded/reload), not read in the body.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terrain, show, version])
}
