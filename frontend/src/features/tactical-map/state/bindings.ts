// Reflect Y.Doc changes into the Zustand mirror (Ultra Plan §2.3). observeDeep on
// each top-level map → coalesce a burst of map events (one transaction fires many)
// into a single store update via queueMicrotask → push a plain snapshot. Returns an
// unbind fn. y-indexeddb durability is wired by the wrapper (useMissionDoc), not here.

import type {
  Faction,
  ID,
  InventoryItem,
  Loadout,
  MapMarker,
  MissionMeta,
  Objective,
  Slot,
  Squad,
  Vehicle,
} from './schema'
import { trackedTypes, type MissionDoc } from './ydoc'
import { useMapStore, type MapSnapshot } from './useMapStore'

export function docToSnapshot(md: MissionDoc): MapSnapshot {
  const e = md.entities
  return {
    meta: md.meta.size > 0 ? (md.meta.toJSON() as MissionMeta) : null,
    factionsById: e.factions.toJSON() as Record<ID, Faction>,
    squadsById: e.squads.toJSON() as Record<ID, Squad>,
    slotsById: e.slots.toJSON() as Record<ID, Slot>,
    loadoutsById: e.loadouts.toJSON() as Record<ID, Loadout>,
    itemsById: e.items.toJSON() as Record<ID, InventoryItem>,
    objectivesById: e.objectives.toJSON() as Record<ID, Objective>,
    vehiclesById: e.vehicles.toJSON() as Record<ID, Vehicle>,
    markersById: e.markers.toJSON() as Record<ID, MapMarker>,
  }
}

export function bindStoreToDoc(md: MissionDoc): () => void {
  const maps = trackedTypes(md)

  let scheduled = false
  const flush = () => {
    scheduled = false
    useMapStore.getState()._applySnapshot(docToSnapshot(md))
  }
  const observer = () => {
    if (scheduled) return
    scheduled = true
    queueMicrotask(flush)
  }

  for (const m of maps) m.observeDeep(observer)
  // Prime the store with whatever is already loaded.
  useMapStore.getState()._applySnapshot(docToSnapshot(md))

  return () => {
    for (const m of maps) m.unobserveDeep(observer)
  }
}
