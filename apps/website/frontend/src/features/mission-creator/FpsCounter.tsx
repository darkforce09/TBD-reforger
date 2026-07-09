// Lightweight debug HUD: real rendered frame rate via requestAnimationFrame (reflects
// main-thread + compositor throughput, so pan/zoom hitches show up as dips) and the live
// orthographic zoom (which LOD band the camera is in). DEV-gated at the MissionCreatorPage
// call site. T-151.9: Deck glyph-stream panel removed with the Deck runtime.

import { useEffect, useRef, useState } from 'react'
import { useMapStore } from '../tactical-map/state/useMapStore'

export function FpsCounter() {
  const [fps, setFps] = useState(0)
  const frames = useRef(0)
  const last = useRef(0)

  useEffect(() => {
    let raf = 0
    last.current = performance.now()
    const tick = (now: number) => {
      frames.current += 1
      const elapsed = now - last.current
      if (elapsed >= 500) {
        setFps(Math.round((frames.current * 1000) / elapsed))
        frames.current = 0
        last.current = now
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [])

  const zoom = useMapStore((s) => s.deckZoom)
  const color = fps >= 55 ? '#4ade80' : fps >= 30 ? '#facc15' : '#f87171'

  return (
    <div className="glass pointer-events-none absolute bottom-4 right-4 z-10 flex items-center gap-2 rounded-md px-3 py-1.5 font-mono text-code-md tabular-nums">
      <span className="text-on-surface-variant">
        z<span className="ml-0.5 text-on-surface">{zoom.toFixed(2)}</span>
      </span>
      <span className="text-on-surface-variant">·</span>
      <span>
        <span style={{ color }}>{fps}</span>
        <span className="text-on-surface-variant"> FPS</span>
      </span>
    </div>
  )
}
