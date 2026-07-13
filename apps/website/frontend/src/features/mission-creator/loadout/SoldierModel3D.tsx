// T-154 — the 3D arsenal doll mount. Dumb per D5: sizes the canvas (deviceSize BEFORE
// create), forwards pointer deltas (drag = turn character) and sub-threshold clicks (pick),
// pushes the 14-byte region state array (RAIL order), and drives a damage-driven rAF loop —
// ALL scene/camera/pick/anchor policy lives in Rust (map_engine_core::doll / DollEngine).
// T-154.1 adds the allowed-DOM layer: a cursor tooltip for the hovered part and a pinned
// name chip + leader line for the ACTIVE part (anchor px comes from Rust; positions are
// mutated directly in the rAF loop — no per-frame React renders). Lifecycle invariants
// I2–I7 mirrored from WgpuTacticalMap.

import { useEffect, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent } from 'react'
import type { RegistryItem } from '@/types/models/registry'
import { RAIL_REGIONS } from './arsenalDollModel'
import type { LoadoutKey } from './arsenalRules'
import { createDollEngine, deviceSize, type DollEngine } from './dollEngine'

const CLICK_SLOP_PX = 4 // same bar as the map's drag threshold
const CALLOUT_DX = 52 // chip offset from the anchor (up-right)
const CALLOUT_DY = -44

export function SoldierModel3D({
  picks,
  activeKey,
  onSelect,
  onUnavailable,
  catalogByName,
}: {
  picks: Record<LoadoutKey, string>
  activeKey: LoadoutKey
  onSelect: (key: LoadoutKey) => void
  /** Engine creation failed (no WebGPU/WebGL2) — caller swaps in the SVG fallback. */
  onUnavailable: () => void
  catalogByName: ReadonlyMap<string, RegistryItem>
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const tooltipRef = useRef<HTMLDivElement | null>(null)
  const calloutRef = useRef<HTMLDivElement | null>(null)
  const leaderRef = useRef<HTMLDivElement | null>(null)
  const engineRef = useRef<DollEngine | null>(null) // I2: never useState/useMemo
  const [ready, setReady] = useState(false)
  const [hoverIdx, setHoverIdx] = useState(-1)
  const unavailableRef = useRef(onUnavailable)
  useEffect(() => {
    unavailableRef.current = onUnavailable
  }, [onUnavailable])

  const nameOf = (key: LoadoutKey): string => {
    const rn = picks[key]
    return rn ? (catalogByName.get(rn)?.display_name ?? rn) : 'empty'
  }
  const activeIdx = RAIL_REGIONS.findIndex((r) => r.key === activeKey)
  const activeIdxRef = useRef(activeIdx)
  useEffect(() => {
    activeIdxRef.current = activeIdx
  }, [activeIdx])

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

    // T-154.1: the active-part callout tracks its Rust-projected anchor every frame
    // (direct style mutation — the chip/leader are display:none while hidden).
    const placeCallout = () => {
      const chip = calloutRef.current
      const leader = leaderRef.current
      if (!chip || !leader || !engine) return
      const anchor = engine.anchor_px(activeIdxRef.current)
      if (anchor.length !== 2) {
        chip.style.display = 'none'
        leader.style.display = 'none'
        return
      }
      const rect = container.getBoundingClientRect()
      const ax = anchor[0]
      const ay = anchor[1]
      const cx = Math.min(Math.max(ax + CALLOUT_DX, 8), Math.max(rect.width - 8, 8))
      const cy = Math.min(Math.max(ay + CALLOUT_DY, 8), Math.max(rect.height - 8, 8))
      chip.style.display = 'block'
      chip.style.transform = `translate(${cx}px, ${cy}px)`
      const dx = cx - ax
      const dy = cy - ay
      const len = Math.hypot(dx, dy)
      leader.style.display = 'block'
      leader.style.width = `${len}px`
      leader.style.transform = `translate(${ax}px, ${ay}px) rotate(${Math.atan2(dy, dx)}rad)`
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
          placeCallout()
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

  const setHover = (idx: number) => {
    if (idx === hoverIdx) return
    engineRef.current?.set_hover(idx)
    setHoverIdx(idx)
  }

  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null)
  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return
    drag.current = { x: e.clientX, y: e.clientY, moved: false }
    e.currentTarget.setPointerCapture(e.pointerId)
  }
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const container = containerRef.current
    const engine = engineRef.current
    const d = drag.current
    if (d) {
      const dx = e.clientX - d.x
      if (!d.moved && Math.abs(dx) + Math.abs(e.clientY - d.y) > CLICK_SLOP_PX) {
        d.moved = true
        setHover(-1) // rotating — drop the hover highlight
      }
      if (d.moved && dx !== 0) engine?.rotate(dx)
      d.x = e.clientX
      return
    }
    if (!container || !engine) return
    const rect = container.getBoundingClientRect()
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    setHover(engine.pick_region(x, y))
    // Cursor tooltip follows the pointer (clamped inside the container).
    const tip = tooltipRef.current
    if (tip) {
      tip.style.transform = `translate(${Math.min(x + 14, Math.max(rect.width - 8, 8))}px, ${Math.max(y - 26, 4)}px)`
    }
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

  const hoverRegion = hoverIdx >= 0 ? RAIL_REGIONS[hoverIdx] : null
  const activeRegion = activeIdx >= 0 ? RAIL_REGIONS[activeIdx] : null

  return (
    <div
      ref={containerRef}
      className={`relative h-full w-full touch-none select-none ${hoverRegion ? 'cursor-pointer' : 'cursor-grab active:cursor-grabbing'}`}
      role="img"
      aria-label="Soldier loadout — drag to rotate, click a part to select"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={() => {
        drag.current = null
      }}
      onPointerLeave={() => setHover(-1)}
    >
      <canvas
        ref={canvasRef}
        className="absolute inset-0 h-full w-full rounded-lg"
        style={{ width: '100%', height: '100%' }}
      />
      {/* Leader line: origin at the anchor, rotated toward the chip (rAF-positioned). */}
      <div
        ref={leaderRef}
        className="pointer-events-none absolute left-0 top-0 hidden h-px origin-left bg-primary/70"
      />
      {/* Active-part callout chip (rAF-positioned at anchor + offset). */}
      {activeRegion && (
        <div
          ref={calloutRef}
          className="pointer-events-none absolute left-0 top-0 hidden whitespace-nowrap rounded-md border border-primary/40 bg-surface-container-lowest/90 px-2 py-1 text-label-sm text-on-surface shadow-lg"
        >
          <span className="text-primary">{activeRegion.label}</span>
          <span className="normal-case text-on-surface-variant"> — {nameOf(activeRegion.key)}</span>
        </div>
      )}
      {/* Hover tooltip follows the cursor. */}
      {hoverRegion && (
        <div
          ref={tooltipRef}
          className="pointer-events-none absolute left-0 top-0 whitespace-nowrap rounded bg-surface-container-lowest/90 px-1.5 py-0.5 text-label-sm text-on-surface shadow"
        >
          {hoverRegion.label}
          <span className="normal-case text-on-surface-variant"> — {nameOf(hoverRegion.key)}</span>
        </div>
      )}
    </div>
  )
}
