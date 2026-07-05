// Lightweight debug HUD: real rendered frame rate via requestAnimationFrame (reflects
// main-thread + compositor throughput, so pan/zoom hitches show up as dips), the live Deck
// orthographic zoom (which LOD band the camera is in), and the world-glyph draw count.
//
// A verbose world-object stream readout (manifest rows/with-render · visibleInstances calls ·
// streamed · lookup + an ERR flag) is gated behind the worldmap-debug setting — toggle with
// Ctrl+Alt+D (persisted in worldLayerPrefs). It pinpoints an empty stage straight off a
// screenshot: m-1 → manifest not loaded; str>0 lut0 → glyph lookup empty; c0 → gate/toggle.
// Phase-1 debug aid; the whole HUD is DEV-gated at the MissionCreatorPage call site.

import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useMapStore } from '../tactical-map/state/useMapStore'
import { toggleWorldmapDebug, useWorldmapDebug } from '../tactical-map/state/worldLayerPrefs'
import {
  getPropGlyphs,
  getTreeGlyphs,
  getTreeStreamDebug,
  subscribeTreeStream,
  type TreeStreamDebug,
} from '../tactical-map/worldmap/treeStore'

/** Snapshot getter for useSyncExternalStore: total drawn tree + prop glyphs (a number, so the
 *  Object.is identity is stable between commits). */
const worldGlyphCount = (): number => getTreeGlyphs().length + getPropGlyphs().length

const EMPTY_DEBUG: TreeStreamDebug = {
  manifestRows: -1,
  manifestRender: -1,
  lookup: 0,
  calls: 0,
  streamed: 0,
  error: '',
}

export function FpsCounter() {
  const [fps, setFps] = useState(0)
  const [dbg, setDbg] = useState<TreeStreamDebug>(EMPTY_DEBUG)
  const frames = useRef(0)
  const last = useRef(0)
  const debug = useWorldmapDebug()

  useEffect(() => {
    let raf = 0
    last.current = performance.now()
    const tick = (now: number) => {
      frames.current += 1
      const elapsed = now - last.current
      if (elapsed >= 500) {
        setFps(Math.round((frames.current * 1000) / elapsed))
        setDbg(getTreeStreamDebug())
        frames.current = 0
        last.current = now
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [])

  // Ctrl+Alt+D toggles the verbose world-object debug readout (persisted).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.altKey && (e.key === 'd' || e.key === 'D')) {
        e.preventDefault()
        toggleWorldmapDebug()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const zoom = useMapStore((s) => s.deckZoom)
  const glyphs = useSyncExternalStore(subscribeTreeStream, worldGlyphCount, worldGlyphCount)

  const color = fps >= 55 ? '#4ade80' : fps >= 30 ? '#facc15' : '#f87171'

  return (
    <div className="glass pointer-events-none absolute bottom-4 right-4 z-10 flex items-center gap-2 rounded-md px-3 py-1.5 font-mono text-code-md tabular-nums">
      <span className="text-on-surface-variant">
        z<span className="ml-0.5 text-on-surface">{zoom.toFixed(2)}</span>
      </span>
      <span className="text-on-surface-variant">·</span>
      {debug && (
        <>
          <span
            className="text-on-surface-variant"
            title="manifest rows/with-render · visibleInstances calls · streamed · lookup"
          >
            m{dbg.manifestRows}/{dbg.manifestRender} c{dbg.calls} str{dbg.streamed} lut{dbg.lookup}
          </span>
          {dbg.error && (
            <span style={{ color: '#f87171' }} title={dbg.error}>
              ERR
            </span>
          )}
          <span className="text-on-surface-variant">·</span>
        </>
      )}
      <span className="text-on-surface-variant">
        glyph<span className="ml-1 text-on-surface">{glyphs}</span>
      </span>
      <span className="text-on-surface-variant">·</span>
      <span>
        <span style={{ color }}>{fps}</span>
        <span className="text-on-surface-variant"> FPS</span>
      </span>
    </div>
  )
}
