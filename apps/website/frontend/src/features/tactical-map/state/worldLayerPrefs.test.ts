// T-090.5.1 — worldLayerPrefs persistence + migration contract. mapStyle must seed from the
// legacy `tbd-mc-basemap-view` key (T-090.1 users keep their raster choice), the new
// `tbd-mc-world-layers` key wins once present, and setMapStyle dual-writes the legacy key
// until T-090.10.2. Module snapshots localStorage at import → each case stubs storage and
// re-imports fresh (pattern: basemapView.test.ts).
import { describe, it, expect, vi, beforeEach } from 'vitest'

const KEY = 'tbd-mc-world-layers'
const LEGACY = 'tbd-mc-basemap-view'

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
  return import('./worldLayerPrefs')
}

describe('worldLayerPrefs (N8 + legacy migration)', () => {
  beforeEach(() => vi.unstubAllGlobals())

  it('defaults: satellite style, spec toggle set (props off)', async () => {
    stubStorage()
    const mod = await importFresh()
    expect(mod.getMapStyle()).toBe('satellite')
    expect(mod.getClassToggles()).toEqual({
      roads: true,
      buildings: true,
      forest: true,
      trees: true,
      props: false,
      fences: true,
      airfield: true,
      contours: true,
      sea: true,
      heights: true,
      townLabels: true,
    })
  })

  it("migrates legacy 'map' and 'satellite' basemap prefs", async () => {
    stubStorage({ [LEGACY]: 'map' })
    expect((await importFresh()).getMapStyle()).toBe('map')

    stubStorage({ [LEGACY]: 'satellite' })
    expect((await importFresh()).getMapStyle()).toBe('satellite')
  })

  it('own key wins over the legacy key', async () => {
    stubStorage({ [KEY]: JSON.stringify({ mapStyle: 'hybrid' }), [LEGACY]: 'map' })
    expect((await importFresh()).getMapStyle()).toBe('hybrid')
  })

  it('garbage / broken storage falls back to defaults', async () => {
    stubStorage({ [KEY]: 'not json', [LEGACY]: 'topo' })
    expect((await importFresh()).getMapStyle()).toBe('satellite')

    stubStorage({ [KEY]: JSON.stringify({ mapStyle: 'sepia' }) })
    expect((await importFresh()).getMapStyle()).toBe('satellite')

    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('denied')
      },
      setItem: () => {
        throw new Error('denied')
      },
    })
    expect((await importFresh()).getMapStyle()).toBe('satellite')
  })

  it('setMapStyle persists JSON and dual-writes the legacy key', async () => {
    const store = stubStorage()
    const mod = await importFresh()
    mod.setMapStyle('map')
    expect(mod.getMapStyle()).toBe('map')
    expect(JSON.parse(store.get(KEY) ?? '{}')).toMatchObject({ mapStyle: 'map' })
    expect(store.get(LEGACY)).toBe('map')
  })

  it("hybrid dual-writes legacy 'satellite' (hybrid renders the satellite raster)", async () => {
    const store = stubStorage()
    const mod = await importFresh()
    mod.setMapStyle('hybrid')
    expect(store.get(LEGACY)).toBe('satellite')
    expect(JSON.parse(store.get(KEY) ?? '{}')).toMatchObject({ mapStyle: 'hybrid' })
  })

  it('setClassToggle persists and preserves the rest', async () => {
    const store = stubStorage()
    const mod = await importFresh()
    mod.setClassToggle('props', true)
    expect(mod.getClassToggles().props).toBe(true)
    expect(mod.getClassToggles().roads).toBe(true)
    const stored = JSON.parse(store.get(KEY) ?? '{}') as { classToggles?: unknown }
    expect(stored.classToggles).toMatchObject({ props: true, roads: true })
  })

  it('setClassToggle persists airfield toggle (T-152.5 G6 UI)', async () => {
    const store = stubStorage()
    const mod = await importFresh()
    mod.setClassToggle('airfield', false)
    expect(mod.getClassToggles().airfield).toBe(false)
    expect(mod.getClassToggles().roads).toBe(true)
    const stored = JSON.parse(store.get(KEY) ?? '{}') as { classToggles?: unknown }
    expect(stored.classToggles).toMatchObject({ airfield: false, roads: true })
  })

  it('partial stored toggles merge over defaults', async () => {
    stubStorage({ [KEY]: JSON.stringify({ mapStyle: 'satellite', classToggles: { sea: false } }) })
    const mod = await importFresh()
    expect(mod.getClassToggles().sea).toBe(false)
    expect(mod.getClassToggles().props).toBe(false)
    expect(mod.getClassToggles().buildings).toBe(true)
  })

  it('notifies subscribers on change', async () => {
    stubStorage()
    const mod = await importFresh()
    let fired = 0
    const off = mod.subscribeWorldLayerPrefs(() => fired++)
    mod.setMapStyle('hybrid')
    mod.setMapStyle('hybrid') // no-op — no second notify
    expect(fired).toBe(1)
    off()
    mod.setMapStyle('map')
    expect(fired).toBe(1)
  })

  // T-090.5.5 — the world-object debug HUD setting (Ctrl+Alt+D). Off by default; the toggle
  // flips + persists + notifies; a stored `true` is read back on import.
  it('worldmapDebug: off by default, toggles + persists + notifies', async () => {
    const store = stubStorage()
    const mod = await importFresh()
    expect(mod.getWorldmapDebug()).toBe(false)

    let fired = 0
    const off = mod.subscribeWorldLayerPrefs(() => fired++)
    mod.toggleWorldmapDebug()
    expect(mod.getWorldmapDebug()).toBe(true)
    expect(fired).toBe(1)
    expect(JSON.parse(store.get(KEY) ?? '{}')).toMatchObject({ worldmapDebug: true })

    mod.toggleWorldmapDebug()
    expect(mod.getWorldmapDebug()).toBe(false)
    expect(fired).toBe(2)
    off()
  })

  it('worldmapDebug: a stored true survives a reload; other prefs preserved', async () => {
    stubStorage({ [KEY]: JSON.stringify({ mapStyle: 'hybrid', worldmapDebug: true }) })
    const mod = await importFresh()
    expect(mod.getWorldmapDebug()).toBe(true)
    expect(mod.getMapStyle()).toBe('hybrid')
  })
})
