// Zustand read-mirror of the Y.Doc (Ultra Plan §2.2). Components read via
// useMapStore(selector); they NEVER setState entity data directly — mutations flow
// through the Y.Doc (state/ydoc.ts) and are reflected here by state/bindings.ts.
// Only UI/runtime state (selection, activeTool) is set on the store directly.

import { create } from 'zustand'
import type {
  Faction,
  ID,
  InventoryItem,
  Loadout,
  MapMarker,
  MissionMeta,
  Objective,
  Selection,
  Slot,
  Squad,
  ToolId,
  Vehicle,
} from './schema'

/** The entity-dictionary slice produced from the Y.Doc by bindings. */
export interface MapSnapshot {
  meta: MissionMeta | null
  factionsById: Record<ID, Faction>
  squadsById: Record<ID, Squad>
  slotsById: Record<ID, Slot>
  loadoutsById: Record<ID, Loadout>
  itemsById: Record<ID, InventoryItem>
  objectivesById: Record<ID, Objective>
  vehiclesById: Record<ID, Vehicle>
  markersById: Record<ID, MapMarker>
}

export interface MapStoreState extends MapSnapshot {
  // UI / runtime (not persisted to json_payload)
  selection: Selection
  activeTool: ToolId

  // Internal: bindings push a fresh snapshot here on every Y.Doc change.
  _applySnapshot: (snapshot: MapSnapshot) => void
  setSelection: (selection: Selection) => void
  setActiveTool: (tool: ToolId) => void
  reset: () => void
}

const EMPTY_SNAPSHOT: MapSnapshot = {
  meta: null,
  factionsById: {},
  squadsById: {},
  slotsById: {},
  loadoutsById: {},
  itemsById: {},
  objectivesById: {},
  vehiclesById: {},
  markersById: {},
}

const NO_SELECTION: Selection = { kind: 'none', id: null }

export const useMapStore = create<MapStoreState>()((set) => ({
  ...EMPTY_SNAPSHOT,
  selection: NO_SELECTION,
  activeTool: 'select',
  _applySnapshot: (snapshot) => set(snapshot),
  setSelection: (selection) => set({ selection }),
  setActiveTool: (activeTool) => set({ activeTool }),
  reset: () => set({ ...EMPTY_SNAPSHOT, selection: NO_SELECTION }),
}))
