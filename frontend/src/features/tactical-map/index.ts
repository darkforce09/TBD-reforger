// Public barrel — the engine's only import surface. Consumers (the Mission Creator
// today, the Mission Planner later) import from here, never from internal paths.

export { TacticalMap } from './TacticalMap'
export { useMapContext } from './context/MapContext'
export { getTerrain, TERRAINS, DEFAULT_TERRAIN } from './coords/terrains'
export type { TerrainDef, TerrainId } from './coords/terrains'
export type { MapViewState, TacticalMapProps } from './types'

// State foundation (Ultra Plan §2)
export { useMapStore } from './state/useMapStore'
export type { MapStoreState, MapSnapshot } from './state/useMapStore'
export {
  createMissionDoc,
  addSlot,
  moveEntity,
  removeEntity,
  clearAll,
  LOCAL_ORIGIN,
} from './state/ydoc'
export type { MissionDoc, EntityMapName } from './state/ydoc'
export { bindStoreToDoc, docToSnapshot } from './state/bindings'
export { createUndoManager } from './state/undo'
export type { UndoController } from './state/undo'
export { selectSlotIcons } from './state/selectors'
export type { SlotIcon } from './state/selectors'
export type {
  ID,
  Slot,
  Squad,
  Faction,
  Selection,
  SelectionKind,
  ToolId,
  MissionMeta,
} from './state/schema'
