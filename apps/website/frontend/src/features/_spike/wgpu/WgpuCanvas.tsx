// T-151 wgpu render-engine spike page (/_spike/wgpu) — the React ↔ canvas ↔ wasm ↔ wgpu
// spine. Not in the app nav; no auth, no chrome (mirrors /_spike/doc-core).
//
// Wasm memory lifecycle invariants (plan §S5; `.free()` is NOT idempotent — see the
// wasm-react-lifecycle memory):
//   I1 module init once + I3 creation mutex — in wasmRender.ts.
//   I2 the engine handle is EFFECT-LOCAL (never useMemo/useState-persisted).
//   I4 free exactly once on every path: cleanup frees iff committed; the async create path
//      checks `disposed` right after the await and frees-without-committing.
//   I5 no render-after-free: the rAF loop re-checks the flag; rAF is cancelled before free.
//   I6 errors surface into the banner, never swallowed.
//   I7 retry = a NEW canvas element (React key bump) — a canvas permanently commits to its
//      first getContext kind, so same-canvas backend retry is impossible by spec.

import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react'
import { useFps } from '../useFps'
import {
  WHEEL_ZOOM_PER_PX,
  createEngine,
  deviceSize,
  type RenderEngine,
} from '@/features/tactical-map/wgpu/wasmRender'

const STRESS_COUNTS = [100_000, 1_000_000, 5_000_000, 20_000_000] as const
const STRESS_SEED = 0x12345678
const HUD_INTERVAL_MS = 250

export default function WgpuCanvas() {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  // For button handlers only — ownership stays with the effect (I2).
  const engineRef = useRef<RenderEngine | null>(null)
  const [canvasKey, setCanvasKey] = useState(0)
  const [forceWebgl, setForceWebgl] = useState(
    () => new URLSearchParams(window.location.search).get('force') === 'webgl',
  )
  const [error, setError] = useState<string | null>(null)
  const [backend, setBackend] = useState('initializing…')
  const [view, setView] = useState<{ tx: number; ty: number; zoom: number } | null>(null)
  const [statsJson, setStatsJson] = useState('')
  const [report, setReport] = useState('')
  const [busy, setBusy] = useState(false)
  const fps = useFps()

  useEffect(() => {
    const container = containerRef.current
    const canvas = canvasRef.current
    if (!container || !canvas) return

    let engine: RenderEngine | null = null // I2
    let disposed = false // I4
    let raf = 0
    let lastDpr = window.devicePixelRatio
    let lastHud = 0

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
        engineRef.current = created
        // Dev-only headless hooks (GPU-R): CDP awaits these for the byte-exact self-check JSON
        // (same mechanism T-151.1 used). `worldBuilding` is the T-151.3 S4 gate; `calibration`
        // (T-151.0) + `texture` (T-151.1) are the regression re-runs.
        // T-151.11.2 (A-10): registration is DEV-gated (the route already is; belt-and-braces).
        if (import.meta.env.DEV) {
          ;(
            window as unknown as { __selfChecks?: Record<string, () => Promise<string>> }
          ).__selfChecks = {
            calibration: () => created.self_check() as Promise<string>,
            texture: () => created.texture_self_check() as Promise<string>,
            worldBuilding: () => created.world_building_self_check() as Promise<string>,
            seaBand: () => created.sea_band_self_check() as Promise<string>,
            roadCenterline: () => created.road_centerline_self_check() as Promise<string>,
            marquee: () => created.marquee_self_check() as Promise<string>, // T-151.11.1 P-02
          }
        }
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
          if (now - lastHud > HUD_INTERVAL_MS) {
            lastHud = now
            setView({ tx: engine.target_x, ty: engine.target_y, zoom: engine.zoom })
            setStatsJson(engine.stats())
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
      delete (window as unknown as { __selfChecks?: Record<string, () => Promise<string>> })
        .__selfChecks
      engineRef.current = null
      engine?.free() // I4 — exactly once
      engine = null
    }
  }, [forceWebgl, canvasKey])

  const runSelfCheck = useCallback(async () => {
    const engine = engineRef.current
    if (!engine) return
    setBusy(true)
    setReport('running self-check…')
    try {
      const result: unknown = await engine.self_check()
      setReport(typeof result === 'string' ? result : JSON.stringify(result))
    } catch (err) {
      setReport(`self-check FAILED: ${String(err)}`)
    } finally {
      setBusy(false)
    }
  }, [])

  const runWorldBuildingCheck = useCallback(async () => {
    const engine = engineRef.current
    if (!engine) return
    setBusy(true)
    setReport('running world-building self-check…')
    try {
      const result: unknown = await engine.world_building_self_check()
      setReport(typeof result === 'string' ? result : JSON.stringify(result))
    } catch (err) {
      setReport(`world-building self-check FAILED: ${String(err)}`)
    } finally {
      setBusy(false)
    }
  }, [])

  const seedStress = useCallback((n: number) => {
    const engine = engineRef.current
    if (!engine) return
    setBusy(true)
    setReport(`seeding ${n.toLocaleString()} instances…`)
    // Let the status paint before the synchronous generate+upload burst.
    setTimeout(() => {
      try {
        const t0 = performance.now()
        engine.seed_stress(n, STRESS_SEED)
        const stats = engine.stats()
        setStatsJson(stats)
        setReport(
          `seeded ${n.toLocaleString()} in ${Math.round(performance.now() - t0)} ms — ${stats}`,
        )
      } catch (err) {
        setReport(`seed_stress FAILED: ${String(err)}`)
      } finally {
        setBusy(false)
      }
    }, 30)
  }, [])

  const clearStress = useCallback(() => {
    engineRef.current?.clear_stress()
    setReport('stress pool cleared')
  }, [])

  const retryWebgl = useCallback(() => {
    setError(null)
    setReport('')
    setBackend('initializing…')
    setForceWebgl(true)
    setCanvasKey((k) => k + 1) // I7 — fresh canvas element
  }, [])

  return (
    <div
      ref={containerRef}
      style={{ position: 'fixed', inset: 0, background: '#0b0f14', color: '#e6ebf2' }}
    >
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ position: 'absolute', inset: 0, width: '100%', height: '100%' }}
      />
      <div style={PANEL}>
        <div style={{ fontWeight: 600, letterSpacing: 0.3 }}>T-151 · wgpu render spike</div>
        <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 22, margin: '2px 0 6px' }}>
          {fps} FPS · {backend}
        </div>
        <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 12, opacity: 0.85 }}>
          {view
            ? `target ${view.tx.toFixed(1)}, ${view.ty.toFixed(1)} · zoom ${view.zoom.toFixed(3)}`
            : 'camera —'}
        </div>
        <div style={{ display: 'flex', gap: 6, margin: '8px 0 6px', flexWrap: 'wrap' }}>
          <button onClick={() => void runSelfCheck()} disabled={busy} style={BTN}>
            Run self-check
          </button>
          <button onClick={() => void runWorldBuildingCheck()} disabled={busy} style={BTN}>
            World-building check
          </button>
          {STRESS_COUNTS.map((n) => (
            <button key={n} onClick={() => seedStress(n)} disabled={busy} style={BTN}>
              {n / 1_000_000 >= 1 ? `${n / 1_000_000}M` : `${n / 1000}k`}
            </button>
          ))}
          <button onClick={clearStress} disabled={busy} style={BTN}>
            Clear
          </button>
          <a href="/_spike/wgpu?force=webgl" style={{ ...BTN, textDecoration: 'none' }}>
            Force WebGL2
          </a>
        </div>
        {statsJson !== '' && <pre style={PRE}>{statsJson}</pre>}
        {report !== '' && <pre style={PRE}>{report}</pre>}
        {error !== null && (
          <div style={{ marginTop: 8 }}>
            <div style={{ color: '#ff9c9c', maxWidth: 420, whiteSpace: 'pre-wrap' }}>{error}</div>
            <button onClick={retryWebgl} style={{ ...BTN, marginTop: 6 }}>
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
  top: 16,
  left: 16,
  padding: '12px 14px',
  borderRadius: 10,
  background: 'rgba(14,20,28,0.78)',
  backdropFilter: 'blur(8px)',
  border: '1px solid rgba(140,198,255,0.18)',
  font: '13px/1.3 ui-sans-serif, system-ui, sans-serif',
  maxWidth: 520,
}

const PRE: CSSProperties = {
  margin: '6px 0 0',
  padding: '6px 8px',
  borderRadius: 6,
  background: 'rgba(0,0,0,0.35)',
  fontSize: 11,
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-all',
  maxWidth: 480,
  maxHeight: 180,
  overflow: 'auto',
}

const BTN: CSSProperties = {
  padding: '5px 10px',
  borderRadius: 7,
  cursor: 'pointer',
  color: '#cdd7e4',
  background: 'rgba(140,198,255,0.12)',
  border: '1px solid rgba(140,198,255,0.25)',
  fontWeight: 600,
  fontSize: 12,
}
