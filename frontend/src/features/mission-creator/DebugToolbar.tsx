// TEMPORARY debug harness for Phase 4 (same throwaway spirit as FpsCounter) — proves
// the state foundation end to end: add/clear units, undo/redo. Replaced by the real
// Top Command Strip + Outliner + Toolbelt in later phases.

import { useEffect, useReducer } from 'react'
import {
  addSlot,
  clearAll,
  getTerrain,
  type MissionDoc,
  type UndoController,
} from '@/features/tactical-map'

interface DebugToolbarProps {
  md: MissionDoc
  undo: UndoController
}

const TERRAIN = getTerrain('everon')

/** Random world position with a margin off the terrain edge. */
function randomPos() {
  const margin = 1000
  return {
    x: margin + Math.random() * (TERRAIN.width - 2 * margin),
    y: margin + Math.random() * (TERRAIN.height - 2 * margin),
  }
}

export function DebugToolbar({ md, undo }: DebugToolbarProps) {
  const [, bump] = useReducer((n: number) => n + 1, 0)

  // Re-render on undo-stack changes so the buttons reflect canUndo/canRedo.
  useEffect(() => undo.subscribe(bump), [undo])

  const btn =
    'rounded-md px-3 py-1.5 text-label-md transition-colors disabled:opacity-40 disabled:cursor-not-allowed'

  return (
    <div className="glass pointer-events-auto absolute bottom-4 left-1/2 z-10 flex -translate-x-1/2 items-center gap-1.5 rounded-lg px-2 py-1.5">
      <button
        className={`${btn} bg-primary/15 text-primary hover:bg-primary/25`}
        onClick={() => addSlot(md, randomPos())}
      >
        Add unit
      </button>
      <button
        className={`${btn} bg-primary/10 text-on-surface-variant hover:bg-primary/20`}
        onClick={() => {
          for (let i = 0; i < 50; i++) addSlot(md, randomPos())
        }}
      >
        Add ×50
      </button>
      <span className="mx-1 h-5 w-px bg-white/10" />
      <button
        className={`${btn} text-on-surface-variant hover:bg-white/10`}
        onClick={undo.undo}
        disabled={!undo.canUndo()}
      >
        Undo
      </button>
      <button
        className={`${btn} text-on-surface-variant hover:bg-white/10`}
        onClick={undo.redo}
        disabled={!undo.canRedo()}
      >
        Redo
      </button>
      <span className="mx-1 h-5 w-px bg-white/10" />
      <button
        className={`${btn} text-error hover:bg-error/15`}
        onClick={() => clearAll(md)}
      >
        Clear
      </button>
    </div>
  )
}
