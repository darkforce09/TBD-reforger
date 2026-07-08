// T-151.1 L5 — Class S oracle for the pyramid LOD. The wgpu map-style path selects tiles with the
// SAME `computeLod` the Deck path uses (extracted to basemapResolve.ts); this pins its output on
// scripted (viewState, viewBounds, mode) tuples to golden literals — structural set equality — so a
// drift in the level pick, viewport cull, 64-tile cap, or the `tileUrl` south-first Y inversion
// fails loudly. Everon: 12800², log2(12800/256) = log2(50) ≈ 5.6439.
import { describe, expect, it } from 'vitest'
import { computeLod, type Lod, type Resolved } from '../layers/basemapResolve'
import { tileUrl } from '../layers/tileUrl'
import { getTerrain } from '../coords/terrains'
import type { MapViewState } from '../types'

const everon = getTerrain('everon')
const TMPL = '/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp'
const pyramid: Resolved = { mode: 'pyramid', template: TMPL, minZoom: 0, maxZoom: 6 }

const vs = (zoom: number): MapViewState => ({ target: [6400, 6400], zoom, minZoom: -6, maxZoom: 6 })
type Bounds = [number, number, number, number] | null

interface Case {
  name: string
  resolved: Resolved
  view: MapViewState
  bounds: Bounds
  visible: boolean
  expect: Lod
}

const cases: Case[] = [
  {
    // Full extent at zoom 0: pre-cap z=6 (ceil 5.64), the 64-tile cap drops to z=3 (8×8=64).
    name: 'full extent, zoom 0 → z3 (cap)',
    resolved: pyramid,
    view: vs(0),
    bounds: null,
    visible: true,
    expect: { kind: 'pyramid', z: 3, txMin: 0, txMax: 7, tyMin: 0, tyMax: 7, template: TMPL },
  },
  {
    name: 'full extent, zoom -2 → z3 (cap, same full-extent tile budget)',
    resolved: pyramid,
    view: vs(-2),
    bounds: null,
    visible: true,
    expect: { kind: 'pyramid', z: 3, txMin: 0, txMax: 7, tyMin: 0, tyMax: 7, template: TMPL },
  },
  {
    name: 'full extent, explicit bounds, zoom 0 → z3',
    resolved: pyramid,
    view: vs(0),
    bounds: [0, 0, 12800, 12800],
    visible: true,
    expect: { kind: 'pyramid', z: 3, txMin: 0, txMax: 7, tyMin: 0, tyMax: 7, template: TMPL },
  },
  {
    // zoom -6 → pre-cap z=clamp(ceil(-0.36),0,6)=0, one tile covers the world.
    name: 'zoomed out, zoom -6 → z0 single tile',
    resolved: pyramid,
    view: vs(-6),
    bounds: null,
    visible: true,
    expect: { kind: 'pyramid', z: 0, txMin: 0, txMax: 0, tyMin: 0, tyMax: 0, template: TMPL },
  },
  {
    // zoom 2, 800 m window → z6 (25 tiles ≤ 64). twx=200 → 6000/200=30, 6800/200=34.
    name: 'zoomed in 800m window, zoom 2 → z6',
    resolved: pyramid,
    view: vs(2),
    bounds: [6000, 6000, 6800, 6800],
    visible: true,
    expect: { kind: 'pyramid', z: 6, txMin: 30, txMax: 34, tyMin: 30, tyMax: 34, template: TMPL },
  },
  {
    // zoom 1, 2000 m window → pre-cap z6 gives 11×11=121 > 64, drops to z5 (36 tiles).
    name: '2000m window, zoom 1 → z5 (cap)',
    resolved: pyramid,
    view: vs(1),
    bounds: [5000, 5000, 7000, 7000],
    visible: true,
    expect: { kind: 'pyramid', z: 5, txMin: 12, txMax: 17, tyMin: 12, tyMax: 17, template: TMPL },
  },
  {
    // zoom 6, tiny 200 m window near center → z6, 2×2 tiles.
    name: 'tiny window, zoom 6 → z6 4 tiles',
    resolved: pyramid,
    view: vs(6),
    bounds: [6300, 6300, 6500, 6500],
    visible: true,
    expect: { kind: 'pyramid', z: 6, txMin: 31, txMax: 32, tyMin: 31, tyMax: 32, template: TMPL },
  },
  {
    name: 'unified mode → none (the GPU picks the mip per fragment)',
    resolved: { mode: 'unified', unifiedUrl: '/x.tbd-sat', minZoom: 0, maxZoom: 6 },
    view: vs(0),
    bounds: null,
    visible: true,
    expect: { kind: 'none' },
  },
  {
    name: 'single-bitmap → single',
    resolved: { mode: 'single-bitmap', image: '/map-assets/everon/full.webp', minZoom: 0, maxZoom: 6 },
    view: vs(0),
    bounds: null,
    visible: true,
    expect: { kind: 'single', image: '/map-assets/everon/full.webp' },
  },
  {
    name: 'none mode → none',
    resolved: { mode: 'none', minZoom: 0, maxZoom: 6 },
    view: vs(0),
    bounds: null,
    visible: true,
    expect: { kind: 'none' },
  },
  {
    name: 'loading mode → none',
    resolved: { mode: 'loading', minZoom: 0, maxZoom: 6 },
    view: vs(0),
    bounds: null,
    visible: true,
    expect: { kind: 'none' },
  },
  {
    name: 'not visible → none',
    resolved: pyramid,
    view: vs(0),
    bounds: null,
    visible: false,
    expect: { kind: 'none' },
  },
]

describe('computeLod (T-151.1 L5 pyramid tile-set oracle)', () => {
  it.each(cases)('$name', ({ resolved, view, bounds, visible, expect: want }) => {
    expect(computeLod(resolved, view, bounds, everon, visible)).toEqual(want)
  })

  it('tileUrl is the sole south-first→XYZ Y inversion at z3', () => {
    // ty=0 is the SOUTHERN world edge → on-disk northernmost row (2^3-1-0 = 7).
    expect(tileUrl(TMPL, 3, 0, 0)).toBe('/map-assets/everon/tiles/satellite/3/0/7.webp')
    // ty=7 (north) → disk row 0.
    expect(tileUrl(TMPL, 3, 7, 7)).toBe('/map-assets/everon/tiles/satellite/3/7/0.webp')
    // zoom 6 window corner (tuple 5): world tile (30,30) → disk row 63-30 = 33.
    expect(tileUrl(TMPL, 6, 30, 30)).toBe('/map-assets/everon/tiles/satellite/6/30/33.webp')
  })
})
