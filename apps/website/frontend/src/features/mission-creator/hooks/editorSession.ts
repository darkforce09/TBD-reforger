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

// ---------------------------------------------------------------------------
// Adopted-server marker (T-130.5 / F4-03 — closes the T-127 U1 new-tab gap).
//
// The warm sessionStorage record above is per-tab, so a NEW tab always cold-boots:
// GET /missions/:id → local IndexedDB has content AND the server has a payload →
// the conflict prompt fired even when the local copy IS the server version the user
// just adopted (or saved). This marker records, in localStorage (cross-tab, survives
// tab close), which server semver the local IndexedDB lineage derives from:
//   - written on initial server hydrate, on "Load saved version", and on Save Version
//   - cleared on "Keep local draft" (local now knowingly diverges → next cold boot
//     must re-prompt, mirroring the pre-existing same-tab semantics)
// A cold boot that finds marker.semver === the server's current semver trusts local
// IDB silently — any delta is the user's own unsaved edits, not divergence. Semver is
// the identity (unique per mission server-side); no payload hash — hashing a multi-MB
// payload per boot costs more than the residual risk of a same-semver payload swap,
// which the immutable versions API doesn't allow.

const ADOPTED_KEY_PREFIX = 'tbd-editor-adopted:'

interface AdoptedMarker {
  semver: string
  at: number
}

/** Record that local IndexedDB now derives from server version `semver`. */
export function markServerVersionAdopted(missionId: string, semver: string | null): void {
  try {
    if (!semver) {
      localStorage.removeItem(ADOPTED_KEY_PREFIX + missionId)
      return
    }
    const marker: AdoptedMarker = { semver, at: Date.now() }
    localStorage.setItem(ADOPTED_KEY_PREFIX + missionId, JSON.stringify(marker))
  } catch {
    /* storage disabled/full → cold boots fall back to the conflict prompt */
  }
}

/** The server semver the local copy was adopted from, or null if none/unparseable. */
export function readAdoptedServerVersion(missionId: string): string | null {
  try {
    const raw = localStorage.getItem(ADOPTED_KEY_PREFIX + missionId)
    if (!raw) return null
    const marker = JSON.parse(raw) as AdoptedMarker
    if (typeof marker.semver !== 'string' || !marker.semver) return null
    return marker.semver
  } catch {
    return null
  }
}

export function clearAdoptedServerVersion(missionId: string): void {
  try {
    localStorage.removeItem(ADOPTED_KEY_PREFIX + missionId)
  } catch {
    /* ignore */
  }
}
