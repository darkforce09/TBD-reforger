// The Y.Doc — the editor's single source of truth (Ultra Plan §2.3). Top-level
// Y.Maps named meta + the eight entity maps; each entity map is Y.Map<ID, Y.Map>
// (an entity is a nested Y.Map). ID-keyed maps (never Y.Array of objects) make
// concurrent insert/delete/reparent commute — the basis for ADR-3 multiplayer.
//
// All writes go through transact(...) with LOCAL_ORIGIN so (a) one user gesture is
// one undo step and (b) Y.UndoManager can track only local edits (state/undo.ts).

import * as Y from 'yjs'
import { ENTITY_MAPS } from './schema'
import type { ID, Slot } from './schema'

/** Origin tag stamped on every local mutation; tracked by the UndoManager. */
export const LOCAL_ORIGIN = 'local-user'

export type EntityMapName = (typeof ENTITY_MAPS)[number]

export interface MissionDoc {
  doc: Y.Doc
  meta: Y.Map<unknown>
  /** The eight entity maps, each Y.Map<ID, Y.Map>. */
  entities: Record<EntityMapName, Y.Map<Y.Map<unknown>>>
}

export function createMissionDoc(): MissionDoc {
  const doc = new Y.Doc()
  const meta = doc.getMap('meta')
  const entities = {} as Record<EntityMapName, Y.Map<Y.Map<unknown>>>
  for (const name of ENTITY_MAPS) {
    entities[name] = doc.getMap(name) as Y.Map<Y.Map<unknown>>
  }
  return { doc, meta, entities }
}

/** Every shared type the UndoManager / observers should scope to. */
export function trackedTypes(md: MissionDoc): Y.AbstractType<unknown>[] {
  return [md.meta, ...ENTITY_MAPS.map((n) => md.entities[n])] as unknown as Y.AbstractType<unknown>[]
}

/** Run a mutation as a single local transaction (one undo step). */
export function transact(md: MissionDoc, fn: () => void): void {
  md.doc.transact(fn, LOCAL_ORIGIN)
}

/** Plain object -> nested Y.Map (complex fields stored as opaque JSON values). */
function entityToYMap(entity: Record<string, unknown>): Y.Map<unknown> {
  const ym = new Y.Map<unknown>()
  for (const [k, v] of Object.entries(entity)) ym.set(k, v)
  return ym
}

const newId = (): ID =>
  typeof crypto !== 'undefined' && crypto.randomUUID
    ? crypto.randomUUID()
    : `id-${Math.random().toString(36).slice(2)}-${Date.now()}`

// ── Actions ─────────────────────────────────────────────────────────────────
// Each wraps its writes in one transact() so the gesture undoes atomically.

/** Ensure a default faction + squad exist; returns the squad id to attach to. */
export function ensureDefaultSquad(md: MissionDoc): ID {
  const { factions, squads } = md.entities
  let factionId = [...factions.keys()][0]
  if (!factionId) {
    factionId = newId()
    factions.set(
      factionId,
      entityToYMap({ id: factionId, key: 'BLUFOR', name: 'BLUFOR', squadIds: [] }),
    )
  }
  let squadId = [...squads.keys()][0]
  if (!squadId) {
    squadId = newId()
    squads.set(
      squadId,
      entityToYMap({
        id: squadId,
        factionId,
        callsign: 'Test',
        name: 'Test Squad',
        slotIds: [],
      }),
    )
    const faction = factions.get(factionId)!
    faction.set('squadIds', [...(faction.get('squadIds') as ID[]), squadId])
  }
  return squadId
}

/** Add a slot (test unit) at a world position. */
export function addSlot(
  md: MissionDoc,
  position: { x: number; y: number },
): ID {
  let id = ''
  transact(md, () => {
    const squadId = ensureDefaultSquad(md)
    const { slots, squads } = md.entities
    const squad = squads.get(squadId)!
    const slotIds = squad.get('slotIds') as ID[]
    id = newId()
    const slot: Slot = {
      id,
      squadId,
      index: slotIds.length,
      role: 'Rifleman',
      position: { x: position.x, y: position.y, z: 0, rotation: 0 },
      stance: 'stand',
      loadoutId: null,
    }
    slots.set(id, entityToYMap(slot as unknown as Record<string, unknown>))
    squad.set('slotIds', [...slotIds, id])
  })
  return id
}

/** Move any positioned entity (slot/vehicle/objective) to a new world x/y. */
export function moveEntity(
  md: MissionDoc,
  mapName: EntityMapName,
  id: ID,
  position: { x: number; y: number },
): void {
  const entity = md.entities[mapName].get(id)
  if (!entity) return
  transact(md, () => {
    const prev = (entity.get('position') as Record<string, number>) ?? {}
    entity.set('position', { ...prev, x: position.x, y: position.y })
  })
}

/** Remove an entity (and detach a slot from its squad). */
export function removeEntity(
  md: MissionDoc,
  mapName: EntityMapName,
  id: ID,
): void {
  transact(md, () => {
    const map = md.entities[mapName]
    if (mapName === 'slots') {
      const slot = map.get(id)
      const squadId = slot?.get('squadId') as ID | undefined
      const squad = squadId ? md.entities.squads.get(squadId) : undefined
      if (squad) {
        squad.set(
          'slotIds',
          (squad.get('slotIds') as ID[]).filter((s) => s !== id),
        )
      }
    }
    map.delete(id)
  })
}

/** Wipe every entity map (keeps meta). */
export function clearAll(md: MissionDoc): void {
  transact(md, () => {
    for (const name of ENTITY_MAPS) md.entities[name].clear()
  })
}
