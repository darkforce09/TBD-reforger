// Mission Y.Doc lifecycle for the mounted route :id (precursor to the Phase-9
// useMissionEditor, which adds backend hydrate/autosave). Phase 4 is local-only:
// create the doc, make it durable via y-indexeddb, attach undo, and bind it to the
// Zustand mirror — tearing all of it down on unmount / id change. When the IndexedDB
// snapshot loads, its inserts flow through observeDeep into the store automatically,
// so no extra "ready" plumbing is needed.

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { IndexeddbPersistence } from 'y-indexeddb'
import {
  beginBulkSync,
  bindStoreToDoc,
  createMissionDoc,
  createUndoManager,
  endBulkSync,
  seedDefaultLayer,
  seedMeta,
  useMapStore,
  type MissionDoc,
  type UndoController,
} from '@/features/tactical-map'
import {
  detectLegacyV1,
  hasV2Persist,
  legacyDbName,
} from '../persistence/missionPersistSchema'
import { loadSlotsWithProgress } from '../persistence/slotChunkStore'
import { loadMissionMetaIntoDoc } from '../persistence/missionMetaStore'
import { migrateLegacyToV2 } from '../persistence/migrateLegacyToV2'

/** loading → IndexedDB snapshot + (when applicable) server hydrate still in flight. */
export type DocStatus = 'loading' | 'ready'

/** Load phases, in execution order: restore from IndexedDB → download server payload →
 *  apply (hydrate) → reflect into the store (the final coalesced snapshot). `value` is the
 *  overall 0..1 (monotonic). */
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
// Restoring band (0–0.15). The v2 chunked store knows its total → a determinate fraction
// (T-062.1). The legacy y-indexeddb replay has no per-entity signal (no `total`) → a
// count-only soft curve that asymptotes to the band top without claiming completion — real
// motion without a fake %; MissionCreatorPage shows the indeterminate sweep for that case.
export const restoringPhase = (done: number, total?: number): LoadProgress => ({
  phase: 'restoring',
  value: total != null && total > 0 ? 0.15 * Math.min(done / total, 1) : 0.15 * (done / (done + 50_000)),
  label: 'Reading local save…',
  done,
  total,
})
export const downloadPhase = (loaded: number, total?: number): LoadProgress => ({
  phase: 'downloading',
  // Content-Length is often unknown for a gzipped/streamed response — `frac(loaded, 0)` would
  // jump straight to the band top. Until `total` is known, ride a soft curve on bytes loaded.
  value:
    0.15 +
    0.2 * (total && total > 0 ? frac(loaded, total) : loaded / (loaded + 2_000_000)),
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
  /** Fired once after the IndexedDB snapshot has synced and defaults are seeded — the
   *  hook point for backend hydrate / conflict checks (Phase 9 useMissionEditor).
   *  May return a Promise; the doc isn't marked `ready` until it settles, so a large
   *  server hydrate keeps the loading overlay up until its content is in the store.
   *  `onLoadProgress` reports the download + apply phases for the load overlay. */
  onSynced?: (
    md: MissionDoc,
    onLoadProgress?: (p: LoadProgress) => void,
  ) => void | Promise<void>
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

  // One doc + undo manager per mission id; recreated if the id changes — or if `instanceKey`
  // is bumped on teardown (StrictMode fix below).
  const missionKey = missionId ?? 'draft'
  const [instanceKey, setInstanceKey] = useState(0)
  // Bump-once guard: the effect below depends on the recreated `md`/`undo`, so an unconditional
  // bump in cleanup would re-trigger itself forever. StrictMode's single setup→cleanup→setup
  // needs exactly one fresh instance, so allow the bump at most once per mount.
  const recreatedRef = useRef(false)
  const { md, undo, dbName } = useMemo(() => {
    const md = createMissionDoc()
    return {
      md,
      undo: createUndoManager(md),
      dbName: `tbd-mission-${missionKey}`,
    }
    // `instanceKey` is intentionally a dep with no body reference: bumping it on teardown is
    // how we force a fresh doc/undo after StrictMode destroys the previous one.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [missionKey, instanceKey])

  // 'loading' until the local IndexedDB snapshot has synced AND (when applicable) the
  // server hydrate has completed; drives the load overlay in MissionCreatorPage. Reset
  // to 'loading' at render time whenever a fresh doc is created (new mission id or the
  // StrictMode instanceKey bump) — the React-sanctioned "reset state on prop change"
  // pattern, which avoids a setState-in-effect.
  const [docStatus, setDocStatus] = useState<DocStatus>('loading')
  const [loadProgress, setLoadProgress] = useState<LoadProgress>(INITIAL_LOAD)
  const [trackedMd, setTrackedMd] = useState(md)
  if (trackedMd !== md) {
    setTrackedMd(md)
    setDocStatus('loading')
    setLoadProgress(INITIAL_LOAD)
  }
  // Stable so the effect's deps don't churn; setState identity is already stable.
  const reportLoad = useCallback((p: LoadProgress) => setLoadProgress(p), [])

  useEffect(() => {
    let mountedHere = true
    // Set the instant the effect tears down so the async boot IIFE below stops applying
    // chunks / seeding into a doc that's about to be destroyed (StrictMode setup→cleanup→setup).
    let cancelled = false
    if (import.meta.env.DEV) console.debug('[mission-doc] mount', { missionKey, instanceKey })
    // Open a bulk-sync window BEFORE binding + before any replay, so the local restore and the
    // prime coalesce into one store flush at endBulkSync instead of flushing per-transaction
    // (T-060 load).
    let bulkOpen = true
    beginBulkSync()
    const unbind = bindStoreToDoc(md)
    // Created only on the legacy branch; tracked so cleanup can destroy it if we tear down
    // mid-migration (v2 + fresh missions never touch y-indexeddb — T-062.1).
    let legacyPersistence: IndexeddbPersistence | null = null

    // Legacy-only restoring poll (T-060.1.1 / T-062.2): the y-indexeddb replay lands the whole
    // doc in ONE synchronous Y.applyUpdate with no per-entity signal, so we poll slots.size to
    // show "Reading local save…" with a growing (indeterminate) count. The v2 branch reports
    // determinate per-chunk progress instead, so it never starts this poll. Visibility-aware:
    // rAF is throttled in a hidden tab, so fall back to setInterval when hidden; one timer live.
    let restoreRaf: number | null = null
    let restoreInterval: ReturnType<typeof setInterval> | null = null
    let polling = false
    const tick = () => reportLoad(restoringPhase(md.entities.slots.size))
    const clearTimers = () => {
      if (restoreRaf != null) cancelAnimationFrame(restoreRaf)
      restoreRaf = null
      if (restoreInterval != null) clearInterval(restoreInterval)
      restoreInterval = null
    }
    const schedule = () => {
      clearTimers()
      if (document.visibilityState === 'hidden') {
        restoreInterval = setInterval(tick, 500)
      } else {
        const rafPoll = () => {
          tick()
          restoreRaf = requestAnimationFrame(rafPoll)
        }
        restoreRaf = requestAnimationFrame(rafPoll)
      }
    }
    const onVisibilityChange = () => {
      if (polling) schedule()
    }
    const startPoll = () => {
      if (polling) return
      polling = true
      document.addEventListener('visibilitychange', onVisibilityChange)
      schedule()
    }
    const stopRestorePoll = () => {
      clearTimers()
      if (!polling) return
      polling = false
      document.removeEventListener('visibilitychange', onVisibilityChange)
    }

    const seedDefaults = () => {
      seedMeta(md, { id: missionKey, title: 'Untitled Mission' })
      seedDefaultLayer(md)
    }

    // Reconcile with the server (download + hydrate reports its own phases), then close the
    // bulk window with the single coalesced snapshot. Kept OPEN through reconcile (T-060.1 fix)
    // so the final flush lands AFTER hydrate, not before (which double-flushed the 300k snapshot).
    const finishReconcile = async () => {
      const reconcile = onSyncedRef.current?.(md, reportLoad)
      await Promise.resolve(reconcile)
      await endBulkSync((done, total) => reportLoad(localPhase(done, total)))
      bulkOpen = false
      if (!cancelled && mountedHere) setDocStatus('ready')
    }

    void (async () => {
      try {
        if (await hasV2Persist(missionKey)) {
          // v2 — chunked restore, NO IndexeddbPersistence.
          if (cancelled) return
          await loadMissionMetaIntoDoc(missionKey, md)
          if (cancelled) return
          await loadSlotsWithProgress(
            missionKey,
            md,
            (done, total) => reportLoad(restoringPhase(done, total)),
            () => cancelled,
          )
          if (cancelled) return
          seedDefaults()
        } else if (await detectLegacyV1(missionKey)) {
          // legacy — blocking y-indexeddb replay (poll for motion), then migrate once to v2.
          if (cancelled) return
          const persistence = new IndexeddbPersistence(dbName, md.doc)
          legacyPersistence = persistence
          startPoll()
          await new Promise<void>((resolve) => persistence.once('synced', () => resolve()))
          stopRestorePoll()
          if (cancelled) return
          seedDefaults()
          await migrateLegacyToV2(missionKey, md, reportLoad, () => cancelled)
          if (cancelled) return
          // Stop observing y-indexeddb then delete its DB so future edits don't re-bloat it
          // (v2 debounced writes own durability now). Null the ref first so cleanup's
          // destroy() won't double-fire.
          legacyPersistence = null
          await persistence.destroy()
          try {
            indexedDB.deleteDatabase(legacyDbName(missionKey))
          } catch {
            /* best-effort; a stale legacy DB is harmless (hasV2Persist gates the path) */
          }
        } else {
          // fresh mission — no persistence yet; v2 debounced writes engage on the first edit.
          seedDefaults()
        }
      } catch (e) {
        if (import.meta.env.DEV) console.error('[mission-doc] boot failed', e)
      }
      if (cancelled) return
      await finishReconcile()
    })()

    return () => {
      mountedHere = false
      cancelled = true
      if (import.meta.env.DEV) console.debug('[mission-doc] unmount', { missionKey, instanceKey })
      stopRestorePoll()
      unbind()
      // Balance the bulk window if we unmounted before reconcile closed it. After unbind,
      // activeFlush is cleared, so this end is a no-op (won't push into a torn-down store).
      if (bulkOpen) void endBulkSync()
      undo.destroy()
      legacyPersistence?.destroy()
      md.doc.destroy()
      useMapStore.getState().reset()
      // React 19 StrictMode (dev) double-invokes this effect setup→cleanup→setup WITHOUT
      // re-running the useMemo above, so the second setup would re-bind this now-destroyed
      // doc + UndoManager (undo silently dead → canUndo() always false). Bump instanceKey so
      // useMemo allocates a fresh md + UndoController before the next setup. Once per mount:
      // a real missionKey change already recreates via the memo dep; on true unmount the guard
      // avoids a setState-after-unmount.
      if (!recreatedRef.current) {
        recreatedRef.current = true
        setInstanceKey((k) => k + 1)
      }
    }
    // instanceKey is only read for the dev mount/unmount log; an instanceKey bump already
    // recreates md (via the useMemo above), so the effect re-runs regardless. Listed to
    // satisfy exhaustive-deps without changing behavior.
  }, [md, undo, dbName, missionKey, reportLoad, instanceKey])

  return { md, undo, docStatus, loadProgress }
}
