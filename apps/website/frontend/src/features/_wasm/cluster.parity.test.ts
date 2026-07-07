import { describe, it, expect } from 'vitest'
import Supercluster from 'supercluster'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { getTerrain } from '@/features/tactical-map/coords/terrains'
import type { SlotIcon } from '@/features/tactical-map/state/selectors'

// Phase 3.0 spike (plan §9.1 criterion 5, cluster path) — the Rust ClusterIndex must be
// supercluster-compatible. The oracle is the REAL `supercluster` library driven through the same
// linear world→lng/lat normalization the app used (kept as a test-only devDep after Phase 3.1 wired
// the Rust index into slotClusterIndex.ts). Class S: on well-separated groups the clustering is
// order-independent → EXACT parity (cluster count, counts, centroids); on dense inputs (greedy result
// is KDBush-order dependent) the pinned invariant is point conservation on both sides.

const terrain = getTerrain('everon') // 12800²
const LNG_SPAN = 360
const LAT_SPAN = 170

interface Marker {
  x: number
  y: number
  count: number
}

const normLng = (x: number) => (x / terrain.width) * LNG_SPAN - 180
const normLat = (y: number) => (y / terrain.height) * LAT_SPAN - 85
const worldX = (lng: number) => ((lng + 180) / LNG_SPAN) * terrain.width
const worldY = (lat: number) => ((lat + 85) / LAT_SPAN) * terrain.height
const superZoom = (dz: number) => Math.max(0, Math.min(16, Math.round(dz + 8)))

/** The real supercluster library, fed the app's normalization — the parity oracle. */
function scOracle(
  icons: SlotIcon[],
): (bbox: [number, number, number, number], dz: number) => Marker[] {
  const sc = new Supercluster({ radius: 60, maxZoom: 16 })
  sc.load(
    icons.map((ic) => ({
      type: 'Feature' as const,
      properties: { id: ic.id },
      geometry: { type: 'Point' as const, coordinates: [normLng(ic.x), normLat(ic.y)] },
    })),
  )
  return (bbox, dz) => {
    const feats = sc.getClusters(
      [normLng(bbox[0]), normLat(bbox[1]), normLng(bbox[2]), normLat(bbox[3])],
      superZoom(dz),
    )
    return feats.map((f) => {
      const [lng, lat] = f.geometry.coordinates
      const isCluster = (f.properties as { cluster?: boolean }).cluster === true
      return {
        x: worldX(lng),
        y: worldY(lat),
        count: isCluster ? (f.properties as { point_count: number }).point_count : 1,
      }
    })
  }
}

/** ClusterResult (parallel columns) → Marker[]. */
function fromWasm(res: wasm.ClusterResult): Marker[] {
  const xs = res.xs
  const ys = res.ys
  const counts = res.counts
  const out: Marker[] = []
  for (let i = 0; i < counts.length; i++) out.push({ x: xs[i], y: ys[i], count: counts[i] })
  return out
}

function build(icons: SlotIcon[]): {
  ref: (bbox: [number, number, number, number], dz: number) => Marker[]
  got: (bbox: [number, number, number, number], dz: number) => Marker[]
} {
  const ref = scOracle(icons)
  const idx = new wasm.ClusterIndex(
    new Float32Array(icons.map((i) => i.x)),
    new Float32Array(icons.map((i) => i.y)),
    terrain.width,
    terrain.height,
  )
  return {
    ref,
    got: (bbox, dz) => fromWasm(idx.get_clusters(bbox[0], bbox[1], bbox[2], bbox[3], dz)),
  }
}

describe('ClusterIndex — criterion 5 cluster path: parity vs supercluster', () => {
  it('well-separated blobs: exact clusters + counts + centroids vs supercluster', () => {
    const centers: [number, number][] = [
      [3000, 3000],
      [6400, 6400],
      [9500, 9000],
    ]
    const per = 5
    const icons: SlotIcon[] = []
    let k = 0
    for (const [cx, cy] of centers) {
      for (let j = 0; j < per; j++) {
        const d = j * 8 - 16 // ±16 m — far inside the ~375 m radius @ super-zoom 2
        icons.push({ id: `b${k++}`, x: cx + d, y: cy - d, selected: false })
      }
    }
    const { ref, got } = build(icons)
    const bbox: [number, number, number, number] = [0, 0, terrain.width, terrain.height]
    const dz = -6 // super-zoom 2

    const r = ref(bbox, dz)
      .filter((m) => m.count > 1)
      .sort((a, b) => a.x - b.x)
    const g = got(bbox, dz)
      .filter((m) => m.count > 1)
      .sort((a, b) => a.x - b.x)

    expect(g.length).toBe(3)
    expect(r.length).toBe(3)
    for (let i = 0; i < 3; i++) {
      expect(g[i].count).toBe(per)
      expect(r[i].count).toBe(per)
      // Class S centroid tolerance (Web-Mercator sin/ln/atan/exp differ ≤ 1 ULP across libm).
      expect(Math.abs(g[i].x - r[i].x)).toBeLessThan(0.01)
      expect(Math.abs(g[i].y - r[i].y)).toBeLessThan(0.01)
    }
  })

  it('dense random input: both conserve all points at every cluster zoom', () => {
    const N = 2000
    const icons: SlotIcon[] = []
    let s = 0x51ed7a17 >>> 0
    const rnd = () => {
      s = (Math.imul(s, 1103515245) + 12345) >>> 0
      return s / 0x100000000
    }
    for (let i = 0; i < N; i++) {
      icons.push({
        id: `p${i}`,
        x: rnd() * terrain.width,
        y: rnd() * terrain.height,
        selected: false,
      })
    }
    const { ref, got } = build(icons)
    const bbox: [number, number, number, number] = [0, 0, terrain.width, terrain.height]

    for (const dz of [-6, -5, -4]) {
      const refTotal = ref(bbox, dz).reduce((a, m) => a + m.count, 0)
      const gotTotal = got(bbox, dz).reduce((a, m) => a + m.count, 0)
      expect(refTotal, `supercluster conserves @ dz ${dz}`).toBe(N)
      expect(gotTotal, `rust conserves @ dz ${dz}`).toBe(N)
    }
  })

  it('a coarse viewport sub-bbox returns clusters (both non-empty, both conserve)', () => {
    const N = 800
    const icons: SlotIcon[] = []
    let s = 0x1234abcd >>> 0
    const rnd = () => {
      s = (Math.imul(s, 1103515245) + 12345) >>> 0
      return s / 0x100000000
    }
    // Tightly pack points into the SW quadrant so a quadrant bbox holds them all.
    for (let i = 0; i < N; i++) {
      icons.push({ id: `q${i}`, x: rnd() * 6000, y: rnd() * 6000, selected: false })
    }
    const { ref, got } = build(icons)
    const bbox: [number, number, number, number] = [0, 0, 6400, 6400]
    const dz = -5

    const r = ref(bbox, dz)
    const g = got(bbox, dz)
    expect(g.length).toBeGreaterThan(0)
    expect(r.length).toBeGreaterThan(0)
    // All points sit inside the queried quadrant, so both must account for all N.
    expect(r.reduce((a, m) => a + m.count, 0)).toBe(N)
    expect(g.reduce((a, m) => a + m.count, 0)).toBe(N)
  })
})
