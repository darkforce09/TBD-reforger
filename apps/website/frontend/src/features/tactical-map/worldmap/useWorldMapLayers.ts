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
import { classVisible, contourIntervalForZoom } from './lodGates'
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
import {
  EMPTY_CONTOURS,
  ensureDemVectors,
  getDemVectors,
  setContourInterval,
  subscribeDemVectors,
  type DemVectorSnapshot,
} from './demVectorStore'
import { buildForestLayers } from './forestMassLayer'
import { buildSeaBandLayer } from './seaBandLayer'
import { buildContourLayer } from './contourLayer'
import { forestFillAlpha } from './forestMass'
import { EMPTY_SEA_BAND, seaFillAlpha } from './seaBand'
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

/** Split return: `sea` layers mount ABOVE the satellite basemap but BELOW dem-hillshade;
 *  `world` layers mount above hillshade (TacticalMap splices the two groups). */
export interface WorldMapLayers {
  sea: Layer[]
  world: Layer[]
}

export interface UseWorldMapLayersOpts {
  terrain: TerrainDef
  deckZoom: number
  /** Visible world AABB (TacticalMap's basemap bounds); null before first measure. */
  viewBounds: Bbox | null
}

const EMPTY_BUILDINGS: BuildingInstance[] = []
const EMPTY_DEM_VECTORS: DemVectorSnapshot = { seaBand: EMPTY_SEA_BAND, contours: EMPTY_CONTOURS }
const getEmptyBuildings = () => EMPTY_BUILDINGS
const getEmptyForest = () => EMPTY_FOREST_COMPOSITE
const getEmptyDemVectors = () => EMPTY_DEM_VECTORS
const noop = () => undefined
const noSubscribe = () => noop

// Store bindings resolve once at module load — the flag is build-time constant, so the
// disabled path never subscribes to (or snapshots) any store.
const subscribeBuildings = WORLDMAP_ENABLED ? subscribeWorldStream : noSubscribe
const snapshotBuildings = WORLDMAP_ENABLED ? getWorldBuildings : getEmptyBuildings
const subscribeForest = WORLDMAP_ENABLED ? subscribeForestStream : noSubscribe
const snapshotForest = WORLDMAP_ENABLED ? getForestMass : getEmptyForest
const subscribeDemVec = WORLDMAP_ENABLED ? subscribeDemVectors : noSubscribe
const snapshotDemVec = WORLDMAP_ENABLED ? getDemVectors : getEmptyDemVectors

/** Derived LOD gate state — stable within a zoom band (the memo keys on these, never on
 *  raw zoom, so continuous pan/zoom rebuilds layers only at band crossings). */
interface WorldGateState {
  roadClassKey: string
  buildingsVisible: boolean
  badgesVisible: boolean
  forestFillVisible: boolean
  forestOutlineVisible: boolean
  forestAlpha: number
  seaVisible: boolean
  seaAlpha: number
  contourVisible: boolean
  contourIntervalM: number
}

const GATES_DISABLED: WorldGateState = {
  roadClassKey: '',
  buildingsVisible: false,
  badgesVisible: false,
  forestFillVisible: false,
  forestOutlineVisible: false,
  forestAlpha: 0,
  seaVisible: false,
  seaAlpha: 0,
  contourVisible: false,
  contourIntervalM: 0,
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
    seaVisible: classVisible('sea', deckZoom),
    seaAlpha: seaFillAlpha(deckZoom),
    contourVisible: classVisible('contour', deckZoom),
    contourIntervalM: contourIntervalForZoom(deckZoom),
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
  demVectors: DemVectorSnapshot
  seaVisible: boolean
  seaAlpha: number
  contourVisible: boolean
}

/** Ordered slot assembly (t090_10 stack). `sea` (slot 2) is split out for TacticalMap to mount
 *  below hillshade; `world` runs 4 land-cover → 5 contours → 6 roads → 7 buildings/badges →
 *  8 forest fill + outline. Pure — the hook memo is a single call. */
function assembleWorldLayers(o: AssembleOpts): WorldMapLayers {
  const sea: Layer[] = []
  if (o.toggles.sea) {
    const seaLayer = buildSeaBandLayer({
      geometry: o.demVectors.seaBand,
      visible: o.seaVisible,
      fillAlpha: o.seaAlpha,
    })
    if (seaLayer) sea.push(seaLayer)
  }

  const world: Layer[] = []
  if (o.toggles.forest && o.regions) {
    const landcover = buildLandCoverLayer({ regions: o.regions, visible: o.forestFillVisible })
    if (landcover) world.push(landcover)
  }
  if (o.toggles.contours) {
    const contour = buildContourLayer({ contours: o.demVectors.contours, visible: o.contourVisible })
    if (contour) world.push(contour)
  }
  if (o.toggles.roads && o.roads && o.roadClassKey) {
    world.push(
      ...buildRoadLayers({
        segments: o.roads,
        visibleClasses: o.roadClassKey.split(',') as RoadClass[],
      }),
    )
  }
  if (o.toggles.buildings) {
    const buildingLayer = buildBuildingLayer({ buildings: o.buildings, visible: o.buildingsVisible })
    if (buildingLayer) world.push(buildingLayer)
    const badges = buildBuildingBadgeLayer({ buildings: o.buildings, atlas: o.atlas, visible: o.badgesVisible })
    if (badges) world.push(badges)
  }
  if (o.toggles.forest) {
    world.push(
      ...buildForestLayers({
        mass: o.forestMass,
        fillAlpha: o.forestAlpha,
        fillVisible: o.forestFillVisible,
        outlineVisible: o.forestOutlineVisible,
      }),
    )
  }
  return { sea, world }
}

export function useWorldMapLayers({ terrain, deckZoom, viewBounds }: UseWorldMapLayersOpts): WorldMapLayers {
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

  // Streamed buildings/forest + DEM-vector (sea/contour) composites: each reference only
  // changes when its store commits, so they double as the useSyncExternalStore snapshots.
  const buildings = useSyncExternalStore(subscribeBuildings, snapshotBuildings, snapshotBuildings)
  const forestMass = useSyncExternalStore(subscribeForest, snapshotForest, snapshotForest)
  const demVectors = useSyncExternalStore(subscribeDemVec, snapshotDemVec, snapshotDemVec)

  // Viewport → streaming. Runs on every camera commit; both stores early-exit when the
  // preloaded chunk set is unchanged, so per-frame cost is chunk math only. Forest density
  // only streams while its toggle is on (nothing fetches what nothing will draw).
  useEffect(() => {
    if (!WORLDMAP_ENABLED) return
    setWorldViewport(viewBounds, deckZoom)
    if (toggles.forest) setForestViewport(viewBounds)
  }, [viewBounds, deckZoom, toggles.forest])

  const {
    roadClassKey,
    buildingsVisible,
    badgesVisible,
    forestFillVisible,
    forestOutlineVisible,
    forestAlpha,
    seaVisible,
    seaAlpha,
    contourVisible,
    contourIntervalM,
  } = deriveWorldGates(deckZoom)

  // DEM-vector geometry (sea + contours) — kicked once per terrain when either toggle is on
  // (idempotent; the store waits on the DEM and produces whole-island static geometry, so
  // nothing here is viewport-driven). Re-runs if a toggle flips on later.
  useEffect(() => {
    if (!WORLDMAP_ENABLED) return
    if (toggles.sea || toggles.contours) ensureDemVectors(terrain)
  }, [terrain, toggles.sea, toggles.contours])

  // Contour interval follows the zoom band (not raw zoom): the store caches per interval and
  // keeps the previous composite until the new one commits, so a band crossing never blanks.
  useEffect(() => {
    if (!WORLDMAP_ENABLED || !toggles.contours) return
    setContourInterval(contourIntervalM)
  }, [contourIntervalM, toggles.contours])

  const roads = loadedRoads?.terrainId === terrain.id ? loadedRoads.roads : null
  const regions = loadedRegions?.terrainId === terrain.id ? loadedRegions.regions : null

  return useMemo(() => {
    if (!WORLDMAP_ENABLED) return { sea: [], world: [] }
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
      demVectors,
      seaVisible,
      seaAlpha,
      contourVisible,
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
    demVectors,
    seaVisible,
    seaAlpha,
    contourVisible,
  ])
}
