// T-151.0 editor dual mount (program D3) — renders the wgpu `RenderEngine`'s calibration scene
// full-bleed inside the Mission Creator shell, selected by `MissionCreatorPage` behind the engine
// flag (`VITE_MC_ENGINE=wgpu` or `?engine=wgpu`). This slice draws ONLY the existing calibration
// scene — no basemap, world objects, or slots (those are T-151.1+). Its job is to prove the engine
// mounts in the editor and that `MissionDoc` + `RenderEngine` share ONE wasm linear memory (the
// shared-memory HUD line, spec L10).
//
// Wasm memory lifecycle invariants I2–I7 are reused VERBATIM from WgpuCanvas.tsx (the spike page):
//   I2 the engine handle (and the proof doc) are EFFECT-LOCAL — never useMemo/useState-persisted;
//      `.free()` is NOT idempotent (see the wasm-react-lifecycle memory).
//   I4 free exactly once on every path: cleanup frees iff committed; the async create path checks
//      `disposed` right after the await and frees-without-committing.
//   I5 no render-after-free: the rAF loop re-checks the flag; rAF is cancelled before free.
//   I6 errors surface into the banner, never swallowed.
//   I7 retry = a NEW canvas element (React key bump) forced to WebGL2 — a canvas permanently
//      commits to its first getContext kind, so same-canvas backend retry is impossible.
// (I1 module-init once + I3 creation mutex live in ./wgpu/wasmRender.)

import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react'
import { MissionDoc } from '@/wasm/pkg/map_engine_wasm'
// `memory` lives on the internal *_bg.wasm module (ESM-deduped to the same instance the engine and
// MissionDoc use), so a Float32Array over its buffer aliases the doc's live slot SoA — the numeric
// proof that the merged pkg is ONE linear memory (spec L10; pattern from DocCoreSpikePage.tsx).
import * as wasmBg from '@/wasm/pkg/map_engine_wasm_bg.wasm'
import { WHEEL_ZOOM_PER_PX, createEngine, deviceSize, type RenderEngine } from './wgpu/wasmRender'
import type { TacticalMapProps } from './types'

const HUD_INTERVAL_MS = 250
// Shared-memory proof (spec L10): seed 1000 slots on a 12800² world; the interleaved xy column is
// then 2000 floats, each expected finite ∧ ≥ 0 ∧ ≤ 12800.
const PROOF_SLOTS = 1000
const PROOF_WORLD = 12_800
const PROOF_SEED = 0x12345678

/** Run the L10 numeric shared-memory check once; returns the HUD line. Frees its own doc. */
function sharedMemoryProof(): string {
  const doc = new MissionDoc()
  try {
    doc.seed_random(PROOF_SLOTS, PROOF_WORLD, PROOF_WORLD, PROOF_SEED)
    doc.refresh()
    const xy = new Float32Array(wasmBg.memory.buffer, doc.slot_xy_ptr, doc.slot_len * 2)
    let ok = 0
    let firstBad = -1
    for (let i = 0; i < xy.length; i++) {
      const v = xy[i]
      if (Number.isFinite(v) && v >= 0 && v <= PROOF_WORLD) ok += 1
      else if (firstBad < 0) firstBad = i
    }
    return firstBad < 0
      ? `shared-memory: PASS (${ok}/${xy.length} in [0,${PROOF_WORLD}])`
      : `shared-memory: FAIL @ index ${firstBad} (${ok}/${xy.length} in [0,${PROOF_WORLD}])`
  } finally {
    doc.free()
  }
}

/**
 * The wgpu engine mount for the editor. Accepts the full `TacticalMapProps` for drop-in parity
 * with the Deck `TacticalMap`, but this slice reads only `className` (the rest — terrain, layer
 * toggles, callbacks — are honored by T-151.1+).
 */
export default function WgpuTacticalMap({ className }: TacticalMapProps) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const [canvasKey, setCanvasKey] = useState(0)
  const [forceWebgl, setForceWebgl] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [backend, setBackend] = useState('initializing…')
  const [fps, setFps] = useState(0)
  const [proof, setProof] = useState('shared-memory: …')

  useEffect(() => {
    const container = containerRef.current
    const canvas = canvasRef.current
    if (!container || !canvas) return

    let engine: RenderEngine | null = null // I2
    let disposed = false // I4
    let raf = 0
    let lastDpr = window.devicePixelRatio
    let lastHud = 0
    let frames = 0

    // Shared-memory proof runs at mount, independent of GPU init (it only needs the shared wasm
    // linear memory the merged pkg gives us). Synchronous + self-freeing (L10).
    setProof(sharedMemoryProof())

    const applySize = () => {
      const rect = container.getBoundingClientRect()
      const dpr = window.devicePixelRatio
      lastDpr = dpr
      const [dw, dh] = deviceSize(rect.width, rect.height, dpr)
      canvas.width = dw
      canvas.height = dh
      engine?.resize(rect.width, rect.height, dpr)
    }

    // The backing store must be sized BEFORE create (the engine reads canvas.width/height).
    {
      const rect = container.getBoundingClientRect()
      const [dw, dh] = deviceSize(rect.width, rect.height, window.devicePixelRatio)
      canvas.width = dw
      canvas.height = dh
    }

    void createEngine(canvas, forceWebgl)
      .then((created) => {
        if (disposed) {
          created.free() // I4: the effect died while create was in flight
          return
        }
        engine = created
        setBackend(created.backend())
        applySize()
        created.set_view(6400, 6400, -2)
        const loop = (now: number) => {
          if (disposed || !engine) return // I5
          if (window.devicePixelRatio !== lastDpr) applySize()
          try {
            engine.render()
          } catch (err) {
            setError(String(err)) // I6
            return
          }
          frames += 1
          if (now - lastHud > HUD_INTERVAL_MS) {
            if (lastHud > 0) setFps(Math.round((frames * 1000) / (now - lastHud)))
            lastHud = now
            frames = 0
          }
          raf = requestAnimationFrame(loop)
        }
        raf = requestAnimationFrame(loop)
      })
      .catch((err: unknown) => {
        if (!disposed) setError(String(err)) // I6
      })

    const ro = new ResizeObserver(() => {
      if (!disposed && engine) applySize()
    })
    ro.observe(container)

    let dragging = false
    let lastX = 0
    let lastY = 0
    const onPointerDown = (ev: PointerEvent) => {
      if (ev.button !== 0) return
      dragging = true
      lastX = ev.clientX
      lastY = ev.clientY
      canvas.setPointerCapture(ev.pointerId)
    }
    const onPointerMove = (ev: PointerEvent) => {
      if (!dragging || !engine || disposed) return
      engine.pan(ev.clientX - lastX, ev.clientY - lastY)
      lastX = ev.clientX
      lastY = ev.clientY
    }
    const onPointerUp = (ev: PointerEvent) => {
      dragging = false
      if (canvas.hasPointerCapture(ev.pointerId)) canvas.releasePointerCapture(ev.pointerId)
    }
    const onWheel = (ev: WheelEvent) => {
      if (!engine || disposed) return
      ev.preventDefault()
      const rect = canvas.getBoundingClientRect()
      engine.zoom_at(-ev.deltaY * WHEEL_ZOOM_PER_PX, ev.clientX - rect.left, ev.clientY - rect.top)
    }
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('pointercancel', onPointerUp)
    canvas.addEventListener('wheel', onWheel, { passive: false })

    return () => {
      disposed = true // I4
      cancelAnimationFrame(raf) // I5 — before free
      ro.disconnect()
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerUp)
      canvas.removeEventListener('pointercancel', onPointerUp)
      canvas.removeEventListener('wheel', onWheel)
      engine?.free() // I4 — exactly once
      engine = null
    }
  }, [forceWebgl, canvasKey])

  const retryWebgl = useCallback(() => {
    setError(null)
    setBackend('initializing…')
    setForceWebgl(true)
    setCanvasKey((k) => k + 1) // I7 — fresh canvas element
  }, [])

  return (
    <div ref={containerRef} className={className} style={{ background: '#0b0f14' }}>
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ position: 'absolute', inset: 0, width: '100%', height: '100%' }}
      />
      <div style={PANEL}>
        <div style={{ fontWeight: 600, letterSpacing: 0.3 }}>T-151.0 · wgpu editor mount</div>
        <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 18, margin: '2px 0 4px' }}>
          {fps} FPS · {backend}
        </div>
        <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 12, opacity: 0.9 }}>
          {proof}
        </div>
        {error !== null && (
          <div style={{ marginTop: 8 }}>
            <div style={{ color: '#ff9c9c', maxWidth: 420, whiteSpace: 'pre-wrap' }}>{error}</div>
            <button onClick={retryWebgl} style={BTN}>
              Retry with WebGL2 (fresh canvas)
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

const PANEL: CSSProperties = {
  position: 'absolute',
  top: 60,
  left: '50%',
  transform: 'translateX(-50%)',
  padding: '10px 14px',
  borderRadius: 10,
  background: 'rgba(14,20,28,0.78)',
  backdropFilter: 'blur(8px)',
  border: '1px solid rgba(140,198,255,0.18)',
  color: '#e6ebf2',
  font: '13px/1.3 ui-sans-serif, system-ui, sans-serif',
  maxWidth: 520,
  textAlign: 'center',
}

const BTN: CSSProperties = {
  padding: '5px 10px',
  marginTop: 6,
  borderRadius: 7,
  cursor: 'pointer',
  color: '#cdd7e4',
  background: 'rgba(140,198,255,0.12)',
  border: '1px solid rgba(140,198,255,0.25)',
  fontWeight: 600,
  fontSize: 12,
}
