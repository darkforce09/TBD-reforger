// T-090.5.2 — Map Engine v2 layer assembly hook. Single insertion point for world-object
// layers in TacticalMap's ordered array (plan §4.2 slots: sea → land-cover → contours →
// roads → buildings → forest → trees/props; hillshade + grid stay their own hooks). This
// slice mounts slots 6–7: `world-roads`, `world-buildings`, `world-building-badges`.
// Later slices (T-090.5.3+ streaming, .5.4 sea/contours, .8.1 forest) extend the array.
//
// WORLDMAP_ENABLED off → [] before any work: no fetch, no layers, first paint identical to
// today (plan risk R3). Visibility authority is lodGates.classVisible only (LOD5); the memo
// keys on the *derived* gate outputs (visible road-class set + building/badge booleans), so
// continuous zoom rebuilds layers only at band crossings, and per-user class toggles
// (worldLayerPrefs, N8) gate each class group.

import { useEffect, useMemo, useState } from 'react'
import type { Layer } from '@deck.gl/core'
import { WORLDMAP_ENABLED } from './config'
import { classVisible } from './lodGates'
import { buildRoadLayer, visibleRoadClasses, type RoadClass } from './roadLayer'
import { buildBuildingLayer, buildBuildingBadgeLayer } from './buildingLayer'
import { loadWorldObjects, type WorldObjectsData } from './worldData'
import { loadWorldGlyphAtlas, type WorldGlyphAtlas } from '../layers/worldGlyphAtlas'
import { useClassToggles } from '../state/worldLayerPrefs'
import type { TerrainDef } from '../coords/terrains'

export interface UseWorldMapLayersOpts {
  terrain: TerrainDef
  deckZoom: number
}

export function useWorldMapLayers({ terrain, deckZoom }: UseWorldMapLayersOpts): Layer[] {
  const toggles = useClassToggles()
  // Loaded data is tagged with its terrain so a terrain switch derives back to null until the
  // new load resolves (no synchronous reset-setState in the effect).
  const [loadedData, setLoadedData] = useState<{
    terrainId: string
    data: WorldObjectsData
  } | null>(null)
  const [atlas, setAtlas] = useState<WorldGlyphAtlas | null>(null)

  // One data load + one atlas load per terrain (both module-cached, so re-mounts and
  // StrictMode double-invokes join the same promise). Nothing here re-fires on pan/zoom.
  useEffect(() => {
    if (!WORLDMAP_ENABLED) return
    let alive = true
    void loadWorldObjects(terrain).then((d) => {
      if (alive) setLoadedData({ terrainId: terrain.id, data: d })
    })
    void loadWorldGlyphAtlas().then((a) => {
      if (alive) setAtlas(a)
    })
    return () => {
      alive = false
    }
  }, [terrain])

  const data = loadedData?.terrainId === terrain.id ? loadedData.data : null

  // Derived LOD gate state — stable within a zoom band (memo deps, not raw zoom).
  const roadClassKey = WORLDMAP_ENABLED ? visibleRoadClasses(deckZoom).join(',') : ''
  const buildingsVisible = WORLDMAP_ENABLED && classVisible('building', deckZoom)
  const badgesVisible = WORLDMAP_ENABLED && classVisible('buildingBadge', deckZoom)

  return useMemo(() => {
    if (!WORLDMAP_ENABLED || !data) return []
    const layers: Layer[] = []
    if (toggles.roads && roadClassKey) {
      const roads = buildRoadLayer({
        segments: data.roads,
        visibleClasses: roadClassKey.split(',') as RoadClass[],
      })
      if (roads) layers.push(roads)
    }
    if (toggles.buildings) {
      const buildings = buildBuildingLayer({ buildings: data.buildings, visible: buildingsVisible })
      if (buildings) layers.push(buildings)
      const badges = buildBuildingBadgeLayer({
        buildings: data.buildings,
        atlas,
        visible: badgesVisible,
      })
      if (badges) layers.push(badges)
    }
    return layers
  }, [data, atlas, toggles, roadClassKey, buildingsVisible, badgesVisible])
}
