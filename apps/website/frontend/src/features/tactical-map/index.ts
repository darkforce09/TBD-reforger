// Public barrel — the engine's only import surface. Consumers (the Mission Creator
// today, the Mission Planner later) import from here, never from internal paths.

export { TacticalMap } from './TacticalMap'
export { useMapContext } from './context/MapContext'
export { getTerrain, TERRAINS, DEFAULT_TERRAIN } from './coords/terrains'
export type { TerrainDef, TerrainId } from './coords/terrains'
export { ASSET_DND_MIME } from './types'
export type { MapViewState, TacticalMapProps, TacticalMapApi, AssetDropPayload } from './types'

// DEM elevation (T-091.1)
export { sampleElevation, isDemReady, isDemDegraded, loadDemForTerrain } from './dem'

// Satellite basemap view pref (T-090.1) — per-user, localStorage-backed. Shim over
// worldLayerPrefs since T-090.5.1; deleted @ T-090.10.2.
export { useBasemapView, getBasemapView, setBasemapView } from './state/basemapView'
export type { BasemapView } from './state/basemapView'

// Map Engine v2 style + world-layer prefs (T-090.5.1) — per-user, localStorage-backed.
export { useMapStyle, getMapStyle, setMapStyle } from './state/worldLayerPrefs'
export type { WorldClassToggles } from './state/worldLayerPrefs'
export type { MapStyle } from './worldmap/styleModes'

// State foundation (Ultra Plan §2)
export { useMapStore, pickMapSnapshot } from './state/useMapStore'
export type { MapStoreState, MapSnapshot } from './state/useMapStore'
export {
  createMissionDoc,
  addSlot,
  pasteSlots,
  moveEntity,
  moveEntities,
  removeEntity,
  removeEntities,
  clearAll,
  seedMeta,
  seedDefaultLayer,
  setTitle,
  updateEnvironment,
  updateSlot,
  updateSlotPosition,
  applyMissionRowMeta,
  addFaction,
  addSquad,
  addEditorLayer,
  renameEditorLayer,
  reparentEditorLayer,
  moveSlotToLayer,
  removeEditorLayer,
  ensureDefaultLayer,
  hydrateMissionDoc,
  hydrateMissionDocWithProgress,
  LOCAL_ORIGIN,
} from './state/ydoc'
export type { MissionDoc, EntityMapName } from './state/ydoc'
export {
  bindStoreToDoc,
  docToSnapshot,
  docToSnapshotWithProgress,
  beginBulkSync,
  endBulkSync,
} from './state/bindings'
export { createUndoManager } from './state/undo'
export type { UndoController } from './state/undo'
export { yieldToUi } from './state/yieldToUi'
export { selectDragOverlayIcons } from './state/selectors'
export type { SlotIcon } from './state/selectors'
export type {
  ID,
  Slot,
  ClipboardSlot,
  Squad,
  Faction,
  EditorLayer,
  Loadout,
  Objective,
  Vehicle,
  MapMarker,
  Selection,
  SelectionKind,
  ToolId,
  MissionMeta,
} from './state/schema'
