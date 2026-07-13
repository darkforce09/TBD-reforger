// T-154 — the 3D arsenal doll mount. Dumb per D5: sizes the canvas (deviceSize BEFORE
// create), forwards pointer deltas (drag = orbit) and sub-threshold clicks (pick), pushes
// the 14-byte region state array (RAIL order), and drives a damage-driven rAF loop — ALL
// scene/camera/pick policy lives in Rust (map_engine_core::doll / DollEngine). Lifecycle
// invariants I2–I7 mirrored from WgpuTacticalMap: effect-local engine handle, free exactly
// once, cancel rAF before free, disposed-guard on the in-flight create.

import { useEffect, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent } from 'react'
import { RAIL_REGIONS } from './arsenalDollModel'
import type { LoadoutKey } from './arsenalRules'
import { createDollEngine, deviceSize, type DollEngine } from './dollEngine'

const CLICK_SLOP_PX = 4 // same bar as the map's drag threshold

export function SoldierModel3D({
  picks,
  activeKey,
  onSelect,
  onUnavailable,
}: {
  picks: Record<LoadoutKey, string>
  activeKey: LoadoutKey
  onSelect: (key: LoadoutKey) => void
  /** Engine creation failed (no WebGPU/WebGL2) — caller swaps in the SVG fallback. */
  onUnavailable: () => void
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const engineRef = useRef<DollEngine | null>(null) // I2: never useState/useMemo
  const [ready, setReady] = useState(false)
  const unavailableRef = useRef(onUnavailable)
  useEffect(() => {
    unavailableRef.current = onUnavailable
  }, [onUnavailable])

  useEffect(() => {
    const container = containerRef.current
    const canvas = canvasRef.current
    if (!container || !canvas) return

    let engine: DollEngine | null = null
    let disposed = false // I4
    let raf = 0
    let lastDpr = window.devicePixelRatio

    const applySize = () => {
      const rect = container.getBoundingClientRect()
      const dpr = window.devicePixelRatio
      lastDpr = dpr
      const [dw, dh] = deviceSize(rect.width || 1, rect.height || 1, dpr)
      canvas.width = dw
      canvas.height = dh
      engine?.resize(rect.width || 1, rect.height || 1, dpr)
    }

    // Backing store BEFORE create — the engine reads canvas.width/height.
    {
      const rect = container.getBoundingClientRect()
      const [dw, dh] = deviceSize(rect.width || 1, rect.height || 1, window.devicePixelRatio)
      canvas.width = dw
      canvas.height = dh
    }

    void createDollEngine(canvas)
      .then((created) => {
        if (disposed) {
          created.free() // I4: effect died while create was in flight
          return
        }
        engine = created
        engineRef.current = created
        applySize()
        setReady(true)
        const loop = () => {
          if (disposed || !engine) return // I5
          if (window.devicePixelRatio !== lastDpr) applySize()
          try {
            engine.render() // damage-driven: Rust no-ops idle frames
          } catch {
            // transient surface hiccup — next frame retries
          }
          raf = requestAnimationFrame(loop)
        }
        raf = requestAnimationFrame(loop)
      })
      .catch(() => {
        if (!disposed) unavailableRef.current()
      })

    const ro = new ResizeObserver(() => {
      if (!disposed && engine) applySize()
    })
    ro.observe(container)

    return () => {
      disposed = true
      cancelAnimationFrame(raf) // I5 before free
      ro.disconnect()
      engineRef.current = null
      setReady(false)
      engine?.free() // I4: exactly once
      engine = null
    }
  }, [])

  // Region states, RAIL order: 2 = active, 1 = equipped, 0 = empty.
  useEffect(() => {
    const engine = engineRef.current
    if (!ready || !engine) return
    const states = new Uint8Array(
      RAIL_REGIONS.map((r) => (r.key === activeKey ? 2 : picks[r.key] ? 1 : 0)),
    )
    engine.set_states(states)
  }, [ready, picks, activeKey])

  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null)
  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return
    drag.current = { x: e.clientX, y: e.clientY, moved: false }
    e.currentTarget.setPointerCapture(e.pointerId)
  }
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current
    if (!d) return
    const dx = e.clientX - d.x
    if (!d.moved && Math.abs(dx) + Math.abs(e.clientY - d.y) > CLICK_SLOP_PX) d.moved = true
    if (d.moved && dx !== 0) engineRef.current?.rotate(dx)
    d.x = e.clientX
  }
  const onPointerUp = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current
    drag.current = null
    if (!d || d.moved) return
    const container = containerRef.current
    const engine = engineRef.current
    if (!container || !engine) return
    const rect = container.getBoundingClientRect()
    const idx = engine.pick_region(e.clientX - rect.left, e.clientY - rect.top)
    if (idx >= 0 && idx < RAIL_REGIONS.length) onSelect(RAIL_REGIONS[idx].key)
  }

  return (
    <div
      ref={containerRef}
      className="relative h-full w-full cursor-grab touch-none select-none active:cursor-grabbing"
      role="img"
      aria-label="Soldier loadout — drag to rotate, click a part to select"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={() => {
        drag.current = null
      }}
    >
      <canvas
        ref={canvasRef}
        className="absolute inset-0 h-full w-full rounded-lg"
        style={{ width: '100%', height: '100%' }}
      />
    </div>
  )
}
