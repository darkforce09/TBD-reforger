// T-090.5.2/.5.3/.8.1 — Map Engine v2 layer assembly hook. Single insertion point for
// world-object layers in TacticalMap's ordered array (plan §4.2 slots: sea → land-cover →
// contours → roads → buildings → forest → trees/props; hillshade + grid stay their own
// hooks). Mounted slots after T-090.8.1: 4 (`world-landcover`), 6–7 (`world-roads*`,
// `world-buildings`, `world-building-badges`) and 8 (`world-forest`,
// `world-forest-outline`). Data paths: buildings stream viewport-driven through the worker
// + chunkStore (T-090.5.3); forest mass streams TBDD density chunks through the same worker
// via forestMassStore (T-090.8.1); roads + land-cover regions are small main-thread
// one-shots (worldData / landCoverRegions).
//
// WORLDMAP_ENABLED off → [] before any work: no fetch, no worker, no layers — first paint
// identical to today (plan risk R3). Visibility authority is lodGates.classVisible only
// (LOD5); the memo keys on the *derived* gate outputs (visible road-class set +
// building/badge/forest booleans + the forest α band) plus the two store revisions, so
// continuous pan/zoom rebuilds layers only at band crossings or when streamed data actually
// changes. The viewport effects run per camera move but both stores early-exit on an
// unchanged chunk set.

import { useEffect, useMemo, useState, useSyncExternalStore } from 'react'
import type { Layer } from '@deck.gl/core'
import { WORLDMAP_ENABLED } from './config'
import { classVisible } from './lodGates'
import { buildRoadLayers, visibleRoadClasses, type RoadClass, type RoadSegment } from './roadLayer'
import { buildBuildingLayer, buildBuildingBadgeLayer } from './buildingLayer'
import { loadWorldRoads } from './worldData'
import { ensureWorldStream, getWorldBuildings, setWorldViewport, subscribeWorldStream } from './chunkStore'
import {
  EMPTY_FOREST_COMPOSITE,
  ensureForestStream,
  getForestMass,
  setForestViewport,
  subscribeForestStream,
} from './forestMassStore'
import { buildForestLayers } from './forestMassLayer'
import { forestFillAlpha } from './forestMass'
import {
  buildLandCoverLayer,
  loadLandCoverRegions,
  type LandCoverRegion,
} from './landCoverRegions'
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
const getEmptyForest = () => EMPTY_FOREST_COMPOSITE
const noop = () => undefined
const noSubscribe = () => noop

// Store bindings resolve once at module load — the flag is build-time constant, so the
// disabled path never subscribes to (or snapshots) either store.
const subscribeBuildings = WORLDMAP_ENABLED ? subscribeWorldStream : noSubscribe
const snapshotBuildings = WORLDMAP_ENABLED ? getWorldBuildings : getEmptyBuildings
const subscribeForest = WORLDMAP_ENABLED ? subscribeForestStream : noSubscribe
const snapshotForest = WORLDMAP_ENABLED ? getForestMass : getEmptyForest

/** Derived LOD gate state — stable within a zoom band (the memo keys on these, never on
 *  raw zoom, so continuous pan/zoom rebuilds layers only at band crossings). */
interface WorldGateState {
  roadClassKey: string
  buildingsVisible: boolean
  badgesVisible: boolean
  forestFillVisible: boolean
  forestOutlineVisible: boolean
  forestAlpha: number
}

const GATES_DISABLED: WorldGateState = {
  roadClassKey: '',
  buildingsVisible: false,
  badgesVisible: false,
  forestFillVisible: false,
  forestOutlineVisible: false,
  forestAlpha: 0,
}

function deriveWorldGates(deckZoom: number): WorldGateState {
  if (!WORLDMAP_ENABLED) return GATES_DISABLED
  return {
    roadClassKey: visibleRoadClasses(deckZoom).join(','),
    buildingsVisible: classVisible('building', deckZoom),
    badgesVisible: classVisible('buildingBadge', deckZoom),
    forestFillVisible: classVisible('forestFill', deckZoom),
    forestOutlineVisible: classVisible('forestOutline', deckZoom),
    forestAlpha: forestFillAlpha(deckZoom),
  }
}

/** Everything the pure slot-order assembly below consumes (all memo deps, no raw zoom). */
interface AssembleOpts {
  toggles: ReturnType<typeof useClassToggles>
  regions: LandCoverRegion[] | null
  roads: RoadSegment[] | null
  roadClassKey: string
  buildings: BuildingInstance[]
  atlas: WorldGlyphAtlas | null
  forestMass: ReturnType<typeof getForestMass>
  forestAlpha: number
  buildingsVisible: boolean
  badgesVisible: boolean
  forestFillVisible: boolean
  forestOutlineVisible: boolean
}

/** Ordered slot assembly (t090_10 stack): 4 land-cover → 6 roads → 7 buildings/badges →
 *  8 forest fill + outline. Pure — the hook memo is a single call. */
function assembleWorldLayers(o: AssembleOpts): Layer[] {
  const layers: Layer[] = []
  if (o.toggles.forest && o.regions) {
    const landcover = buildLandCoverLayer({ regions: o.regions, visible: o.forestFillVisible })
    if (landcover) layers.push(landcover)
  }
  if (o.toggles.roads && o.roads && o.roadClassKey) {
    layers.push(
      ...buildRoadLayers({
        segments: o.roads,
        visibleClasses: o.roadClassKey.split(',') as RoadClass[],
      }),
    )
  }
  if (o.toggles.buildings) {
    const buildingLayer = buildBuildingLayer({ buildings: o.buildings, visible: o.buildingsVisible })
    if (buildingLayer) layers.push(buildingLayer)
    const badges = buildBuildingBadgeLayer({ buildings: o.buildings, atlas: o.atlas, visible: o.badgesVisible })
    if (badges) layers.push(badges)
  }
  if (o.toggles.forest) {
    layers.push(
      ...buildForestLayers({
        mass: o.forestMass,
        fillAlpha: o.forestAlpha,
        fillVisible: o.forestFillVisible,
        outlineVisible: o.forestOutlineVisible,
      }),
    )
  }
  return layers
}

export function useWorldMapLayers({ terrain, deckZoom, viewBounds }: UseWorldMapLayersOpts): Layer[] {
  const toggles = useClassToggles()
  // Roads/regions are tagged with their terrain so a terrain switch derives back to null
  // until the new load resolves (no synchronous reset-setState in the effect).
  const [loadedRoads, setLoadedRoads] = useState<{
    terrainId: string
    roads: RoadSegment[]
  } | null>(null)
  const [loadedRegions, setLoadedRegions] = useState<{
    terrainId: string
    regions: LandCoverRegion[]
  } | null>(null)
  const [atlas, setAtlas] = useState<WorldGlyphAtlas | null>(null)

  // One roads + regions + atlas load per terrain (all module-cached, so re-mounts and
  // StrictMode double-invokes join the same promise), plus both streaming session kick-offs
  // (idempotent per terrain; they share the one worker core). Nothing here re-fires on
  // pan/zoom.
  useEffect(() => {
    if (!WORLDMAP_ENABLED) return
    let alive = true
    ensureWorldStream(terrain)
    ensureForestStream(terrain)
    void loadWorldRoads(terrain).then((roads) => {
      if (alive) setLoadedRoads({ terrainId: terrain.id, roads })
    })
    void loadLandCoverRegions(terrain).then((regions) => {
      if (alive) setLoadedRegions({ terrainId: terrain.id, regions })
    })
    void loadWorldGlyphAtlas().then((a) => {
      if (alive) setAtlas(a)
    })
    return () => {
      alive = false
    }
  }, [terrain])

  // Streamed buildings/forest: each composite reference only changes when its store commits
  // a change, so they double as the useSyncExternalStore snapshots.
  const buildings = useSyncExternalStore(subscribeBuildings, snapshotBuildings, snapshotBuildings)
  const forestMass = useSyncExternalStore(subscribeForest, snapshotForest, snapshotForest)

  // Viewport → streaming. Runs on every camera commit; both stores early-exit when the
  // preloaded chunk set is unchanged, so per-frame cost is chunk math only. Forest density
  // only streams while its toggle is on (nothing fetches what nothing will draw).
  useEffect(() => {
    if (!WORLDMAP_ENABLED) return
    setWorldViewport(viewBounds, deckZoom)
    if (toggles.forest) setForestViewport(viewBounds)
  }, [viewBounds, deckZoom, toggles.forest])

  const roads = loadedRoads?.terrainId === terrain.id ? loadedRoads.roads : null
  const regions = loadedRegions?.terrainId === terrain.id ? loadedRegions.regions : null

  const {
    roadClassKey,
    buildingsVisible,
    badgesVisible,
    forestFillVisible,
    forestOutlineVisible,
    forestAlpha,
  } = deriveWorldGates(deckZoom)

  return useMemo(() => {
    if (!WORLDMAP_ENABLED) return []
    return assembleWorldLayers({
      toggles,
      regions,
      roads,
      roadClassKey,
      buildings,
      atlas,
      forestMass,
      forestAlpha,
      buildingsVisible,
      badgesVisible,
      forestFillVisible,
      forestOutlineVisible,
    })
  }, [
    roads,
    regions,
    buildings,
    forestMass,
    atlas,
    toggles,
    roadClassKey,
    buildingsVisible,
    badgesVisible,
    forestFillVisible,
    forestOutlineVisible,
    forestAlpha,
  ])
}
