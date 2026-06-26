// Public barrel — the engine's only import surface. Consumers (the Mission Creator
// today, the Mission Planner later) import from here, never from internal paths.

export { TacticalMap } from './TacticalMap'
export { useMapContext } from './context/MapContext'
export { getTerrain, TERRAINS, DEFAULT_TERRAIN } from './coords/terrains'
export type { TerrainDef, TerrainId } from './coords/terrains'
export { ASSET_DND_MIME } from './types'
export type {
  MapViewState,
  TacticalMapProps,
  TacticalMapApi,
  AssetDropPayload,
} from './types'

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
export {
  selectSlotIconsBase,
  selectDragOverlayIcons,
  selectSlotCount,
  selectFactionList,
  selectSquadsOf,
  selectSlotsOf,
} from './state/selectors'
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
