// T-090.1.1 — basemapView persistence contract (dual-view N8 / M5). A persisted 'map' must be
// honored on load (the T-127 map→satellite coercion is gone now that the Map pyramid ships);
// garbage falls back to the satellite default. The module snapshots localStorage at import, so
// each case stubs storage first and re-imports fresh.
import { describe, it, expect, vi, beforeEach } from 'vitest'

function stubStorage(initial: Record<string, string> = {}) {
  const store = new Map(Object.entries(initial))
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
  })
  return store
}

async function importFresh() {
  vi.resetModules()
  return import('./basemapView')
}

describe('basemapView persistence (N8)', () => {
  beforeEach(() => vi.unstubAllGlobals())

  it("honors a persisted 'map' across reload (M5)", async () => {
    stubStorage({ 'tbd-mc-basemap-view': 'map' })
    const mod = await importFresh()
    expect(mod.getBasemapView()).toBe('map')
  })

  it("honors a persisted 'satellite'", async () => {
    stubStorage({ 'tbd-mc-basemap-view': 'satellite' })
    const mod = await importFresh()
    expect(mod.getBasemapView()).toBe('satellite')
  })

  it('falls back to the satellite default on garbage / missing / broken storage', async () => {
    stubStorage({ 'tbd-mc-basemap-view': 'topo' })
    expect((await importFresh()).getBasemapView()).toBe('satellite')

    stubStorage()
    expect((await importFresh()).getBasemapView()).toBe('satellite')

    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('denied')
      },
      setItem: () => {
        throw new Error('denied')
      },
    })
    expect((await importFresh()).getBasemapView()).toBe('satellite')
  })

  it('setBasemapView persists the choice and notifies', async () => {
    const store = stubStorage()
    const mod = await importFresh()
    mod.setBasemapView('map')
    expect(mod.getBasemapView()).toBe('map')
    expect(store.get('tbd-mc-basemap-view')).toBe('map')
  })
})
