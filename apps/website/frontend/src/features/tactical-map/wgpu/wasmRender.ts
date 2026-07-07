// Module glue for the wgpu `RenderEngine`, now shipped inside the single bundler wasm pkg
// (`make wasm` → `@/wasm/pkg/map_engine_wasm`; T-151.0 merge). Shared by the /_spike/wgpu page and
// the editor's `WgpuTacticalMap`. Owns the one module-level lifecycle invariant that still needs a
// home here (T-151 plan §S5):
//
// I1 — module init once: NO LONGER our job. The bundler target is auto-instantiated by
//      vite-plugin-wasm at import time and ESM guarantees a single module instance, so the old
//      web-target `init()` promise memoization is gone — importing `RenderEngine`/`MissionDoc`
//      anywhere yields the same wasm instance + linear memory (the merge's whole point).
// I3 — at most one live engine per canvas, ever: `RenderEngine.create` calls are serialized
//      through a module-level promise chain, so the React StrictMode interleave
//      (setup₁ → cleanup₁ → setup₂ while create₁ is still awaiting) can never have two engines
//      configuring one canvas concurrently — engine A is freed (I4 in the mount) before engine
//      B's surface exists.

import { RenderEngine } from '@/wasm/pkg/map_engine_wasm'

let creationChain: Promise<unknown> = Promise.resolve()

/** Serialized engine creation (invariant I3). */
export function createEngine(
  canvas: HTMLCanvasElement,
  forceWebgl: boolean,
): Promise<RenderEngine> {
  const next = creationChain
    .catch(() => undefined) // a failed predecessor must not poison the chain
    .then(() => RenderEngine.create(canvas, forceWebgl))
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
