// Main-thread client for the registry compat worker (T-068.9) — compilerClient.ts
// pattern: one lazily-spawned Worker, torn down via terminateRegistryWorker().
// The fetch stays HERE (not in the worker) so it rides the shared axios client's
// bearer injection + single-flight 401 refresh; the worker owns IDB + the index.
//
// Init flow: worker warm-starts from IDB -> conditional GET with If-None-Match ->
// 304 keeps the cached graph, 200 hands the fresh payload to worker.adopt. The
// graph transfers once per ETag change; queries after that are worker-local.

import * as Comlink from 'comlink'

import { api } from '@/api/client'
import type { RegistryCompatResponse } from '@/types/models/registry'
import type { RegistryWorkerApi } from './registry.worker'

let worker: Worker | null = null
let proxy: Comlink.Remote<RegistryWorkerApi> | null = null
let initFlight: Promise<void> | null = null

/** Lazily spawn + wrap the worker. Reused across queries within a session. */
function getWorker(): Comlink.Remote<RegistryWorkerApi> {
  if (!proxy) {
    worker = new Worker(new URL('./registry.worker.ts', import.meta.url), { type: 'module' })
    proxy = Comlink.wrap<RegistryWorkerApi>(worker)
  }
  return proxy
}

/** Terminate the worker (editor unmount). Safe no-op if never spawned. */
export function terminateRegistryWorker(): void {
  worker?.terminate()
  worker = null
  proxy = null
  initFlight = null
}

async function fetchAndAdopt(modpackId?: string): Promise<void> {
  const w = getWorker()
  const cached = await w.init(modpackId ?? null)
  // Only revalidate with the cached ETag when it belongs to the requested modpack
  // (or when the caller asked for "current", in which case the last-cached graph
  // is the best warm-start we have).
  const conditional = cached && (!modpackId || cached.modpackId === modpackId)
  const res = await api.get<RegistryCompatResponse>('/registry/compat', {
    params: modpackId ? { modpack: modpackId } : undefined,
    headers: conditional ? { 'If-None-Match': cached.etag } : undefined,
    // axios rejects 304 by default; it is a success here (cached graph stands).
    validateStatus: (s) => s === 200 || s === 304,
  })
  if (res.status === 200) {
    await w.adopt({
      modpack_id: res.data.modpack_id,
      modpack_version: res.data.modpack_version,
      etag: res.data.etag,
      edges: res.data.data,
    })
  }
  // 304 -> the worker already indexed the IDB copy in init().
}

/**
 * Load the compat graph for a modpack (default: the current one) into the
 * worker: IDB warm-start + conditional revalidation. Single-flight per session;
 * pass a different modpackId (or call after terminate) to re-init.
 */
export function initRegistryCompat(modpackId?: string): Promise<void> {
  initFlight ??= fetchAndAdopt(modpackId).catch((e: unknown) => {
    initFlight = null // allow a retry after a failed init
    throw e
  })
  return initFlight
}

/** True if `item` is in `character`'s default loadout (T-150 evidence). */
export async function canEquip(item: string, character: string): Promise<boolean> {
  return getWorker().canEquip(item, character)
}

/** True if `item` attaches to / loads into `host` (mag/optic/attachment/ammo families). */
export async function canAttach(item: string, host: string): Promise<boolean> {
  return getWorker().canAttach(item, host)
}

/** Generic edge probe: any family, or exactly `type` when given. */
export async function hasEdge(from: string, to: string, type?: string): Promise<boolean> {
  return getWorker().hasEdge(from, to, type)
}

/** Items a host accepts (sorted; optionally one family) — Forge dropdown feed. */
export async function itemsFor(host: string, type?: string): Promise<string[]> {
  return getWorker().itemsFor(host, type)
}

/** Hosts an item goes in/on (sorted; optionally one family). */
export async function hostsFor(item: string, type?: string): Promise<string[]> {
  return getWorker().hostsFor(item, type)
}

/** Loaded-graph telemetry (null until initRegistryCompat resolves). */
export async function getRegistryCompatStats(): Promise<{
  modpackId: string
  total: number
  byType: Record<string, number>
} | null> {
  return getWorker().stats()
}
