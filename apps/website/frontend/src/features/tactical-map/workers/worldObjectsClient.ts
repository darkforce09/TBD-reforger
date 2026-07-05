// Main-thread client for the world-objects worker (T-090.5.1 skeleton). Lazy spawn on first
// use, torn down on mission unmount via terminateWorldObjects() — same harness as the compiler
// worker (mission-creator/compiler/compilerClient.ts). No call sites mount it this slice
// (WORLDMAP_ENABLED default off; layers arrive T-090.5.2+); chunk hydration RPCs land in
// T-090.5.3.

import * as Comlink from 'comlink'
import type { WorldObjectsStatus, WorldObjectsWorkerApi } from './worldObjects.worker'

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

/** Worker capability status — `ready: false` until chunk streaming ships (T-090.5.3). */
export async function getWorldObjectsStatus(): Promise<WorldObjectsStatus> {
  return getWorldObjects().getStatus()
}
