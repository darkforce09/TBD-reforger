// T-090.5.1 — SHIM over worldLayerPrefs (was the T-090.1 Satellite|Map singleton). The 3-way
// `mapStyle` in state/worldLayerPrefs.ts is the user-facing control now; this module keeps the
// legacy 2-way surface alive for raster consumers (useTerrainBasemapLayer, degraded toasts):
// satellite + hybrid resolve to the 'satellite' raster, 'map' to the legacy Map pyramid.
// worldLayerPrefs migrates the old `tbd-mc-basemap-view` localStorage value on load and
// dual-writes it on change. Deleted (with BasemapView itself) @ T-090.10.2.

import { useSyncExternalStore } from 'react'
import { basemapViewForStyle } from '../worldmap/styleModes'
import { getMapStyle, setMapStyle, subscribeWorldLayerPrefs } from './worldLayerPrefs'

/** Which raster pipeline a style renders — 'satellite' (unified/pyramid) or legacy 'map'. */
export type BasemapView = 'satellite' | 'map'

export function getBasemapView(): BasemapView {
  return basemapViewForStyle(getMapStyle())
}

/** Legacy setter: maps 1:1 onto the matching mapStyle (both values are valid styles). */
export function setBasemapView(v: BasemapView): void {
  setMapStyle(v)
}

/** React hook: current basemap view, re-rendering on any style change. */
export function useBasemapView(): BasemapView {
  return useSyncExternalStore(subscribeWorldLayerPrefs, getBasemapView, getBasemapView)
}
