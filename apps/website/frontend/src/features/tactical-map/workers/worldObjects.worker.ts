// World-objects Web Worker (T-090.5.1 skeleton). Will own chunk fetch + gunzip
// (DecompressionStream) + catalog/density decode + the world rbush + marching squares,
// returning transferable typed arrays — never 1M JS objects (plan §6 W-transfer rule).
// This slice ships the Comlink plumbing only: a stub API so the client harness, bundling
// (`new Worker(new URL(...), { type: 'module' })` in worldObjectsClient.ts) and teardown
// path are proven before real work lands in T-090.5.3.
//
// Worker-safety: comlink only (pure worldmap/ modules allowed later) — no DOM, no React,
// no barrel imports (pattern: mission-creator/compiler/compiler.worker.ts).

import * as Comlink from 'comlink'

/** Worker lifecycle status — `ready` stays false until chunk streaming exists (T-090.5.3). */
export interface WorldObjectsStatus {
  ready: boolean
}

const api = {
  /** Liveness probe for the client harness + smoke tests. */
  ping(): string {
    return 'world-objects-worker'
  },
  /** Honest capability report: nothing is loaded and nothing can be until T-090.5.3. */
  getStatus(): WorldObjectsStatus {
    return { ready: false }
  },
}

/** RPC surface mirrored by the main-thread client (worldObjectsClient.ts). */
export type WorldObjectsWorkerApi = typeof api

Comlink.expose(api)
