// Phase 9 editor lifecycle — wraps useMissionDoc (local Y.Doc + y-indexeddb) and adds the
// backend layer: load/hydrate the current server version, prompt on a local-vs-server
// conflict (Decisions log), track unsaved changes, manual Save Version (immutable semver
// snapshot via POST /missions/:id/versions), and Export (download the camelCase mod JSON).
// Debounced autosave stays LOCAL (y-indexeddb) — the versions API has no draft/overwrite route.

import { useCallback, useEffect, useRef, useState } from 'react'
import {
  applyMissionRowMeta,
  hydrateMissionDoc,
  hydrateMissionDocWithProgress,
  pickMapSnapshot,
  useMapStore,
  type MissionDoc,
} from '@/features/tactical-map'
import { LOCAL_ORIGIN } from '@/features/tactical-map'
import { api } from '@/api/client'
import type { MissionDetail } from '@/types/api'
import {
  useMissionDoc,
  applyPhase,
  downloadPhase,
  type LoadProgress,
  type MissionDocHandle,
} from './useMissionDoc'
import {
  clearEditorSession,
  markEditorSessionReady,
  readWarmEditorSession,
} from './editorSession'
import { flushSlots, saveSlotsFromDocDebounced } from '../persistence/slotChunkStore'
import { flushMeta, saveMissionMetaFromDocDebounced } from '../persistence/missionMetaStore'
// Compile + version-blob assembly run in a Web Worker (T-066); call sites below are unchanged.
import {
  buildVersionBlob,
  compileMission,
  compileMissionWithProgress,
  terminateCompiler,
} from '../compiler/compilerClient'
import { toMissionExport } from '../compiler/exportSchema'
import {
  estimateCompiledBytes,
  formatBytes,
  getLocalDocBytes,
  SERVER_VERSION_BODY_LIMIT,
} from '../lib/missionSize'

export interface SaveResult {
  ok: boolean
  error?: string
  /** Always populated on failure (and on success); see SaveDebugReport. T-060.1.3. */
  debug?: SaveDebugReport
}

/** Which network path the version POST took (T-060.1.2/.3). `direct` = dev proxy bypass to :8080;
 *  `configured` = VITE_API_URL is already absolute; `proxy` = default /api/v1 (Vite dev proxy). */
export type UploadRoute = 'proxy' | 'direct' | 'configured'

/** Save Version progress: phase + optional 0..1 fraction (undefined = indeterminate).
 *  `preparing` = building the (chunked) request body before the upload starts (T-060.1.2).
 *  The size/route fields (T-060.1.3) feed the Save dialog readout. */
export interface SaveProgress {
  phase: 'compiling' | 'preparing' | 'uploading'
  value?: number
  // T-060.1.3 observability (all optional — populated as each step learns them):
  slotCount?: number
  compiledBytes?: number // exact request-body bytes once buildVersionBlob is done
  estimatedBytes?: number // fast pre-compile estimate
  localDocBytes?: number // local Y.Doc footprint
  bytesLoaded?: number
  bytesTotal?: number
  uploadRoute?: UploadRoute
  uploadUrl?: string // resolved base + path, no token
  elapsedMs?: number
}

/** Full diagnostic snapshot returned on save failure (and success). Surfaced in the Save dialog's
 *  copyable "Debug details" so a mid-upload ERR_NETWORK is diagnosable in one attempt. T-060.1.3. */
export interface SaveDebugReport {
  phaseAtFailure: SaveProgress['phase']
  slotCount: number
  estimatedBytes: number
  localDocBytes: number
  compiledBytes?: number
  bytesLoaded?: number
  bytesTotal?: number
  uploadRoute?: UploadRoute
  uploadUrl?: string
  axiosCode?: string
  axiosMessage?: string
  responseStatus?: number
  elapsedMs: number
  timestamp: string
}

export interface MissionEditorHandle extends MissionDocHandle {
  dirty: boolean
  suggestedSemver: string
  saveVersion: (
    semver: string,
    notes?: string,
    onProgress?: (p: SaveProgress) => void,
  ) => Promise<SaveResult>
  exportJson: () => void
  /** Server payload awaiting a keep-local / load-server decision; null when none. */
  conflict: Record<string, unknown> | null
  resolveConflict: (choice: 'local' | 'server') => void
  /** The route :id isn't a real mission UUID — server persistence can't work. */
  invalidMissionId: boolean
}

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const NEEDS_REAL_ID =
  'This editor URL needs a real mission id — create one from Mission Library first.'

// A large version POST in dev must skip the Vite dev proxy: http-proxy drops big POST bodies
// (~2 MB+) → ECONNRESET, which the browser reports as ERR_NETWORK with no HTTP response. CORS on
// the API already allows :5173, so we override the baseURL for that one request to hit :8080
// directly. Small saves + prod keep the default client baseURL. T-060.1.2 (E3b).
const DIRECT_UPLOAD_THRESHOLD = 1_048_576 // 1 MB
let warnedDirectUpload = false
/** Resolve the version POST baseURL + a label for it. `baseURL: undefined` → axios uses the client
 *  default (proxy in dev, or an already-absolute VITE_API_URL). T-060.1.2/.3. */
function resolveVersionUpload(bodyBytes: number): { baseURL: string | undefined; route: UploadRoute } {
  const configured = import.meta.env.VITE_API_URL
  // Client baseURL already absolute → axios hits the API directly; nothing to override.
  if (typeof configured === 'string' && configured.startsWith('http')) {
    return { baseURL: undefined, route: 'configured' }
  }
  if (import.meta.env.DEV && bodyBytes > DIRECT_UPLOAD_THRESHOLD) {
    return {
      baseURL: import.meta.env.VITE_API_DIRECT_URL ?? 'http://localhost:8080/api/v1',
      route: 'direct',
    }
  }
  return { baseURL: undefined, route: 'proxy' } // small save / prod → default /api/v1
}

const bumpPatch = (semver: string | null): string => {
  const m = semver?.match(/^(\d+)\.(\d+)\.(\d+)/)
  return m ? `${m[1]}.${m[2]}.${Number(m[3]) + 1}` : '0.1.0'
}

/** Does the local doc already hold authored content (beyond seeded defaults)? */
const hasLocalContent = (md: MissionDoc): boolean =>
  md.entities.factions.size > 0 ||
  md.entities.slots.size > 0 ||
  md.entities.objectives.size > 0 ||
  md.entities.vehicles.size > 0 ||
  md.entities.markers.size > 0

export function useMissionEditor(missionId: string | undefined): MissionEditorHandle {
  const [dirty, setDirty] = useState(false)
  const [currentSemver, setCurrentSemver] = useState<string | null>(null)
  const [conflict, setConflict] = useState<Record<string, unknown> | null>(null)
  const mounted = useRef(true)
  // Row metadata (title/terrain/env) from the initial GET, kept so a "load server"
  // conflict resolution can re-apply it after hydrating the payload.
  const lastRowMeta = useRef<{
    title: string
    terrain: string
    time_of_day?: string
    weather?: string
  } | null>(null)

  // After the local snapshot syncs, reconcile with the server's current version.
  const onSynced = useCallback(
    (md: MissionDoc, onLoadProgress?: (p: LoadProgress) => void) => {
      if (!missionId) return
      // Warm session fast path (T-062.2): if THIS mission was marked ready earlier this tab
      // session (e.g. before an alt-tab HMR reload) and IndexedDB has already replayed its
      // content into the doc, skip the multi-MB GET entirely. Title/terrain/env are already
      // in md.meta from y-indexeddb; restore currentSemver from the session record. Trusts
      // local IDB — remote changes aren't seen until a cold load (see editorSession.ts).
      const warm = readWarmEditorSession(missionId)
      if (warm && hasLocalContent(md)) {
        lastRowMeta.current = null
        setCurrentSemver(warm.currentSemver)
        return Promise.resolve()
      }
      // Cold load (no warm record, a different mission, or no local content yet): drop any
      // stale marker so a mid-boot reload can't wrongly trust it. The ready effect re-marks.
      if (!hasLocalContent(md)) clearEditorSession()
      // Return the chain so useMissionDoc holds the loading overlay until the server
      // payload is hydrated (or the request fails); the .catch resolves either way.
      return api
        .get(`/missions/${missionId}`, {
          // Large missions ship a multi-MB payload — report the download phase to the overlay.
          onDownloadProgress: (e) => onLoadProgress?.(downloadPhase(e.loaded, e.total)),
        })
        .then(async (res) => {
          if (!mounted.current) return
          const data = res.data as MissionDetail
          // Always hydrate the mission-row meta — even for a brand-new mission whose
          // json_payload is still {} (the bug this fixes: the old early-return left the
          // editor on "Untitled Mission" / Everon).
          const row = {
            title: data.title,
            terrain: data.terrain,
            time_of_day: data.time_of_day,
            weather: data.weather,
          }
          lastRowMeta.current = row
          const version = data.current_version
          setCurrentSemver(version?.semver ?? null)
          const payload = version?.json_payload
          const isEmpty = !payload || Object.keys(payload).length === 0
          if (isEmpty) {
            applyMissionRowMeta(md, row) // new mission → title/terrain/env from the row
            return
          }
          if (hasLocalContent(md)) {
            setConflict(payload) // local edits + server version → prompt the user
          } else {
            // Empty local → adopt server. Chunked so a 300k payload reports the apply phase
            // and doesn't block; runs inside the still-open bulk window (one snapshot after).
            await hydrateMissionDocWithProgress(md, payload, (d, t) =>
              onLoadProgress?.(applyPhase(d, t)),
            )
            applyMissionRowMeta(md, row) // row title wins (compile omits title from payload)
          }
        })
        .catch(() => {
          /* mission not on the API (e.g. ad-hoc id) → stay local-only */
        })
    },
    [missionId],
  )

  const { md, undo, docStatus, loadProgress } = useMissionDoc(missionId, { onSynced })

  // Read inside the (md-scoped) update listener without re-subscribing on every status flip:
  // only persist edits once the editor is past boot (INIT/restore writes are filtered anyway).
  const docReadyRef = useRef(false)
  useEffect(() => {
    docReadyRef.current = docStatus === 'ready'
  }, [docStatus])

  // Doc-liveness guard for v2 persistence (T-062.1): useMissionDoc destroys md.doc on teardown.
  // A debounced/queued save that's mid-flight then must NOT read the destroyed doc and rewrite a
  // truncated record. `docAlive` flips false the instant the doc is destroyed; it's passed to the
  // savers as their isCancelled. useMissionDoc's effect is defined first, so its destroy fires
  // before this hook's flush cleanup runs.
  const docAlive = useRef(true)
  useEffect(() => {
    docAlive.current = true
    const onDestroy = () => {
      docAlive.current = false
    }
    md.doc.on('destroy', onDestroy)
    return () => {
      md.doc.off('destroy', onDestroy)
    }
  }, [md])
  const isDocCancelled = useCallback(() => !docAlive.current, [])

  // Once the editor is ready, record a warm session marker (T-062.2) so a subsequent boot
  // of this mission in the same tab (e.g. an alt-tab HMR reload) can skip the server GET.
  // Reads slotCount straight off the store (no O(n) recompute) and carries currentSemver,
  // which lives here, not in useMissionDoc.
  useEffect(() => {
    if (docStatus === 'ready' && missionId) {
      markEditorSessionReady(missionId, {
        slotCount: useMapStore.getState().slotCount,
        currentSemver,
      })
    }
  }, [docStatus, missionId, currentSemver])

  // Mark unsaved on any local (user) edit; INIT/persistence-origin (boot restore) updates don't
  // count. The same gesture also persists to the v2 idb store (T-062.1): meta lightly debounced,
  // slots debounced ~2s + serialized. Gated on `docReadyRef` so a stray pre-ready local edit
  // doesn't write mid-boot.
  useEffect(() => {
    mounted.current = true
    const onUpdate = (_u: Uint8Array, origin: unknown) => {
      if (origin !== LOCAL_ORIGIN) return
      setDirty(true)
      if (docReadyRef.current && missionId) {
        saveMissionMetaFromDocDebounced(md, missionId, isDocCancelled)
        saveSlotsFromDocDebounced(md, missionId, isDocCancelled)
      }
    }
    md.doc.on('update', onUpdate)
    return () => {
      mounted.current = false
      md.doc.off('update', onUpdate)
    }
  }, [md, missionId, isDocCancelled])

  // Flush pending v2 writes when the tab is hidden / closed / reloaded (T-062.1). These fire
  // while the doc is still alive, so the async chunked rewrite completes and an F5 restores the
  // latest edits. On a plain SPA unmount the doc is already destroyed (useMissionDoc tears down
  // first), so the queued saves' isDocCancelled aborts them — never corrupting the record; the
  // ≤2s debounce window is the only edits not persisted in that case.
  useEffect(() => {
    if (!missionId) return
    const flushAll = () => {
      void flushMeta(missionId)
      void flushSlots(missionId)
    }
    const onVisibility = () => {
      if (document.visibilityState === 'hidden') flushAll()
    }
    document.addEventListener('visibilitychange', onVisibility)
    window.addEventListener('pagehide', flushAll)
    return () => {
      document.removeEventListener('visibilitychange', onVisibility)
      window.removeEventListener('pagehide', flushAll)
      flushAll()
    }
  }, [missionId])

  // Tear down the compiler Web Worker on mission unmount (T-066). The worker is spawned lazily on
  // the first save/export, so a StrictMode dev double-mount or a route leave before any save is a
  // safe no-op; getCompiler respawns the singleton on the next save/export.
  useEffect(() => () => terminateCompiler(), [])

  const invalidMissionId = !!missionId && !UUID_RE.test(missionId)

  const saveVersion = useCallback(
    async (
      semver: string,
      notes?: string,
      onProgress?: (p: SaveProgress) => void,
    ): Promise<SaveResult> => {
      if (!missionId || invalidMissionId) return { ok: false, error: NEEDS_REAL_ID }
      // Measure up-front (T-060.1.3) so every progress tick + the debug report carry real bytes.
      const t0 = performance.now()
      // Pick only the serializable entity dicts (T-066): the full store carries action functions
      // that structuredClone can't transfer to the compiler Worker (DataCloneError).
      const snapshot = pickMapSnapshot(useMapStore.getState())
      const slotCount = Object.keys(snapshot.slotsById).length
      const report: SaveDebugReport = {
        phaseAtFailure: 'compiling',
        slotCount,
        estimatedBytes: estimateCompiledBytes(snapshot),
        localDocBytes: getLocalDocBytes(md),
        elapsedMs: 0,
        timestamp: new Date().toISOString(),
      }
      const logPhase = (phase: SaveProgress['phase']) => {
        report.phaseAtFailure = phase
        report.elapsedMs = Math.round(performance.now() - t0)
        if (import.meta.env.DEV) console.debug('[mission-save]', phase, { ...report })
      }
      try {
        // Compile off the render path, chunked + yielding so a 50k+ mission doesn't hang.
        // omitOrbat (T-062.1.1): drop the redundant top-level orbat[] — slots already ship in
        // editor.slots and the server derives ORBAT from the editor graph on event attach.
        const payload = await compileMissionWithProgress(
          snapshot,
          (done, total) =>
            onProgress?.({
              phase: 'compiling',
              value: total ? done / total : undefined,
              slotCount,
              estimatedBytes: report.estimatedBytes,
              localDocBytes: report.localDocBytes,
            }),
          { omitOrbat: true },
        )
        logPhase('compiling')
        // Build the request body as a Blob, streaming editor.slots (T-060.1.2): posting a pre-built
        // Blob means axios sends it as-is (no second 100MB+ graph stringify) and keeps the tab
        // responsive. The Blob does NOT fix ERR_NETWORK on its own — see resolveVersionUpload.
        const body = await buildVersionBlob(semver, payload, notes ?? '', (done, total) =>
          onProgress?.({
            phase: 'preparing',
            value: total ? done / total : undefined,
            slotCount,
            estimatedBytes: report.estimatedBytes,
          }),
        )
        report.compiledBytes = body.size
        report.bytesTotal = body.size
        logPhase('preparing')

        // In dev, large uploads bypass the Vite proxy (which drops big POST bodies → ERR_NETWORK)
        // and hit the API directly; small saves + prod keep the default baseURL.
        const { baseURL, route } = resolveVersionUpload(body.size)
        report.uploadRoute = route
        const base =
          baseURL ??
          (typeof import.meta.env.VITE_API_URL === 'string' ? import.meta.env.VITE_API_URL : '/api/v1')
        report.uploadUrl = `${base}/missions/${missionId}/versions`
        if (route === 'direct' && !warnedDirectUpload) {
          warnedDirectUpload = true
          console.info(
            `[mission-creator] Large save (${formatBytes(body.size)}) bypassing the Vite dev proxy → ${baseURL}`,
          )
        }

        // Pre-upload gate (T-060.1.3): reject an over-cap body before POSTing, with exact bytes —
        // the server's MaxBytesReader otherwise aborts mid-stream and the browser sees ERR_NETWORK.
        if (body.size > SERVER_VERSION_BODY_LIMIT) {
          report.phaseAtFailure = 'preparing'
          report.elapsedMs = Math.round(performance.now() - t0)
          if (import.meta.env.DEV) console.error('[mission-save] OVER LIMIT', { ...report })
          return {
            ok: false,
            error:
              `Compiled payload is ${formatBytes(body.size)} — server limit is ${SERVER_VERSION_BODY_LIMIT >> 20} MB. ` +
              `Raise MISSION_VERSION_MAX_BODY_BYTES on the API for a mission this large.`,
            debug: { ...report },
          }
        }

        await api.post(`/missions/${missionId}/versions`, body, {
          baseURL, // undefined → axios falls back to the client baseURL
          // A 360k payload is tens–hundreds of MB and minutes to upload — without these the
          // default/proxy timeout drops the request and we get a no-response network error.
          headers: { 'Content-Type': 'application/json' },
          timeout: 600_000,
          maxBodyLength: Infinity,
          maxContentLength: Infinity,
          onUploadProgress: (e) => {
            // The upload has begun — record it so a mid-upload failure reports
            // phaseAtFailure: 'uploading' rather than the stale 'preparing' (T-060.1.4).
            report.phaseAtFailure = 'uploading'
            // axios `total` is often undefined for a Blob → fall back to body.size (the "4%" red herring).
            const bytesTotal = e.total ?? body.size
            report.bytesLoaded = e.loaded
            report.bytesTotal = bytesTotal
            onProgress?.({
              phase: 'uploading',
              value: bytesTotal ? e.loaded / bytesTotal : undefined,
              slotCount,
              compiledBytes: body.size,
              bytesLoaded: e.loaded,
              bytesTotal,
              uploadRoute: route,
              uploadUrl: report.uploadUrl,
            })
          },
        })
        logPhase('uploading')
        if (mounted.current) {
          setCurrentSemver(semver)
          setDirty(false)
        }
        // Keep the warm marker's semver current so a post-save reload restores the right
        // version without a GET (T-062.2).
        if (missionId) {
          markEditorSessionReady(missionId, {
            slotCount: useMapStore.getState().slotCount,
            currentSemver: semver,
          })
        }
        return { ok: true, debug: { ...report } }
      } catch (e) {
        // Surface the real failure (T-060): the body cap (413) and backend messages were
        // previously hidden behind a generic "Could not save version".
        const err = e as {
          response?: { status?: number; data?: { error?: string } }
          code?: string
          message?: string
        }
        const resp = err.response
        report.elapsedMs = Math.round(performance.now() - t0)
        report.axiosCode = err.code
        report.axiosMessage = err.message
        report.responseStatus = resp?.status
        if (import.meta.env.DEV) console.error('[mission-save] FAILED', { ...report })
        const debug: SaveDebugReport = { ...report }
        if (!resp) {
          // A non-serializable value reached the compiler Worker (T-066) — structuredClone throws
          // a DataCloneError, which has no HTTP response. Call it out rather than blaming the network.
          if (err.message?.includes('could not be cloned')) {
            return { ok: false, error: 'Worker compile failed: non-serializable state', debug }
          }
          // No HTTP response: timeout, or a connection drop mid-upload (proxy or direct). Surface
          // axios code + message (e.g. ECONNABORTED, ERR_NETWORK) — full bytes/route in `debug`.
          const detail = [err.code, err.message].filter(Boolean).join(': ')
          return {
            ok: false,
            error: `Could not reach the server${detail ? ` (${detail})` : ' (network error or timeout)'}.`,
            debug,
          }
        }
        const backend = resp.data?.error
        if (resp.status === 413) {
          return {
            ok: false,
            error: backend ?? 'Mission too large for the server limit (max 256 MB).',
            debug,
          }
        }
        if (resp.status === 409) {
          return { ok: false, error: backend ?? `Version ${semver} already exists`, debug }
        }
        return { ok: false, error: backend ?? 'Could not save version', debug }
      }
    },
    [missionId, invalidMissionId, md],
  )

  const exportJson = useCallback(async () => {
    const state = pickMapSnapshot(useMapStore.getState()) // function-free for the Worker (T-066)
    const payload = await compileMission(state) // worker compile (T-066)
    const doc = toMissionExport(state.meta, payload, currentSemver ?? '0.1.0')
    const blob = new Blob([JSON.stringify(doc, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `mission-${state.meta?.id ?? missionId ?? 'draft'}.json`
    a.click()
    URL.revokeObjectURL(url)
  }, [currentSemver, missionId])

  const resolveConflict = useCallback(
    (choice: 'local' | 'server') => {
      if (choice === 'server' && conflict) {
        hydrateMissionDoc(md, conflict)
        if (lastRowMeta.current) applyMissionRowMeta(md, lastRowMeta.current)
        setDirty(false)
        // Adopted the server payload — drop any warm marker so the next boot re-validates
        // against the server (the ready effect re-marks once this state settles). T-062.2.
        clearEditorSession()
      } else {
        setDirty(true) // local kept → it differs from the server, so it's unsaved
      }
      setConflict(null)
    },
    [conflict, md],
  )

  return {
    md,
    undo,
    docStatus,
    loadProgress,
    dirty,
    suggestedSemver: bumpPatch(currentSemver),
    saveVersion,
    exportJson,
    conflict,
    resolveConflict,
    invalidMissionId,
  }
}
