// T-145 Phase 3.2 flip F2 — v3 mission persistence: the yrs doc-core's durability layer, replacing
// the v2 chunked slot store + y-indexeddb (both Y.Doc-shaped). The whole document is one
// `encode_state()` blob keyed by mission id (the 3.0.d `_spike/yrsIndexeddb.ts` mechanism, operator-
// verified: Save 156 MB → Reload 1M in 3.89 s). Legacy v1/v2 drafts are dropped (re-hydrate from the
// server) — a fresh, distinct DB name, no migration.
//
// Whole-blob writes are heavy at scale, so the debounced writer is IDLE-gated (a longer debounce than
// v2's 2 s: each edit resets the timer, so a burst coalesces into one write) plus an explicit
// `flushState` on tab-hide / Save. Writes are serialized per mission so they never interleave, and an
// `isCancelled` (the shell was detached / doc destroyed) aborts a queued write before it reads an
// empty blob. Isolated in F2: nothing imports this yet.

import { openDB, type IDBPDatabase } from 'idb'

const DB_NAME = 'tbd-mission-yrs' // v3; distinct from v1 `tbd-mission-${id}` + v2 `tbd-mission-persist`.
const STORE = 'doc-state'

async function db(): Promise<IDBPDatabase> {
  return openDB(DB_NAME, 1, {
    upgrade(d) {
      if (!d.objectStoreNames.contains(STORE)) d.createObjectStore(STORE)
    },
  })
}

/** Persist a mission's whole yrs update-stream blob. */
export async function saveState(missionId: string, bytes: Uint8Array): Promise<void> {
  const d = await db()
  await d.put(STORE, bytes, missionId)
}

/** Load a mission's saved blob, or `null` if none. Apply into a fresh doc via `wasm.apply_update`. */
export async function loadState(missionId: string): Promise<Uint8Array | null> {
  const d = await db()
  const value = (await d.get(STORE, missionId)) as Uint8Array | undefined
  return value ?? null
}

/** Drop a mission's saved blob (e.g. before a conflict re-adopt). */
export async function clearState(missionId: string): Promise<void> {
  const d = await db()
  await d.delete(STORE, missionId)
}

// ── Debounced + serialized writer ─────────────────────────────────────────────
// `getBytes` is called at WRITE time (not queue time) so the debounce coalesces to the latest doc
// state; `isCancelled` guards against reading a detached shell (empty blob → truncated record).
interface PendingSave {
  getBytes: () => Uint8Array
  isCancelled?: () => boolean
}
const timers = new Map<string, ReturnType<typeof setTimeout>>()
const pending = new Map<string, PendingSave>()
const chains = new Map<string, Promise<void>>()

function enqueue(missionId: string, p: PendingSave): Promise<void> {
  const prev = chains.get(missionId) ?? Promise.resolve()
  const next = prev
    .catch(() => {
      // Keep the chain alive regardless of a prior link's outcome (that link logged its own failure).
    })
    .then(async () => {
      if (p.isCancelled?.()) return
      const bytes = p.getBytes() // synchronous, after the cancel check → reads a live handle
      if (bytes.length === 0) return // detached / empty → never overwrite a good record with nothing
      await saveState(missionId, bytes)
    })
    .catch((e: unknown) => {
      // Dev diagnostic; F3 wires the user-facing autosave toast (console.error is lint-allowed).
      if (import.meta.env.DEV) console.error('[yrs-persist] save failed', e)
    })
  chains.set(missionId, next)
  void next.finally(() => {
    if (chains.get(missionId) === next) chains.delete(missionId)
  })
  return next
}

/** Queue an idle-gated save: resets a `delay`-ms timer on each call, so a burst of edits writes once
 *  after the edits settle. Reads the latest bytes at write time. */
export function saveStateDebounced(
  missionId: string,
  getBytes: () => Uint8Array,
  isCancelled?: () => boolean,
  delay = 5000,
): void {
  pending.set(missionId, { getBytes, isCancelled })
  const existing = timers.get(missionId)
  if (existing) clearTimeout(existing)
  timers.set(
    missionId,
    setTimeout(() => {
      timers.delete(missionId)
      const p = pending.get(missionId)
      pending.delete(missionId)
      if (p) void enqueue(missionId, p)
    }, delay),
  )
}

/** Flush a pending debounced save now and await it settling (tab hidden / Save / unmount). Honors the
 *  pending save's `isCancelled`, so an unmount-time flush that races the shell's detach aborts rather
 *  than writing an empty blob. */
export async function flushState(missionId: string): Promise<void> {
  const t = timers.get(missionId)
  if (t) {
    clearTimeout(t)
    timers.delete(missionId)
  }
  const p = pending.get(missionId)
  if (p) {
    pending.delete(missionId)
    enqueue(missionId, p)
  }
  await chains.get(missionId)
}
