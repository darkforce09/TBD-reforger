import { describe, it, expect } from 'vitest'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import * as sci from '@/features/tactical-map/state/slotClusterIndex'
import { getTerrain } from '@/features/tactical-map/coords/terrains'
import type { SlotIcon } from '@/features/tactical-map/state/selectors'

// Phase 3.0 spike (plan §9.1 criterion 5, cluster path) — the Rust ClusterIndex must be
// supercluster-compatible. Class S: on well-separated groups the clustering is order-independent, so
// parity is EXACT (same cluster count, counts, centroids vs the real `slotClusterIndex` supercluster);
// on dense inputs (where supercluster's greedy result is KDBush-order dependent) the pinned invariant
// is point conservation on both sides. The Rust index mirrors the same normalize→fround(mercator)→
// cluster→inverse-project pipeline, so the projection + centroid math is what parity locks.

const terrain = getTerrain('everon') // 12800²

interface Marker {
  x: number
  y: number
  count: number
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

/** supercluster ClusterMarker[] → Marker[]. */
function fromSci(markers: { x: number; y: number; count: number }[]): Marker[] {
  return markers.map((m) => ({ x: m.x, y: m.y, count: m.count }))
}

function build(icons: SlotIcon[]): {
  ref: (bbox: [number, number, number, number], dz: number) => Marker[]
  got: (bbox: [number, number, number, number], dz: number) => Marker[]
} {
  sci.clear()
  sci.setTerrain(terrain)
  sci.rebuild(icons)
  const idx = new wasm.ClusterIndex(
    new Float32Array(icons.map((i) => i.x)),
    new Float32Array(icons.map((i) => i.y)),
    terrain.width,
    terrain.height,
  )
  return {
    ref: (bbox, dz) => fromSci(sci.getClusters(bbox, dz)),
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

  it('a coarse viewport sub-bbox returns clusters (both non-empty, both conserve within reason)', () => {
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
