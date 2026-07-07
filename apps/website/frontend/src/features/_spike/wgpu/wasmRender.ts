// Module glue for the map-engine-render wasm pkg (wasm-pack --target web; built by
// `make wasm-render`). Owns the two module-level lifecycle invariants (T-151 plan §S5):
//
// I1 — module init once: the web-target `init()` instantiates a wasm module + memory;
//      calling it twice would create two instances, so the promise is memoized here.
// I3 — at most one live engine per canvas, ever: `RenderEngine.create` calls are serialized
//      through a module-level promise chain, so the React StrictMode interleave
//      (setup₁ → cleanup₁ → setup₂ while create₁ is still awaiting) can never have two
//      engines configuring one canvas concurrently — engine A is freed (I4 in WgpuCanvas)
//      before engine B's surface exists.

import init, { RenderEngine } from '@/wasm/render/map_engine_render'

let wasmReady: Promise<void> | null = null

function ensureWasm(): Promise<void> {
  wasmReady ??= init().then(() => undefined)
  return wasmReady
}

let creationChain: Promise<unknown> = Promise.resolve()

/** Serialized engine creation (invariants I1 + I3). */
export function createEngine(
  canvas: HTMLCanvasElement,
  forceWebgl: boolean,
): Promise<RenderEngine> {
  const next = creationChain
    .catch(() => undefined) // a failed predecessor must not poison the chain
    .then(async () => {
      await ensureWasm()
      return RenderEngine.create(canvas, forceWebgl)
    })
  creationChain = next
  return next
}

/**
 * CSS→device-pixel mapping for the canvas backing store. Must mirror
 * `RenderEngine::resize` bit-for-bit (`js_round(css·dpr).max(1)`, JS `Math.round` = half
 * toward +∞) so the surface configuration and the canvas size always agree — pinned by
 * `deviceSize.test.ts` literals.
 */
export function deviceSize(cssW: number, cssH: number, dpr: number): [number, number] {
  return [Math.max(1, Math.round(cssW * dpr)), Math.max(1, Math.round(cssH * dpr))]
}

/** Wheel deltaY → zoom delta. Feel-tuning constant only — the camera-side `zoom_at`
 *  semantics (cursor-fixed point, clamped band) are property-tested in Rust. */
export const WHEEL_ZOOM_PER_PX = 1 / 500

export type { RenderEngine }
