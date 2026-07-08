// T-151.1 — React glue for the wgpu basemap controller. Threads the same mapStyle / layer-toggle
// inputs the Deck path uses (useMapStyle → styleForMode/basemapViewForStyle, useDemVersion) into the
// imperative `WgpuBasemapController` held by `WgpuTacticalMap`. Prop changes fire narrow effects so
// a satellite↔hybrid dim or a hillshade-slider drag re-tints (no reload), while a view switch or a
// DEM landing reloads. The controller lifecycle + camera wiring live in WgpuTacticalMap.

import { useEffect } from 'react'
import type { RefObject } from 'react'
import { useMapStyle } from '../state/worldLayerPrefs'
import { basemapViewForStyle, styleForMode } from '../worldmap/styleModes'
import { useDemVersion } from '../dem/useDemVersion'
import type { TerrainId } from '../coords/terrains'
import type { WgpuBasemapController } from './wgpuBasemap'

export function useWgpuBasemap(
  controllerRef: RefObject<WgpuBasemapController | null>,
  ready: boolean,
  opts: {
    terrainId: TerrainId
    showGrid: boolean
    showHillshade: boolean
    hillshadeOpacity: number
  },
): void {
  const { terrainId, showGrid, showHillshade, hillshadeOpacity } = opts
  const mapStyle = useMapStyle()
  const basemapView = basemapViewForStyle(mapStyle)
  const { satOpacity } = styleForMode(mapStyle)

  const demVersion = useDemVersion()

  // Basemap view: reload only on a VIEW switch (satellite unified/pyramid ↔ map pyramid). The
  // Map pyramid draws at opacity 1; satellite/hybrid start at satOpacity (re-tinted below).
  useEffect(() => {
    if (!ready) return
    void controllerRef.current?.setBasemapView(basemapView, basemapView === 'map' ? 1 : satOpacity)
    // satOpacity excluded on purpose — hybrid dim is a re-tint, not a reload.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, terrainId, basemapView])

  // Paper-tint clear underlay (L8) — map style → cartographic paper, else dark field.
  useEffect(() => {
    if (!ready) return
    controllerRef.current?.setPaperTint(basemapView === 'map')
  }, [ready, basemapView, controllerRef])

  // Hybrid/satellite dim — re-tint the satellite field in place (no reload).
  useEffect(() => {
    if (!ready || basemapView === 'map') return
    controllerRef.current?.setSatOpacity(satOpacity)
  }, [ready, basemapView, satOpacity, controllerRef])

  // Hillshade lane: (re)load on show + when the DEM lands (demVersion), clear on hide.
  useEffect(() => {
    if (!ready) return
    controllerRef.current?.setHillshade(showHillshade, hillshadeOpacity)
    // hillshadeOpacity handled by its own re-tint effect; demVersion re-runs once the DEM is ready.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, terrainId, showHillshade, demVersion])

  // Blend-strength slider — re-tint only (never rebuilds the Horn image, L6).
  useEffect(() => {
    if (!ready) return
    controllerRef.current?.setHillshadeOpacity(hillshadeOpacity, showHillshade)
  }, [ready, hillshadeOpacity, showHillshade, controllerRef])

  // Grid lane — palette depends on whether the hillshade overlay is showing.
  useEffect(() => {
    if (!ready) return
    controllerRef.current?.setGrid(showGrid, showHillshade)
  }, [ready, terrainId, showGrid, showHillshade, controllerRef])
}
