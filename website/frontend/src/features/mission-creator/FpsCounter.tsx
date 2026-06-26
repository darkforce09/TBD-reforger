// Lightweight debug HUD: measures real rendered frame rate via requestAnimationFrame
// (reflects main-thread + compositor throughput, so pan/zoom hitches show up as dips).
// Phase-1 debug aid; remove or gate behind a dev flag once the editor matures.

import { useEffect, useRef, useState } from 'react'

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

  const color = fps >= 55 ? '#4ade80' : fps >= 30 ? '#facc15' : '#f87171'

  return (
    <div className="glass pointer-events-none absolute bottom-4 right-4 z-10 rounded-md px-3 py-1.5 font-mono text-code-md tabular-nums">
      <span style={{ color }}>{fps}</span>
      <span className="text-on-surface-variant"> FPS</span>
    </div>
  )
}
