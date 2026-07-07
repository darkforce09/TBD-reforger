import { useEffect, useState } from 'react'

/** rAF frame-rate meter (main-thread fps; drops when a render can't keep up) — the operator's
 *  criterion-6 readout on the doc-core spike page. */
export function useFps(): number {
  const [fps, setFps] = useState(0)
  useEffect(() => {
    let raf = 0
    let frames = 0
    let last = performance.now()
    const loop = () => {
      frames++
      const now = performance.now()
      if (now - last >= 500) {
        setFps(Math.round((frames * 1000) / (now - last)))
        frames = 0
        last = now
      }
      raf = requestAnimationFrame(loop)
    }
    raf = requestAnimationFrame(loop)
    return () => cancelAnimationFrame(raf)
  }, [])
  return fps
}
