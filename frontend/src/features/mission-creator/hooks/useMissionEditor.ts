// Phase 9 editor lifecycle — wraps useMissionDoc (local Y.Doc + y-indexeddb) and adds the
// backend layer: load/hydrate the current server version, prompt on a local-vs-server
// conflict (Decisions log), track unsaved changes, manual Save Version (immutable semver
// snapshot via POST /missions/:id/versions), and Export (download the camelCase mod JSON).
// Debounced autosave stays LOCAL (y-indexeddb) — the versions API has no draft/overwrite route.

import { useCallback, useEffect, useRef, useState } from 'react'
import { hydrateMissionDoc, useMapStore, type MissionDoc } from '@/features/tactical-map'
import { LOCAL_ORIGIN } from '@/features/tactical-map'
import { api } from '@/api/client'
import { useMissionDoc, type MissionDocHandle } from './useMissionDoc'
import { compileMission } from '../compiler/compile'
import { toMissionExport } from '../compiler/exportSchema'

export interface SaveResult {
  ok: boolean
  error?: string
}

export interface MissionEditorHandle extends MissionDocHandle {
  dirty: boolean
  suggestedSemver: string
  saveVersion: (semver: string, notes?: string) => Promise<SaveResult>
  exportJson: () => void
  /** Server payload awaiting a keep-local / load-server decision; null when none. */
  conflict: Record<string, unknown> | null
  resolveConflict: (choice: 'local' | 'server') => void
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

  // After the local snapshot syncs, reconcile with the server's current version.
  const onSynced = useCallback(
    (md: MissionDoc) => {
      if (!missionId) return
      api
        .get(`/missions/${missionId}`)
        .then((res) => {
          if (!mounted.current) return
          const version = res.data?.current_version as
            | { semver?: string; json_payload?: Record<string, unknown> }
            | undefined
          setCurrentSemver(version?.semver ?? null)
          const payload = version?.json_payload
          if (!payload) return // nothing on the server → keep local
          if (hasLocalContent(md)) setConflict(payload) // prompt the user
          else hydrateMissionDoc(md, payload) // empty local → adopt server
        })
        .catch(() => {
          /* mission not on the API (e.g. ad-hoc id) → stay local-only */
        })
    },
    [missionId],
  )

  const { md, undo } = useMissionDoc(missionId, { onSynced })

  // Mark unsaved on any local (user) edit; INIT/persistence-origin updates don't count.
  useEffect(() => {
    mounted.current = true
    const onUpdate = (_u: Uint8Array, origin: unknown) => {
      if (origin === LOCAL_ORIGIN) setDirty(true)
    }
    md.doc.on('update', onUpdate)
    return () => {
      mounted.current = false
      md.doc.off('update', onUpdate)
    }
  }, [md])

  const saveVersion = useCallback(
    async (semver: string, notes?: string): Promise<SaveResult> => {
      if (!missionId) return { ok: false, error: 'No mission id' }
      const payload = compileMission(useMapStore.getState())
      try {
        await api.post(`/missions/${missionId}/versions`, {
          semver,
          payload,
          editor_notes: notes ?? '',
        })
        if (mounted.current) {
          setCurrentSemver(semver)
          setDirty(false)
        }
        return { ok: true }
      } catch (e) {
        const status = (e as { response?: { status?: number } }).response?.status
        return {
          ok: false,
          error: status === 409 ? `Version ${semver} already exists` : 'Could not save version',
        }
      }
    },
    [missionId],
  )

  const exportJson = useCallback(() => {
    const state = useMapStore.getState()
    const payload = compileMission(state)
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
        setDirty(false)
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
    dirty,
    suggestedSemver: bumpPatch(currentSemver),
    saveVersion,
    exportJson,
    conflict,
    resolveConflict,
  }
}
