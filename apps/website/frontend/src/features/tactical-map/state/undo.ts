// Y.UndoManager wrapper (Ultra Plan §2.3). Scoped to the nine tracked maps and to
// LOCAL_ORIGIN only, so undo/redo replays exactly the local user gestures (each one
// transaction = one step). subscribe() drives button enabled-state now and the
// Phase-9 "Visual-Git" timeline scrubber later.

import * as Y from 'yjs'
import { LOCAL_ORIGIN, trackedTypes, type MissionDoc } from './ydoc'

export interface UndoController {
  manager: Y.UndoManager
  undo: () => void
  redo: () => void
  clear: () => void
  canUndo: () => boolean
  canRedo: () => boolean
  /** Fires after every push/pop/clear; returns an unsubscribe fn. */
  subscribe: (cb: () => void) => () => void
  destroy: () => void
}

export function createUndoManager(md: MissionDoc): UndoController {
  const manager = new Y.UndoManager(trackedTypes(md), {
    trackedOrigins: new Set([LOCAL_ORIGIN]),
  })

  const subscribe = (cb: () => void) => {
    manager.on('stack-item-added', cb)
    manager.on('stack-item-popped', cb)
    manager.on('stack-cleared', cb)
    return () => {
      manager.off('stack-item-added', cb)
      manager.off('stack-item-popped', cb)
      manager.off('stack-cleared', cb)
    }
  }

  return {
    manager,
    undo: () => manager.undo(),
    redo: () => manager.redo(),
    clear: () => manager.clear(),
    canUndo: () => manager.undoStack.length > 0,
    canRedo: () => manager.redoStack.length > 0,
    subscribe,
    destroy: () => manager.destroy(),
  }
}
