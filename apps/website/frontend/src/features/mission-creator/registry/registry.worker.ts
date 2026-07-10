// Registry compat Web Worker (T-068.9). Owns the IDB cache + the adjacency index
// so canEquip/canAttach answer off the main thread with no per-call graph scan.
// Exposed over Comlink; the main-thread client (registryCompatClient.ts) does the
// authenticated fetch (auth store + single-flight 401 refresh are main-thread
// state) and hands the payload to `adopt`.
//
// Worker-safety: imports only comlink + the pure graph module + the IDB cache
// (idb runs in workers). No DOM, no React, no axios.

import * as Comlink from 'comlink'

import type { RegistryCompatEdge } from '@/types/models/registry'
import { getCached, getLastModpackId, putCached } from './registryCompatCache'
import {
  buildIndex,
  canAttach,
  canEquip,
  hasEdge,
  hostsFor,
  itemsFor,
  stats,
  type CompatEdgeTuple,
  type CompatIndex,
} from './registryGraph'

let index: CompatIndex | null = null
let loadedModpackId: string | null = null

const api = {
  /**
   * Warm-start from IDB: load the given modpack's cached graph (or the most
   * recently cached one when `null`) and index it. Returns the cache identity
   * for the client's conditional GET, or `null` on miss.
   */
  async init(modpackId: string | null): Promise<{ modpackId: string; etag: string } | null> {
    const id = modpackId ?? (await getLastModpackId())
    if (!id) return null
    const hit = await getCached(id)
    if (!hit) return null
    index = buildIndex(hit.edges)
    loadedModpackId = hit.modpack_id
    return { modpackId: hit.modpack_id, etag: hit.etag }
  },

  /**
   * Adopt a fresh API payload: strip rows to tuples (evidence '' ≡ absent),
   * persist to IDB under the modpack, and rebuild the index.
   */
  async adopt(payload: {
    modpack_id: string
    modpack_version: string
    etag: string
    edges: RegistryCompatEdge[]
  }): Promise<{ total: number }> {
    const tuples: CompatEdgeTuple[] = payload.edges.map((e) => ({
      from_node: e.from_node,
      to_node: e.to_node,
      edge_type: e.edge_type,
      ...(e.evidence ? { evidence: e.evidence } : {}),
    }))
    await putCached({
      modpack_id: payload.modpack_id,
      modpack_version: payload.modpack_version,
      etag: payload.etag,
      fetched_at: Date.now(),
      edges: tuples,
    })
    index = buildIndex(tuples)
    loadedModpackId = payload.modpack_id
    return { total: tuples.length }
  },

  canEquip: (item: string, character: string): boolean =>
    index !== null && canEquip(index, item, character),
  canAttach: (item: string, host: string): boolean =>
    index !== null && canAttach(index, item, host),
  hasEdge: (from: string, to: string, type?: string): boolean =>
    index !== null && hasEdge(index, from, to, type),
  itemsFor: (host: string, type?: string): string[] =>
    index === null ? [] : itemsFor(index, host, type),
  hostsFor: (item: string, type?: string): string[] =>
    index === null ? [] : hostsFor(index, item, type),
  stats: (): { modpackId: string; total: number; byType: Record<string, number> } | null =>
    index === null || loadedModpackId === null
      ? null
      : { modpackId: loadedModpackId, ...stats(index) },
}

/** RPC surface mirrored by the main-thread client (registryCompatClient.ts). */
export type RegistryWorkerApi = typeof api

Comlink.expose(api)
