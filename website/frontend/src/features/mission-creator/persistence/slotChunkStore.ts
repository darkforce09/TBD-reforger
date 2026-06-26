// v2 chunked slot store (T-062.1). Slots are persisted as fixed-size chunks and restored
// incrementally — one INIT_ORIGIN transaction per chunk with a yieldToUi between — so the
// load overlay ticks done/total smoothly instead of jumping 0→300k after a single blocking
// replay. Writes are debounced + serialized per mission so a burst of edits coalesces into
// one rewrite and overlapping saves never interleave.

import type { MissionDoc } from '../../tactical-map/state/ydoc'
import { INIT_ORIGIN, entityToYMap } from '../../tactical-map/state/ydoc'
import { yieldToUi } from '../../tactical-map/state/yieldToUi'
import type { Slot } from '../../tactical-map/state/schema'
import {
  PERSIST_CHUNK_SIZE,
  PERSIST_SCHEMA_V2,
  SLOTS_STORE,
  openPersistDb,
  slotsKey,
} from './missionPersistSchema'
import type { MissionSlotsRecord } from './missionPersistSchema'

/** Read the chunked slot record and apply it into the doc incrementally. Each stored chunk
 *  is applied in its own INIT_ORIGIN transaction (not undo-tracked), yielding to the UI
 *  between chunks; `onProgress(done, total)` drives the determinate restore bar. Honors an
 *  optional `isCancelled` so a StrictMode teardown stops applying into a destroyed doc. */
export async function loadSlotsWithProgress(
  missionId: string,
  md: MissionDoc,
  onProgress?: (done: number, total: number) => void,
  isCancelled?: () => boolean,
): Promise<void> {
  const db = await openPersistDb()
  const record = await db.get(SLOTS_STORE, slotsKey(missionId))
  const total = record?.slotCount ?? 0
  onProgress?.(0, total)
  if (!record || total === 0) return

  let done = 0
  for (const chunk of record.chunks) {
    if (isCancelled?.()) return
    md.doc.transact(() => {
      for (const row of chunk.slots) {
        if (row && typeof row.id === 'string') {
          md.entities.slots.set(row.id, entityToYMap(row as unknown as Record<string, unknown>))
        }
      }
    }, INIT_ORIGIN)
    done += chunk.slots.length
    onProgress?.(Math.min(done, total), total)
    await yieldToUi()
  }
  onProgress?.(total, total)
}

/** Export the doc's slots to fixed-size chunks and rewrite the slot record. Yields during
 *  the toJSON sweep so a 360k export doesn't block the main thread; `onProgress` drives the
 *  migration bar. Returns without writing if `isCancelled` trips mid-sweep (teardown). */
export async function saveSlotsFromDoc(
  md: MissionDoc,
  missionId: string,
  onProgress?: (done: number, total: number) => void,
  isCancelled?: () => boolean,
): Promise<void> {
  if (isCancelled?.()) return
  const slots = md.entities.slots
  const total = slots.size
  const chunks: MissionSlotsRecord['chunks'] = []
  let current: Slot[] = []
  let count = 0
  let sinceYield = 0
  onProgress?.(0, total)
  for (const key of slots.keys()) {
    const ym = slots.get(key)
    if (!ym) continue
    current.push(ym.toJSON() as Slot)
    count++
    if (current.length >= PERSIST_CHUNK_SIZE) {
      chunks.push({ index: chunks.length, slots: current })
      current = []
    }
    if (++sinceYield >= PERSIST_CHUNK_SIZE) {
      sinceYield = 0
      onProgress?.(count, total)
      await yieldToUi()
      if (isCancelled?.()) return
    }
  }
  if (current.length > 0) chunks.push({ index: chunks.length, slots: current })

  const record: MissionSlotsRecord = {
    schemaVersion: PERSIST_SCHEMA_V2,
    slotCount: count,
    chunks,
  }
  const db = await openPersistDb()
  await db.put(SLOTS_STORE, record, slotsKey(missionId))
  onProgress?.(count, total)
}

// ── Debounced + serialized writer ───────────────────────────────────────────
// `isCancelled` (= the doc was destroyed, T-062.1) is threaded into every queued save so a
// save that's mid-flight when the editor tears down aborts WITHOUT rewriting a truncated /
// empty record — corruption-safe. Saves are serialized per mission so they never interleave.
interface PendingSlots {
  md: MissionDoc
  isCancelled?: () => boolean
}
const timers = new Map<string, ReturnType<typeof setTimeout>>()
const pending = new Map<string, PendingSlots>()
const chains = new Map<string, Promise<void>>()

function enqueueSlotSave(p: PendingSlots, missionId: string): Promise<void> {
  const prev = chains.get(missionId) ?? Promise.resolve()
  const next = prev
    .catch(() => {})
    .then(() => saveSlotsFromDoc(p.md, missionId, undefined, p.isCancelled))
  chains.set(missionId, next)
  void next.finally(() => {
    if (chains.get(missionId) === next) chains.delete(missionId)
  })
  return next
}

export function saveSlotsFromDocDebounced(
  md: MissionDoc,
  missionId: string,
  isCancelled?: () => boolean,
  delay = 2000,
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
      if (p) void enqueueSlotSave(p, missionId)
    }, delay),
  )
}

/** Flush a pending debounced slot save now and await it settling (tab hidden / unmount).
 *  A queued save still honors its own isCancelled, so an unmount-time flush that races the
 *  doc's destruction aborts rather than persisting garbage. */
export async function flushSlots(missionId: string): Promise<void> {
  const t = timers.get(missionId)
  if (t) {
    clearTimeout(t)
    timers.delete(missionId)
  }
  const p = pending.get(missionId)
  if (p) {
    pending.delete(missionId)
    enqueueSlotSave(p, missionId)
  }
  await chains.get(missionId)
}
