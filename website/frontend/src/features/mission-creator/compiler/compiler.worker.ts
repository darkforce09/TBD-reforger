// Mission compiler Web Worker (T-066). Runs compile + version-blob assembly off the main thread
// so Save Version / Export stay responsive at scale (~367k slots today; ≤10 s @ 1M stretch goal).
// Exposed over Comlink; the main-thread client lives in compilerClient.ts.
//
// Worker-safety: only imports compile.ts (which now imports the worker-safe `coords/terrains` leaf
// and a type-only MapSnapshot — never the Deck/React barrel) and comlink. No DOM, no React, no
// `yieldToUi`. Vite bundles this via `new Worker(new URL('./compiler.worker.ts', import.meta.url),
// { type: 'module' })` in compilerClient.ts.

import * as Comlink from 'comlink'
import { buildVersionBlob, compileMission, compileMissionWithProgress } from './compile'

const api = { compileMission, compileMissionWithProgress, buildVersionBlob }

/** RPC surface mirrored by the main-thread client (compilerClient.ts). */
export type CompilerWorkerApi = typeof api

Comlink.expose(api)
