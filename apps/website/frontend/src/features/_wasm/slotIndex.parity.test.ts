import { describe, it, expect } from 'vitest'
import RBush from 'rbush'
import * as wasm from '@/wasm/pkg/map_engine_wasm'

// Phase 3.0 spike (criterion 5): the Rust grid SlotIndex must return the same result SET as the
// JS rbush (slotSpatialIndex / worldSpatialIndex) — Class S (set-equality, not layout identity).
// 100k pseudorandom points; rbush is the reference (the actual library the cutover replaces).
interface Item {
  minX: number
  minY: number
  maxX: number
  maxY: number
  i: number
}

const N = 100_000
const xs = new Float32Array(N)
const ys = new Float32Array(N)
{
  let s = 0x12345678 >>> 0
  const rnd = () => {
    s = (Math.imul(s, 1103515245) + 12345) >>> 0
    return s / 0x100000000
  }
  for (let k = 0; k < N; k++) {
    xs[k] = rnd() * 12800
    ys[k] = rnd() * 12800
  }
}

const tree = new RBush<Item>()
tree.load(
  Array.from({ length: N }, (_, k) => ({
    minX: xs[k],
    minY: ys[k],
    maxX: xs[k],
    maxY: ys[k],
    i: k,
  })),
)
const idx = wasm.SlotIndex.build(xs, ys, 256)

const asc = (a: number, b: number) => a - b

describe('map-engine-wasm SlotIndex — Class S set-equality vs rbush (100k points)', () => {
  it('holds all points', () => {
    expect(idx.size).toBe(N)
  })

  it('pick_rect == rbush.search over a probe battery', () => {
    const rects: [number, number, number, number][] = [
      [1000, 2000, 3000, 5000],
      [0, 0, 12800, 12800],
      [6000, 6000, 6100, 6100],
      [12700, 0, 13000, 500], // straddles the east edge
      [5000, 5000, 5001, 5001], // tiny
      [-500, -500, 250, 250], // straddles the SW corner
    ]
    for (const [a, b, c, d] of rects) {
      const ref = tree
        .search({ minX: a, minY: b, maxX: c, maxY: d })
        .map((it) => it.i)
        .sort(asc)
      const got = Array.from(idx.pick_rect(a, b, c, d)).sort(asc)
      expect(got).toEqual(ref)
    }
  })

  it('pick_nearest == rbush box-search + circular nearest', () => {
    const probes: [number, number, number][] = [
      [6400, 6400, 500],
      [0, 0, 300],
      [12800, 12800, 1000],
      [3333, 9999, 50],
      [100, 100, 5],
      [6400, 6400, 0], // zero radius → almost surely none
    ]
    for (const [x, y, rad] of probes) {
      let best = -1
      let bestD = Infinity
      for (const h of tree.search({ minX: x - rad, minY: y - rad, maxX: x + rad, maxY: y + rad })) {
        const dx = xs[h.i] - x
        const dy = ys[h.i] - y
        const d2 = dx * dx + dy * dy
        if (d2 <= rad * rad && d2 < bestD) {
          bestD = d2
          best = h.i
        }
      }
      expect(idx.pick_nearest(x, y, rad)).toBe(best)
    }
  })
})
