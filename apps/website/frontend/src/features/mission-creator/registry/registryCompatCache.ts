// IDB cache for the registry compat graph (T-068.9) — yrsPersist.ts mechanism.
// One record per modpack (modpacks cache side-by-side; switching never evicts),
// keyed by modpack_id with the server's weak ETag for conditional revalidation
// (If-None-Match -> 304 keeps the cached graph). A 'last' pointer lets a session
// warm-start before it knows the current modpack id. Runs on the main thread or
// in a worker (idb works in both).

import { openDB, type IDBPDatabase } from 'idb'

import type { CompatEdgeTuple } from './registryGraph'

const DB_NAME = 'tbd-registry-compat'
const STORE = 'compat'
const META = 'meta'
const LAST_KEY = 'last'

/** One modpack's cached graph: the ETag it was fetched under + stripped tuples. */
export interface CachedCompat {
  modpack_id: string
  modpack_version: string
  etag: string
  fetched_at: number
  edges: CompatEdgeTuple[]
}

async function db(): Promise<IDBPDatabase> {
  return openDB(DB_NAME, 1, {
    upgrade(d) {
      if (!d.objectStoreNames.contains(STORE)) d.createObjectStore(STORE)
      if (!d.objectStoreNames.contains(META)) d.createObjectStore(META)
    },
  })
}

/** Load a modpack's cached graph, or `null` on miss. */
export async function getCached(modpackId: string): Promise<CachedCompat | null> {
  const d = await db()
  const value = (await d.get(STORE, modpackId)) as CachedCompat | undefined
  return value ?? null
}

/** Persist a modpack's graph and mark it as the most recent one. */
export async function putCached(entry: CachedCompat): Promise<void> {
  const d = await db()
  await d.put(STORE, entry, entry.modpack_id)
  await d.put(META, entry.modpack_id, LAST_KEY)
}

/** The modpack id of the most recently cached graph, or `null`. */
export async function getLastModpackId(): Promise<string | null> {
  const d = await db()
  const value = (await d.get(META, LAST_KEY)) as string | undefined
  return value ?? null
}

/** Drop one modpack's cached graph (clears the 'last' pointer if it matches). */
export async function clearCached(modpackId: string): Promise<void> {
  const d = await db()
  await d.delete(STORE, modpackId)
  if ((await d.get(META, LAST_KEY)) === modpackId) await d.delete(META, LAST_KEY)
}
