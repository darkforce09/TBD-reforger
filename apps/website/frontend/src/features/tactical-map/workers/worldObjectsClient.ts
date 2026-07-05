// Main-thread client for the world-objects worker (T-090.5.3). Lazy spawn on first use,
// torn down on mission unmount via terminateWorldObjects() — same harness as the compiler
// worker (mission-creator/compiler/compilerClient.ts). Typed RPC wrappers over the full
// streaming API; chunkStore.ts is the primary consumer (viewport-driven hydration), the
// pick/resolve wrappers are the read-only surface T-090.9 interaction builds on.

import * as Comlink from 'comlink'
import { PICK_RADIUS_PX } from '../worldmap/lodGates'
import type { Bbox } from '../worldmap/chunkMath'
import type {
  ChunkLoadResult,
  LoadChunksOpts,
  ResolvedWorldObject,
  VisibleSet,
  WorldManifestLite,
} from './worldObjectsCore'
import type { WorldObjectsStatus, WorldObjectsWorkerApi } from './worldObjects.worker'

export type { WorldObjectsStatus }

let worker: Worker | null = null
let proxy: Comlink.Remote<WorldObjectsWorkerApi> | null = null

/** Lazily spawn + wrap the worker. Reused across calls within an editor session. */
function getWorldObjects(): Comlink.Remote<WorldObjectsWorkerApi> {
  if (!proxy) {
    worker = new Worker(new URL('./worldObjects.worker.ts', import.meta.url), { type: 'module' })
    proxy = Comlink.wrap<WorldObjectsWorkerApi>(worker)
  }
  return proxy
}

/** Terminate the worker (mission unmount). Safe no-op if never spawned; next call respawns. */
export function terminateWorldObjects(): void {
  worker?.terminate()
  worker = null
  proxy = null
}

/** Liveness probe (smoke). */
export async function pingWorldObjects(): Promise<string> {
  return getWorldObjects().ping()
}

/** Worker capability status — `ready: true` once a terrain manifest is loaded. */
export async function getWorldObjectsStatus(): Promise<WorldObjectsStatus> {
  return getWorldObjects().getStatus()
}

/** Load a terrain's world-object manifest (prefabs + chunk grid); null = no export (R11). */
export async function loadWorldManifest(terrainId: string): Promise<WorldManifestLite | null> {
  return getWorldObjects().loadManifest(terrainId)
}

/** Hydrate chunks for a viewport bbox — typed-array payloads arrive via transferables. */
export async function loadWorldChunksInBbox(
  bbox: Bbox,
  marginCells: number,
  opts: LoadChunksOpts,
): Promise<ChunkLoadResult> {
  return getWorldObjects().loadChunksInBbox(bbox, marginCells, opts)
}

/** Instances visible in a bbox at a zoom, per lodGates class gates (W4, budget-capped). */
export async function worldVisibleInstances(bbox: Bbox, deckZoom: number): Promise<VisibleSet> {
  return getWorldObjects().visibleInstances(bbox, deckZoom)
}

/** Nearest world object within radiusM; pass deckZoom to pick only visible classes (N4). */
export async function pickWorldNearest(
  worldXY: [number, number],
  radiusM: number,
  deckZoom?: number,
): Promise<string | null> {
  return getWorldObjects().pickNearest(worldXY, radiusM, deckZoom)
}

/** World-object ids inside a world-meter rect (read-only marquee analogue). */
export async function pickWorldRect(bbox: Bbox, deckZoom?: number): Promise<string[]> {
  return getWorldObjects().pickRect(bbox, deckZoom)
}

/** Join one instance id back to its prefab identity (hover/inspect data, T-090.9). */
export async function resolveWorldObject(id: string): Promise<ResolvedWorldObject | null> {
  return getWorldObjects().resolve(id)
}

/** Drop the worker's chunks/index/manifest (terrain switch). Worker stays alive. */
export async function unloadWorldObjects(): Promise<void> {
  if (!proxy) return
  return proxy.unload()
}

/** Screen pick radius → world meters at a Deck orthographic zoom (contract N4:
 *  radius = PICK_RADIUS_PX · mpp, mpp = 2^-zoom). Main-thread helper for pick callers. */
export function worldPickRadiusM(deckZoom: number): number {
  return PICK_RADIUS_PX * 2 ** -deckZoom
}
