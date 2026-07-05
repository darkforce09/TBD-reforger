// T-090.5.2/.5.3 — Map Engine v2 layer assembly hook. Single insertion point for world-object
// layers in TacticalMap's ordered array (plan §4.2 slots: sea → land-cover → contours →
// roads → buildings → forest → trees/props; hillshade + grid stay their own hooks). Mounted
// slots stay 6–7 (`world-roads`, `world-buildings`, `world-building-badges` — NO new layers
// in .5.3); what changed is the data path: buildings stream viewport-driven through the
// worker + chunkStore (T-090.5.3) instead of the removed fetch-all loader, roads stay a
// one-shot main-thread load (worldData).
//
// WORLDMAP_ENABLED off → [] before any work: no fetch, no worker, no layers — first paint
// identical to today (plan risk R3). Visibility authority is lodGates.classVisible only
// (LOD5); the memo keys on the *derived* gate outputs (visible road-class set +
// building/badge booleans) plus the chunkStore revision, so continuous pan/zoom rebuilds
// layers only at band crossings or when streamed data actually changes. The viewport effect
// runs per camera move but setWorldViewport early-exits on an unchanged chunk set.

import { useEffect, useMemo, useState, useSyncExternalStore } from 'react'
import type { Layer } from '@deck.gl/core'
import { WORLDMAP_ENABLED } from './config'
import { classVisible } from './lodGates'
import { buildRoadLayers, visibleRoadClasses, type RoadClass, type RoadSegment } from './roadLayer'
import { buildBuildingLayer, buildBuildingBadgeLayer } from './buildingLayer'
import { loadWorldRoads } from './worldData'
import { ensureWorldStream, getWorldBuildings, setWorldViewport, subscribeWorldStream } from './chunkStore'
import type { BuildingInstance } from './buildingLayer'
import { loadWorldGlyphAtlas, type WorldGlyphAtlas } from '../layers/worldGlyphAtlas'
import { useClassToggles } from '../state/worldLayerPrefs'
import type { TerrainDef } from '../coords/terrains'
import type { Bbox } from './chunkMath'

export interface UseWorldMapLayersOpts {
  terrain: TerrainDef
  deckZoom: number
  /** Visible world AABB (TacticalMap's basemap bounds); null before first measure. */
  viewBounds: Bbox | null
}

const EMPTY_BUILDINGS: BuildingInstance[] = []
const getEmptyBuildings = () => EMPTY_BUILDINGS
const noop = () => undefined
const noSubscribe = () => noop

export function useWorldMapLayers({ terrain, deckZoom, viewBounds }: UseWorldMapLayersOpts): Layer[] {
  const toggles = useClassToggles()
  // Roads are tagged with their terrain so a terrain switch derives back to null until the
  // new load resolves (no synchronous reset-setState in the effect).
  const [loadedRoads, setLoadedRoads] = useState<{
    terrainId: string
    roads: RoadSegment[]
  } | null>(null)
  const [atlas, setAtlas] = useState<WorldGlyphAtlas | null>(null)

  // One roads load + one atlas load per terrain (both module-cached, so re-mounts and
  // StrictMode double-invokes join the same promise), plus the streaming session kick-off
  // (idempotent per terrain). Nothing here re-fires on pan/zoom.
  useEffect(() => {
    if (!WORLDMAP_ENABLED) return
    let alive = true
    ensureWorldStream(terrain)
    void loadWorldRoads(terrain).then((roads) => {
      if (alive) setLoadedRoads({ terrainId: terrain.id, roads })
    })
    void loadWorldGlyphAtlas().then((a) => {
      if (alive) setAtlas(a)
    })
    return () => {
      alive = false
    }
  }, [terrain])

  // Streamed buildings: the composite array reference only changes when the chunk store
  // commits a drain/evict/pin change, so it doubles as the useSyncExternalStore snapshot.
  const buildings = useSyncExternalStore(
    WORLDMAP_ENABLED ? subscribeWorldStream : noSubscribe,
    WORLDMAP_ENABLED ? getWorldBuildings : getEmptyBuildings,
    WORLDMAP_ENABLED ? getWorldBuildings : getEmptyBuildings,
  )

  // Viewport → streaming. Runs on every camera commit; the store early-exits when the
  // preloaded chunk set is unchanged, so per-frame cost is chunk math only.
  useEffect(() => {
    if (!WORLDMAP_ENABLED) return
    setWorldViewport(viewBounds, deckZoom)
  }, [viewBounds, deckZoom])

  const roads = loadedRoads?.terrainId === terrain.id ? loadedRoads.roads : null

  // Derived LOD gate state — stable within a zoom band (memo deps, not raw zoom).
  const roadClassKey = WORLDMAP_ENABLED ? visibleRoadClasses(deckZoom).join(',') : ''
  const buildingsVisible = WORLDMAP_ENABLED && classVisible('building', deckZoom)
  const badgesVisible = WORLDMAP_ENABLED && classVisible('buildingBadge', deckZoom)

  return useMemo(() => {
    if (!WORLDMAP_ENABLED) return []
    const layers: Layer[] = []
    if (toggles.roads && roads && roadClassKey) {
      layers.push(
        ...buildRoadLayers({
          segments: roads,
          visibleClasses: roadClassKey.split(',') as RoadClass[],
        }),
      )
    }
    if (toggles.buildings) {
      const buildingLayer = buildBuildingLayer({ buildings, visible: buildingsVisible })
      if (buildingLayer) layers.push(buildingLayer)
      const badges = buildBuildingBadgeLayer({ buildings, atlas, visible: badgesVisible })
      if (badges) layers.push(badges)
    }
    return layers
  }, [roads, buildings, atlas, toggles, roadClassKey, buildingsVisible, badgesVisible])
}
