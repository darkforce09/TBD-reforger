// T-090.5.1 — Map Engine v2 style modes (implementation plan §4.3). The Satellite|Map radio
// becomes ONE 3-way style control driving satellite-field opacity + vector emphasis — not two
// raster pipelines. Pure decision module: no React, no Deck, node-testable (vitest env rule).
//
// Interim contract (plan Q5): until T-090.5.4 + T-090.8 reach visual parity, `mapStyle: 'map'`
// keeps rendering the legacy `tiles/map/` pyramid via the BasemapView shim below; the paper
// tint + cartographic emphasis only take visual effect when vector layers land (T-090.5.2+).
// No automatic sat↔map zoom crossfade in v1 (plan Q4) — opacity is a pure f(style).

import type { BasemapView } from '../state/basemapView'

/** The 3-way map style (replaces the 2-way BasemapView as the user-facing control). */
export type MapStyle = 'satellite' | 'hybrid' | 'map'

/** How strongly world vector layers assert themselves over the raster (consumed T-090.5.2+). */
export type VectorEmphasis = 'overlay' | 'full' | 'cartographic'

export interface StyleMode {
  /** Opacity of the satellite field (unified texture / sat pyramid). 0 = hidden. */
  satOpacity: number
  /** Flat paper background RGB under vectors in `map` style (plan Q7); null = none. */
  paperTint: [number, number, number] | null
  vectorEmphasis: VectorEmphasis
}

/** Cartographic paper tint (plan Q7: flat color token, no texture asset). Provisional value =
 *  the T-090.1.1.1 open-ground tint #CDC6A3; final visual pass @ T-090.5.4. */
export const PAPER_TINT: [number, number, number] = [0xcd, 0xc6, 0xa3]

const MODES: Record<MapStyle, StyleMode> = {
  satellite: { satOpacity: 1.0, paperTint: null, vectorEmphasis: 'overlay' },
  hybrid: { satOpacity: 0.55, paperTint: null, vectorEmphasis: 'full' },
  map: { satOpacity: 0, paperTint: PAPER_TINT, vectorEmphasis: 'cartographic' },
}

/** mapStyle → render parameters (plan §4.3 mapping, locked). */
export function styleForMode(style: MapStyle): StyleMode {
  return MODES[style]
}

/** Which legacy basemap raster a style renders: satellite + hybrid draw the satellite field
 *  (hybrid just dims it); `map` keeps the legacy Map pyramid until T-090.10.2 retires it. */
export function basemapViewForStyle(style: MapStyle): BasemapView {
  return style === 'map' ? 'map' : 'satellite'
}
