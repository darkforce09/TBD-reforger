// Mission compiler (Ultra Plan §8) — traverse the normalized state mirror into the
// `json_payload` SUPERSET saved to a MissionVersion. The `orbat[]` block matches exactly
// what internal/handlers/events.go `parseOrbatTemplate` reads (faction/callsign/squad +
// slots[{role,loadout,tag}]), so attaching the mission to an event still auto-builds ORBAT.
// Everything else is additive. An `editor` block carries the full normalized graph (positions,
// folders) the backend ignores but the editor reloads losslessly — orbat[] alone has no
// coordinates. Pure + synchronous (a few hundred entities → sub-ms; no worker needed).

import { getTerrain, type MapSnapshot } from '@/features/tactical-map'

export interface OrbatSlot {
  role: string
  loadout: string
  tag: string
}
export interface OrbatSquad {
  faction: string
  callsign: string
  squad: string
  slots: OrbatSlot[]
}

export interface MissionPayload {
  schemaVersion: 1
  map: { terrain: string; bounds: [number, number, number, number] }
  environment: Record<string, unknown>
  orbat: OrbatSquad[]
  loadouts: Record<string, unknown>
  objectives: unknown[]
  vehicles: unknown[]
  markers: unknown[]
  /** Editor-only fidelity block — ignored by the backend ORBAT parser. */
  editor: {
    factions: unknown[]
    squads: unknown[]
    slots: unknown[]
    editorLayers: unknown[]
  }
}

/** Compile the store snapshot (useMapStore.getState()) into the json_payload superset. */
export function compileMission(s: MapSnapshot): MissionPayload {
  const terrainId = s.meta?.terrain ?? 'everon'
  const terrain = getTerrain(terrainId)

  // ORBAT (backend contract): factions → squads → slots, in authored order.
  const orbat: OrbatSquad[] = Object.values(s.factionsById).flatMap((faction) =>
    faction.squadIds
      .map((sid) => s.squadsById[sid])
      .filter(Boolean)
      .map((squad) => ({
        faction: faction.key,
        callsign: squad.callsign ?? '',
        squad: squad.name,
        slots: squad.slotIds
          .map((slid) => s.slotsById[slid])
          .filter(Boolean)
          .sort((a, b) => a.index - b.index)
          .map((slot) => ({
            role: slot.role,
            loadout: '', // resolved loadout name lands with the Arsenal (Phase 6)
            tag: slot.tag ?? '',
          })),
      })),
  )

  return {
    schemaVersion: 1,
    map: { terrain: terrainId, bounds: [0, 0, terrain.width, terrain.height] },
    environment: { ...(s.meta?.environment ?? {}) },
    orbat,
    loadouts: { ...s.loadoutsById },
    objectives: Object.values(s.objectivesById),
    vehicles: Object.values(s.vehiclesById),
    markers: Object.values(s.markersById),
    editor: {
      factions: Object.values(s.factionsById),
      squads: Object.values(s.squadsById),
      slots: Object.values(s.slotsById),
      editorLayers: Object.values(s.editorLayersById),
    },
  }
}
