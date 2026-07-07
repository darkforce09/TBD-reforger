// Undo/redo over the wasm `yrs` doc (T-145 Phase 3.2 F3). The Rust MissionDoc owns the undo stack,
// origin-scoped so only LOCAL user gestures are tracked (load/seed under INIT are not undoable). This
// thin controller drives the toolbar buttons and resyncs the Zustand store after an undo/redo.
// subscribe() fires on every change-version bump (mutation + undo/redo) so the buttons re-read
// canUndo/canRedo. The wasm handle is freed by the shell's detach (useMissionDoc), so destroy() is a
// no-op.

import { useMapStore } from './useMapStore'
import type { WasmMissionDoc } from './wasmDoc'

export interface UndoController {
  undo: () => void
  redo: () => void
  canUndo: () => boolean
  canRedo: () => boolean
  /** Fires after every change (mutation + undo/redo); returns an unsubscribe fn. */
  subscribe: (cb: () => void) => () => void
  destroy: () => void
}

export function createUndoManager(md: WasmMissionDoc): UndoController {
  return {
    undo: () => {
      if (md.undo()) useMapStore.getState()._applySnapshot(md.snapshot())
    },
    redo: () => {
      if (md.redo()) useMapStore.getState()._applySnapshot(md.snapshot())
    },
    canUndo: () => md.canUndo(),
    canRedo: () => md.canRedo(),
    subscribe: (cb) => md.subscribe(() => cb()),
    destroy: () => {
      /* nothing to tear down — the wasm handle is freed by the shell's detach (useMissionDoc) */
    },
  }
}
