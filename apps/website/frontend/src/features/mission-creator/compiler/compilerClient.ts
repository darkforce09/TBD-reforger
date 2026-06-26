// Main-thread client for the compiler Web Worker (T-066). A single Worker per editor session is
// spawned lazily on the first save/export and torn down on mission unmount (terminateCompiler).
// The API mirrors compile.ts so useMissionEditor's call sites are unchanged — they just run in the
// worker now. onProgress callbacks are wrapped with Comlink.proxy so the worker can call back
// across the boundary with the same (done, total) signature.

import * as Comlink from 'comlink'
import type { MapSnapshot } from '@/features/tactical-map/state/useMapStore'
import type { CompileOptions, MissionPayload } from './compile'
import type { CompilerWorkerApi } from './compiler.worker'

type ProgressFn = (done: number, total: number) => void

let worker: Worker | null = null
let proxy: Comlink.Remote<CompilerWorkerApi> | null = null

/** Lazily spawn + wrap the worker. Reused across saves/exports within an editor session. */
function getCompiler(): Comlink.Remote<CompilerWorkerApi> {
  if (!proxy) {
    worker = new Worker(new URL('./compiler.worker.ts', import.meta.url), { type: 'module' })
    proxy = Comlink.wrap<CompilerWorkerApi>(worker)
  }
  return proxy
}

/** Terminate the worker (mission unmount). Safe no-op if never spawned; getCompiler respawns. */
export function terminateCompiler(): void {
  worker?.terminate()
  worker = null
  proxy = null
}

/** Sync-style compile (Export path) — runs in the worker, returns the full payload incl. orbat[]. */
export async function compileMission(s: MapSnapshot): Promise<MissionPayload> {
  return getCompiler().compileMission(s)
}

/** Chunked compile with progress (Save path). onProgress is proxied across the worker boundary. */
export async function compileMissionWithProgress(
  s: MapSnapshot,
  onProgress?: ProgressFn,
  options?: CompileOptions,
): Promise<MissionPayload> {
  return getCompiler().compileMissionWithProgress(
    s,
    onProgress ? Comlink.proxy(onProgress) : undefined,
    options,
  )
}

/** Build the version-POST request body as a Blob (Save path) — assembled in the worker. */
export async function buildVersionBlob(
  semver: string,
  payload: MissionPayload,
  notes: string,
  onProgress?: ProgressFn,
  chunkSize?: number,
): Promise<Blob> {
  return getCompiler().buildVersionBlob(
    semver,
    payload,
    notes,
    onProgress ? Comlink.proxy(onProgress) : undefined,
    chunkSize,
  )
}
