// v2 mission persistence (T-062.1). Replaces y-indexeddb for slot durability so a
// return visit restores slots INCREMENTALLY (chunked + yielding) instead of via the
// single synchronous Y.applyUpdate that froze the boot for 30–60s @ 360k and made the
// load overlay jump 0→300k.
//
// Why not keep y-indexeddb: IndexeddbPersistence syncs the WHOLE Y.Doc, so every slot
// edit re-bloats IDB and the only restore primitive is the all-at-once replay. v2
// durability is instead debounced `idb` writes of (a) a small meta JSON blob and (b) a
// chunked slot store, both keyed per mission. The Y.Doc itself is unchanged — undo still
// rides Y.UndoManager + LOCAL_ORIGIN; load/restore writes use INIT_ORIGIN.

import { openDB } from 'idb'
import type { DBSchema, IDBPDatabase } from 'idb'
import type { Slot } from '../../tactical-map/state/schema'

export const PERSIST_SCHEMA_V2 = 2

/** Slots per chunk — matches hydrateMissionDocWithProgress / docToSnapshotWithProgress. */
export const PERSIST_CHUNK_SIZE = 5000

const DB_NAME = 'tbd-mission-persist'
const DB_VERSION = 1
export const META_STORE = 'meta'
export const SLOTS_STORE = 'slots'

/** The non-slot maps (everything small) serialized as one JSON blob in the meta store. */
export interface MissionMetaRecord {
  schemaVersion: number
  meta: Record<string, unknown> | null
  factions: Record<string, unknown>
  squads: Record<string, unknown>
  editorLayers: Record<string, unknown>
  objectives: Record<string, unknown>
  vehicles: Record<string, unknown>
  markers: Record<string, unknown>
  loadouts: Record<string, unknown>
  items: Record<string, unknown>
}

export interface SlotChunk {
  index: number
  slots: Slot[]
}

export interface MissionSlotsRecord {
  schemaVersion: number
  slotCount: number
  chunks: SlotChunk[]
}

interface PersistDb extends DBSchema {
  [META_STORE]: { key: string; value: MissionMetaRecord }
  [SLOTS_STORE]: { key: string; value: MissionSlotsRecord }
}

let dbPromise: Promise<IDBPDatabase<PersistDb>> | null = null

export function openPersistDb(): Promise<IDBPDatabase<PersistDb>> {
  if (!dbPromise) {
    dbPromise = openDB<PersistDb>(DB_NAME, DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains(META_STORE)) db.createObjectStore(META_STORE)
        if (!db.objectStoreNames.contains(SLOTS_STORE)) db.createObjectStore(SLOTS_STORE)
      },
    })
  }
  return dbPromise
}

export const metaKey = (missionId: string): string => `meta-${missionId}`
export const slotsKey = (missionId: string): string => `slots-${missionId}`

/** The legacy y-indexeddb database name for a mission (v1 persistence). */
export const legacyDbName = (missionId: string): string => `tbd-mission-${missionId}`

/** True once a v2 slot record exists for this mission (the v2-vs-legacy boot signal). */
export async function hasV2Persist(missionId: string): Promise<boolean> {
  try {
    const db = await openPersistDb()
    const key = await db.getKey(SLOTS_STORE, slotsKey(missionId))
    return key !== undefined
  } catch {
    return false
  }
}

/** True if a legacy y-indexeddb DB exists for this mission. Where indexedDB.databases()
 *  is unsupported (older Safari) we conservatively return true: the legacy boot branch
 *  migrates-if-content / seeds-if-empty (writing v2 either way), so a false-positive only
 *  costs one extra replay attempt against an empty DB. */
export async function detectLegacyV1(missionId: string): Promise<boolean> {
  try {
    const idb = indexedDB as IDBFactory & {
      databases?: () => Promise<{ name?: string }[]>
    }
    if (typeof idb.databases !== 'function') return true
    const dbs = await idb.databases()
    return dbs.some((d) => d.name === legacyDbName(missionId))
  } catch {
    return true
  }
}

/** Drop both v2 records for a mission (tests / cleanup). */
export async function deleteV2(missionId: string): Promise<void> {
  try {
    const db = await openPersistDb()
    await Promise.all([
      db.delete(META_STORE, metaKey(missionId)),
      db.delete(SLOTS_STORE, slotsKey(missionId)),
    ])
  } catch {
    /* ignore */
  }
}
