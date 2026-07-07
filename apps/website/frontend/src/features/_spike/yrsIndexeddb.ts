// Phase 3.0.d spike — IndexedDB persistence for the yrs update stream (the replacement for
// `y-indexeddb`). The whole document is one `encode_state()` blob keyed by mission id; reload applies
// it into a fresh `MissionDoc`. The cutover will switch to an append-only update log, but the
// round-trip proven here (encode → IDB → apply) is the mechanism.

import { openDB, type IDBPDatabase } from 'idb'

const DB_NAME = 'tbd-doc-core-spike'
const STORE = 'yrs-state'

async function db(): Promise<IDBPDatabase> {
  return openDB(DB_NAME, 1, {
    upgrade(d) {
      if (!d.objectStoreNames.contains(STORE)) d.createObjectStore(STORE)
    },
  })
}

/** Persist a yrs update-stream blob under `id`. */
export async function saveState(id: string, bytes: Uint8Array): Promise<void> {
  const d = await db()
  await d.put(STORE, bytes, id)
}

/** Load a previously-saved blob, or `null` if none. */
export async function loadState(id: string): Promise<Uint8Array | null> {
  const d = await db()
  const value = (await d.get(STORE, id)) as Uint8Array | undefined
  return value ?? null
}

/** Drop the saved blob for `id`. */
export async function clearState(id: string): Promise<void> {
  const d = await db()
  await d.delete(STORE, id)
}
