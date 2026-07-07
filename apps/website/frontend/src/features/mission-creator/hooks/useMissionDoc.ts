// Mission document lifecycle for the mounted route :id (T-145 Phase 3.2 F3). The authoritative doc is
// the wasm `yrs` core behind a stable `WasmMissionDoc` shell: the shell is memoized (StrictMode-safe —
// no wasm handle inside the memo), and the lifecycle effect attaches a FRESH wasm handle on setup and
// detaches (frees) it on cleanup, so React 19 StrictMode's setup→cleanup→setup double-invoke never
// shares — and then double-frees — one handle (wasm `.free()` is not idempotent; [[wasm-react-lifecycle]]).
//
// Boot: load the v3 whole-doc blob from IndexedDB (yrsPersist) → apply_update (INIT/untracked) → resync
// the store → seed defaults if empty → reconcile with the server (onSynced) → ready. Legacy v1/v2 IDB
// drafts are dropped (re-hydrate from the server) — the whole v1/v2/migrate branch set is gone.

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import {
  createMissionDoc,
  createUndoManager,
  seedDefaultLayer,
  seedMeta,
  useMapStore,
  type MissionDoc,
  type UndoController,
} from '@/features/tactical-map'
import { loadState } from '../persistence/yrsPersist'

/** loading → the local blob restore + (when applicable) server hydrate are still in flight.
 *  error → the local restore threw (corrupt/blocked IndexedDB); the editor must NOT present as an
 *  empty `ready` doc — the consumer shows an error overlay. */
export type DocStatus = 'loading' | 'ready' | 'error'

/** Load phases, in execution order: restore blob → download server payload → apply (hydrate) →
 *  reflect into the store. `value` is the overall 0..1 (monotonic). */
export type LoadPhase = 'restoring' | 'downloading' | 'applying' | 'local'
export interface LoadProgress {
  phase: LoadPhase
  value: number
  label: string
  done?: number
  total?: number
}

// Weighted, monotonic, in execution order (a skipped phase just fast-forwards its band):
// restoring 0–0.15, download 0.15–0.35, apply 0.35–0.55, local (final snapshot) 0.55–1.0.
const frac = (done: number, total: number) => (total > 0 ? Math.min(done / total, 1) : 1)
// The v3 blob load has no per-entity signal (one apply_update) → a count-only soft curve that
// asymptotes to the band top; MissionCreatorPage shows the indeterminate sweep for that band.
export const restoringPhase = (done: number, total?: number): LoadProgress => ({
  phase: 'restoring',
  value:
    total != null && total > 0 ? 0.15 * Math.min(done / total, 1) : 0.15 * (done / (done + 50_000)),
  label: 'Reading local save…',
  done,
  total,
})
export const downloadPhase = (loaded: number, total?: number): LoadProgress => ({
  phase: 'downloading',
  value: 0.15 + 0.2 * (total && total > 0 ? frac(loaded, total) : loaded / (loaded + 2_000_000)),
  label: 'Downloading mission…',
})
export const applyPhase = (done: number, total: number): LoadProgress => ({
  phase: 'applying',
  value: 0.35 + 0.2 * frac(done, total),
  label: 'Applying server data…',
  done,
  total,
})
export const localPhase = (done: number, total: number): LoadProgress => ({
  phase: 'local',
  value: 0.55 + 0.45 * frac(done, total),
  label: 'Loading mission data…',
  done,
  total,
})

export interface MissionDocHandle {
  md: MissionDoc
  undo: UndoController
  docStatus: DocStatus
  loadProgress: LoadProgress
}

export interface UseMissionDocOptions {
  /** Fired once after the local blob has restored and defaults are seeded — the hook point for
   *  backend hydrate / conflict checks (useMissionEditor). May return a Promise; the doc isn't marked
   *  `ready` until it settles, so a large server hydrate keeps the overlay up. */
  onSynced?: (md: MissionDoc, onLoadProgress?: (p: LoadProgress) => void) => void | Promise<void>
}

const INITIAL_LOAD: LoadProgress = restoringPhase(0)

export function useMissionDoc(
  missionId: string | undefined,
  options?: UseMissionDocOptions,
): MissionDocHandle {
  // Keep the latest onSynced without re-running the lifecycle effect.
  const onSyncedRef = useRef(options?.onSynced)
  useEffect(() => {
    onSyncedRef.current = options?.onSynced
  })

  // One shell + undo controller per mission id; recreated if the id changes — or if `instanceKey` is
  // bumped on teardown (StrictMode fix below). The shell holds no wasm handle until the effect attaches
  // one, so memoizing it is safe (unlike a raw wasm object).
  const missionKey = missionId ?? 'draft'
  const [instanceKey, setInstanceKey] = useState(0)
  const recreatedRef = useRef(false)
  const { shell, undo } = useMemo(() => {
    const s = createMissionDoc()
    return { shell: s, undo: createUndoManager(s) }
    // `instanceKey` is intentionally a dep with no body reference: bumping it on teardown forces a
    // fresh shell/undo after StrictMode destroys the previous one.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [missionKey, instanceKey])

  const [docStatus, setDocStatus] = useState<DocStatus>('loading')
  const [loadProgress, setLoadProgress] = useState<LoadProgress>(INITIAL_LOAD)
  // Reset to 'loading' at render time whenever a fresh shell is created (new id or the StrictMode
  // instanceKey bump) — the React-sanctioned "reset state on prop change" pattern.
  const [trackedShell, setTrackedShell] = useState(shell)
  if (trackedShell !== shell) {
    setTrackedShell(shell)
    setDocStatus('loading')
    setLoadProgress(INITIAL_LOAD)
  }
  const reportLoad = useCallback((p: LoadProgress) => setLoadProgress(p), [])

  useEffect(() => {
    // Set the instant the effect tears down so the async boot IIFE stops applying into a shell whose
    // handle is about to be freed (StrictMode setup→cleanup→setup).
    let cancelled = false
    // eslint-disable-next-line no-console -- dev diagnostic behind import.meta.env.DEV (LOG-2)
    if (import.meta.env.DEV) console.debug('[mission-doc] mount', { missionKey, instanceKey })
    // Effect-local wasm handle — created + freed per effect invocation so StrictMode never double-frees.
    shell.attach(new wasm.MissionDoc())
    // loadProgress already starts at INITIAL_LOAD (restoringPhase(0)) via useState + the trackedShell
    // reset, so no synchronous reportLoad is needed here (that would be a setState-in-effect).

    void (async () => {
      let bootError: unknown = null
      try {
        const blob = await loadState(missionKey)
        if (cancelled) return
        if (blob) shell.applyUpdate(blob) // INIT/untracked in the Rust doc; throws on a corrupt blob
        useMapStore.getState()._applySnapshot(shell.snapshot())
        if (!shell.hasContent()) {
          // fresh mission / empty blob → seed title + a default folder (both INIT, not undoable).
          seedMeta(shell, { id: missionKey, title: 'Untitled Mission' })
          seedDefaultLayer(shell)
        }
      } catch (e) {
        bootError = e
        if (import.meta.env.DEV) console.error('[mission-doc] boot failed', e)
      }
      if (cancelled) return
      if (bootError) {
        // Local restore failed — surface it instead of dropping the user into a blank `ready` editor.
        setDocStatus('error')
        return
      }
      // Reconcile with the server (download + hydrate reports its own phases). onSynced surfaces its
      // own load errors as toasts, so a rejection here must not block `ready`.
      try {
        await Promise.resolve(onSyncedRef.current?.(shell, reportLoad))
      } catch (e) {
        if (import.meta.env.DEV) console.error('[mission-doc] reconcile failed', e)
      }
      if (cancelled) return
      // Reflect the final state (a server adopt hydrates under INIT) into the store.
      useMapStore.getState()._applySnapshot(shell.snapshot())
      setDocStatus('ready')
    })()

    return () => {
      cancelled = true
      // eslint-disable-next-line no-console -- dev diagnostic behind import.meta.env.DEV (LOG-2)
      if (import.meta.env.DEV) console.debug('[mission-doc] unmount', { missionKey, instanceKey })
      undo.destroy()
      shell.detach() // frees the wasm handle
      useMapStore.getState().reset()
      // React 19 StrictMode (dev) double-invokes this effect setup→cleanup→setup WITHOUT re-running the
      // useMemo above, so the second setup would re-attach to a shell whose handle was just freed. Bump
      // instanceKey so useMemo allocates a fresh shell + undo before the next setup. Once per mount.
      if (!recreatedRef.current) {
        recreatedRef.current = true
        setInstanceKey((k) => k + 1)
      }
    }
  }, [shell, undo, missionKey, reportLoad, instanceKey])

  return { md: shell, undo, docStatus, loadProgress }
}
