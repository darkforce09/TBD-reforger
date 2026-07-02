// T-130.5 F4-03 — adopted-server marker contract. The marker is what lets a NEW tab's
// cold boot skip the conflict prompt when local IndexedDB derives from the server's
// current version (adopt / save lineage); "Keep local draft" clears it so genuine
// divergence still prompts. localStorage is stubbed (vitest runs in the node env).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  clearAdoptedServerVersion,
  markServerVersionAdopted,
  readAdoptedServerVersion,
} from './editorSession'

function stubLocalStorage() {
  const store = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
  })
  return store
}

describe('adopted-server marker (F4-03)', () => {
  let store: Map<string, string>

  beforeEach(() => {
    store = stubLocalStorage()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('round-trips the adopted semver per mission', () => {
    markServerVersionAdopted('mission-a', '1.2.3')
    expect(readAdoptedServerVersion('mission-a')).toBe('1.2.3')
    // Another mission's marker is independent.
    expect(readAdoptedServerVersion('mission-b')).toBeNull()
  })

  it('null semver clears the marker (version-less mission)', () => {
    markServerVersionAdopted('mission-a', '1.2.3')
    markServerVersionAdopted('mission-a', null)
    expect(readAdoptedServerVersion('mission-a')).toBeNull()
  })

  it('clearAdoptedServerVersion drops it ("Keep local draft")', () => {
    markServerVersionAdopted('mission-a', '1.2.3')
    clearAdoptedServerVersion('mission-a')
    expect(readAdoptedServerVersion('mission-a')).toBeNull()
  })

  it('a corrupt or foreign record reads as null, never throws', () => {
    store.set('tbd-editor-adopted:mission-a', '{not json')
    expect(readAdoptedServerVersion('mission-a')).toBeNull()
    store.set('tbd-editor-adopted:mission-a', JSON.stringify({ semver: 42 }))
    expect(readAdoptedServerVersion('mission-a')).toBeNull()
  })

  it('no-ops safely when storage is unavailable', () => {
    vi.unstubAllGlobals() // node env: no localStorage global at all
    expect(() => markServerVersionAdopted('mission-a', '1.0.0')).not.toThrow()
    expect(readAdoptedServerVersion('mission-a')).toBeNull()
    expect(() => clearAdoptedServerVersion('mission-a')).not.toThrow()
  })
})
