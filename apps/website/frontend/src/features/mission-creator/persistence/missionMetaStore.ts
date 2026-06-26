// v2 meta store (T-062.1). Everything except slots — meta + the small entity maps
// (factions, squads, editorLayers, objectives, vehicles, markers, loadouts, items) — is
// persisted as one JSON blob, applied with INIT_ORIGIN (mirroring the non-slot path of
// hydrateMissionDocWithProgress). Cheap; lightly debounced + serialized per mission.

import type { MissionDoc } from '../../tactical-map/state/ydoc'
import { INIT_ORIGIN, entityToYMap } from '../../tactical-map/state/ydoc'
import {
  META_STORE,
  PERSIST_SCHEMA_V2,
  metaKey,
  openPersistDb,
} from './missionPersistSchema'
import type { MissionMetaRecord } from './missionPersistSchema'

/** The non-slot entity maps, in the order hydrateMissionDocWithProgress applies them. */
const SMALL_MAPS = [
  'objectives',
  'vehicles',
  'markers',
  'loadouts',
  'factions',
  'squads',
  'editorLayers',
  'items',
] as const

/** Read the meta blob and apply it into the doc in one INIT_ORIGIN transaction. Clears the
 *  small maps first so a re-entrant boot can't double-apply (slots are loaded separately). */
export async function loadMissionMetaIntoDoc(missionId: string, md: MissionDoc): Promise<void> {
  const db = await openPersistDb()
  const record = await db.get(META_STORE, metaKey(missionId))
  if (!record) return
  md.doc.transact(() => {
    for (const name of SMALL_MAPS) md.entities[name].clear()
    if (record.meta) for (const [k, v] of Object.entries(record.meta)) md.meta.set(k, v)
    for (const name of SMALL_MAPS) {
      const rows = record[name]
      if (!rows) continue
      for (const [id, ent] of Object.entries(rows)) {
        if (ent && typeof ent === 'object') {
          md.entities[name].set(id, entityToYMap(ent as Record<string, unknown>))
        }
      }
    }
  }, INIT_ORIGIN)
}

/** Serialize the doc's meta + small maps and rewrite the meta record. The read is one
 *  synchronous pass (no yields), so it's safe as long as it doesn't START after the doc is
 *  destroyed — `isCancelled` guards exactly that for a teardown-time flush. */
export async function saveMissionMetaFromDoc(
  md: MissionDoc,
  missionId: string,
  isCancelled?: () => boolean,
): Promise<void> {
  if (isCancelled?.()) return
  const e = md.entities
  const record: MissionMetaRecord = {
    schemaVersion: PERSIST_SCHEMA_V2,
    meta: md.meta.size > 0 ? (md.meta.toJSON() as Record<string, unknown>) : null,
    factions: e.factions.toJSON() as Record<string, unknown>,
    squads: e.squads.toJSON() as Record<string, unknown>,
    editorLayers: e.editorLayers.toJSON() as Record<string, unknown>,
    objectives: e.objectives.toJSON() as Record<string, unknown>,
    vehicles: e.vehicles.toJSON() as Record<string, unknown>,
    markers: e.markers.toJSON() as Record<string, unknown>,
    loadouts: e.loadouts.toJSON() as Record<string, unknown>,
    items: e.items.toJSON() as Record<string, unknown>,
  }
  const db = await openPersistDb()
  await db.put(META_STORE, record, metaKey(missionId))
}

// ── Debounced + serialized writer ───────────────────────────────────────────
interface PendingMeta {
  md: MissionDoc
  isCancelled?: () => boolean
}
const timers = new Map<string, ReturnType<typeof setTimeout>>()
const pending = new Map<string, PendingMeta>()
const chains = new Map<string, Promise<void>>()

function enqueueMetaSave(p: PendingMeta, missionId: string): Promise<void> {
  const prev = chains.get(missionId) ?? Promise.resolve()
  const next = prev
    .catch(() => {})
    .then(() => saveMissionMetaFromDoc(p.md, missionId, p.isCancelled))
  chains.set(missionId, next)
  void next.finally(() => {
    if (chains.get(missionId) === next) chains.delete(missionId)
  })
  return next
}

export function saveMissionMetaFromDocDebounced(
  md: MissionDoc,
  missionId: string,
  isCancelled?: () => boolean,
  delay = 1000,
): void {
  pending.set(missionId, { md, isCancelled })
  const existing = timers.get(missionId)
  if (existing) clearTimeout(existing)
  timers.set(
    missionId,
    setTimeout(() => {
      timers.delete(missionId)
      const p = pending.get(missionId)
      pending.delete(missionId)
      if (p) void enqueueMetaSave(p, missionId)
    }, delay),
  )
}

/** Flush a pending debounced meta save now and await it settling (tab hidden / unmount). */
export async function flushMeta(missionId: string): Promise<void> {
  const t = timers.get(missionId)
  if (t) {
    clearTimeout(t)
    timers.delete(missionId)
  }
  const p = pending.get(missionId)
  if (p) {
    pending.delete(missionId)
    enqueueMetaSave(p, missionId)
  }
  await chains.get(missionId)
}
