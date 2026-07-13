// T-154 doll-engine glue — mirrors wasmRender.ts invariants for the SECOND engine instance:
// the wasm module is an ESM singleton (importing DollEngine reuses the map's instance), and
// creation is serialized through a module-level chain so the StrictMode setup→cleanup→setup
// interleave can never run two creates concurrently (I3). D5: no policy here.

import { DollEngine } from '@/wasm/pkg/map_engine_wasm'

export type { DollEngine }
export { deviceSize } from '@/features/tactical-map/wgpu/wasmRender'

let creationChain: Promise<unknown> = Promise.resolve()

/** Serialized `DollEngine.create` — a failed predecessor cannot poison the chain. */
export function createDollEngine(
  canvas: HTMLCanvasElement,
  forceWebgl = false,
): Promise<DollEngine> {
  const next = creationChain.catch(() => undefined).then(() => DollEngine.create(canvas, forceWebgl))
  creationChain = next
  return next
}
