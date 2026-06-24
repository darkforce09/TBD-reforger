// Warm editor session marker (T-062.2). A small sessionStorage record written once the
// editor reaches `ready`. On a subsequent boot of the SAME mission within the TTL — e.g. a
// Vite HMR full reload after an alt-tab — useMissionEditor.onSynced can trust the local
// IndexedDB copy and SKIP the multi-MB `GET /missions/:id` (+ its conflict prompt), since
// y-indexeddb has already replayed the slots and meta back into the Y.Doc.
//
// Tradeoff: the warm path trusts local IDB. Remote server changes made since this session
// last synced are NOT detected until a cold load (no warm record, or a different mission /
// expired TTL → the normal GET + hydrate/conflict path runs and re-marks).
//
// Scoped to sessionStorage (per-tab, cleared when the tab closes) — a brand-new tab is
// always a cold load.

const KEY = 'tbd-editor-session'
const TTL_MS = 24 * 60 * 60 * 1000 // 24h

export interface EditorSession {
  missionId: string
  readyAt: number
  slotCount: number
  currentSemver: string | null
}

export function markEditorSessionReady(
  missionId: string,
  fields: { slotCount: number; currentSemver: string | null },
): void {
  try {
    const session: EditorSession = {
      missionId,
      readyAt: Date.now(),
      slotCount: fields.slotCount,
      currentSemver: fields.currentSemver,
    }
    sessionStorage.setItem(KEY, JSON.stringify(session))
  } catch {
    /* storage disabled/full → warm path simply won't engage */
  }
}

/** The warm session for `missionId`, or null if missing, for a different mission, expired,
 *  or unparseable. */
export function readWarmEditorSession(missionId: string): EditorSession | null {
  try {
    const raw = sessionStorage.getItem(KEY)
    if (!raw) return null
    const session = JSON.parse(raw) as EditorSession
    if (session.missionId !== missionId) return null
    if (Date.now() - session.readyAt > TTL_MS) return null
    return session
  } catch {
    return null
  }
}

export function clearEditorSession(): void {
  try {
    sessionStorage.removeItem(KEY)
  } catch {
    /* ignore */
  }
}
