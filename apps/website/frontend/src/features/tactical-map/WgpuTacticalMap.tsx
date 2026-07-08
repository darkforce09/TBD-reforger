// T-151.1 W1 basemap lane — the wgpu `RenderEngine` editor mount now renders the basemap stack
// (satellite unified / pyramid + hillshade + grid) to parity with the Deck `TacticalMap`, selected
// by `MissionCreatorPage` behind the engine flag (`VITE_MC_ENGINE=wgpu` or `?engine=wgpu`). The
// T-151.0 calibration scene is hidden (`hide_calibration`); it survives only on the /_spike/wgpu
// page. World objects + slots + interaction are T-151.2+ (the `onReady`/`onCursorMove`/… props stay
// no-ops until W7).
//
// Wasm memory lifecycle invariants I2–I7 are reused VERBATIM from WgpuCanvas.tsx (the spike page):
//   I2 the engine handle is EFFECT-LOCAL — never useMemo/useState-persisted; `.free()` is NOT
//      idempotent (see the wasm-react-lifecycle memory).
//   I4 free exactly once on every path: cleanup frees iff committed; the async create path checks
//      `disposed` right after the await and frees-without-committing.
//   I5 no render-after-free: the rAF loop re-checks the flag; rAF is cancelled before free.
//   I6 errors surface into the banner, never swallowed.
//   I7 retry = a NEW canvas element (React key bump) forced to WebGL2 — a canvas permanently
//      commits to its first getContext kind, so same-canvas backend retry is impossible.
// (I1 module-init once + I3 creation mutex live in ./wgpu/wasmRender.) Terrain switches remount the
// whole component (MissionCreatorPage keys it on terrain) → a fresh canvas + engine.

import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react'
import { WHEEL_ZOOM_PER_PX, createEngine, deviceSize, type RenderEngine } from './wgpu/wasmRender'
import { WgpuBasemapController } from './wgpu/wgpuBasemap'
import { useWgpuBasemap } from './wgpu/useWgpuBasemap'
import { WgpuWorldController } from './wgpu/wgpuWorldLoader'
import { useWgpuWorldResidency } from './wgpu/useWgpuWorldResidency'
import { getTerrain } from './coords/terrains'
import type { TacticalMapProps } from './types'

const HUD_INTERVAL_MS = 250

/**
 * The wgpu engine mount for the editor (T-151.1). Honors `terrain` / `showGrid` / `showHillshade` /
 * `hillshadeOpacity` / `onBasemapDegraded` / `onBasemapProgress` and reads `mapStyle` via the
 * basemap hook. Interaction + selection callbacks remain no-ops until W7.
 */
export default function WgpuTacticalMap({
  terrain = 'everon',
  className,
  showGrid = true,
  showHillshade = false,
  hillshadeOpacity = 0.4,
  onBasemapDegraded,
  onBasemapProgress,
}: TacticalMapProps) {
  const terrainDef = getTerrain(terrain)
  const containerRef = useRef<HTMLDivElement | null>(null)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const controllerRef = useRef<WgpuBasemapController | null>(null)
  const worldControllerRef = useRef<WgpuWorldController | null>(null)
  const [canvasKey, setCanvasKey] = useState(0)
  const [forceWebgl, setForceWebgl] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [backend, setBackend] = useState('initializing…')
  const [fps, setFps] = useState(0)
  const [basemapMode, setBasemapMode] = useState('…')
  const [ready, setReady] = useState(false)

  // Latest-callback refs: the controller is created ONCE (effect-local), so it must call the
  // current prop callbacks (MissionCreatorPage's are useCallback-stable, but this is robust anyway).
  const onDegradedRef = useRef(onBasemapDegraded)
  const onProgressRef = useRef(onBasemapProgress)
  useEffect(() => {
    onDegradedRef.current = onBasemapDegraded
    onProgressRef.current = onBasemapProgress
  })

  // Prop → controller effects (basemap view, hillshade, grid, opacity re-tints, paper tint).
  useWgpuBasemap(controllerRef, ready, {
    terrainId: terrain,
    showGrid,
    showHillshade,
    hillshadeOpacity,
  })
  // World-object residency (W3): kicks the manifest/prefab/chunk-index load once ready.
  useWgpuWorldResidency(worldControllerRef, ready, { terrainId: terrain })

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
        created.set_view(terrainDef.width / 2, terrainDef.height / 2, -2)
        created.hide_calibration() // W1: no calibration scene in the editor (L1)
        controllerRef.current = new WgpuBasemapController(created, terrainDef, {
          onProgress: (f) => onProgressRef.current?.(f),
          onDegraded: (v) => onDegradedRef.current?.(v),
        })
        worldControllerRef.current = new WgpuWorldController(created, terrainDef)
        setReady(true) // fires the basemap + world hook effects
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
            try {
              setBasemapMode(String((JSON.parse(engine.stats()) as { basemap_mode: string }).basemap_mode))
            } catch {
              /* stats parse is best-effort HUD only */
            }
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
      controllerRef.current?.onCameraMoved() // pyramid LOD follows the pan (debounced)
      worldControllerRef.current?.onCameraMoved() // world chunk residency follows the pan
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
      controllerRef.current?.onCameraMoved()
      worldControllerRef.current?.onCameraMoved()
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
      controllerRef.current?.dispose()
      controllerRef.current = null
      worldControllerRef.current?.dispose() // frees the WorldResidency wasm handle (once)
      worldControllerRef.current = null
      setReady(false)
      engine?.free() // I4 — exactly once
      engine = null
    }
  }, [forceWebgl, canvasKey, terrainDef])

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
        <div style={{ fontWeight: 600, letterSpacing: 0.3 }}>T-151.1 · wgpu basemap</div>
        <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 18, margin: '2px 0 4px' }}>
          {fps} FPS · {backend}
        </div>
        <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 12, opacity: 0.9 }}>
          basemap: {basemapMode}
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
