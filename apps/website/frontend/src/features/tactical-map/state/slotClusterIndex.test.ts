import { describe, it, expect, beforeEach } from 'vitest'
import * as sci from './slotClusterIndex'
import { getTerrain } from '../coords/terrains'
import type { SlotIcon } from './selectors'

// T-145 Phase 3.1 — the Rust-backed slotClusterIndex wrapper. The clustering math is pinned by
// cluster.parity (Rust ≡ supercluster); this locks the incremental glue: rebuild/insert/remove/clear
// bookkeeping, leaf-id resolution through the row→id table, and the pan-stable version counter.

const terrain = getTerrain('everon')
const icon = (id: string, x: number, y: number): SlotIcon => ({ id, x, y, selected: false })
const spread = (n: number): SlotIcon[] =>
  Array.from({ length: n }, (_, i) => icon(`s${i}`, (i * 17) % 12000, (i * 37) % 12000))
const full: [number, number, number, number] = [0, 0, terrain.width, terrain.height]
const total = (dz: number) => sci.getClusters(full, dz).reduce((a, m) => a + m.count, 0)

beforeEach(() => {
  sci.clear()
  sci.setTerrain(terrain)
})

describe('slotClusterIndex (Rust-backed wrapper)', () => {
  it('empty index returns no clusters', () => {
    expect(sci.getClusters(full, -6)).toEqual([])
    expect(sci.getClusterMarkers(-6)).toEqual([])
  })

  it('rebuild → getClusters conserves every point', () => {
    sci.rebuild(spread(600))
    expect(total(-6)).toBe(600)
  })

  it('insert skips duplicates; remove drops via swap-pop (conservation holds)', () => {
    sci.rebuild([icon('a', 100, 100), icon('b', 200, 200)])
    sci.insert([icon('c', 300, 300), icon('a', 999, 999)]) // 'a' already present → skipped
    expect(total(-6)).toBe(3)
    sci.remove(['b'])
    expect(total(-6)).toBe(2)
    sci.remove(['missing']) // no-op
    expect(total(-6)).toBe(2)
  })

  it('a lone far leaf reports count 1 + its id', () => {
    sci.rebuild([icon('solo', 6400, 6400)])
    const markers = sci.getClusters(full, -6)
    expect(markers).toHaveLength(1)
    expect(markers[0].count).toBe(1)
    expect(markers[0].id).toBe('solo')
  })

  it('getClusterMarkers is stable on identical re-query, bumps on zoom-bucket change', () => {
    sci.rebuild(spread(600))
    sci.getClusterMarkers(-6)
    const v1 = sci.getClusterMarkersVersion()
    sci.getClusterMarkers(-6) // same bucket, no edit → no recompute
    expect(sci.getClusterMarkersVersion()).toBe(v1)
    sci.getClusterMarkers(-4) // super-zoom 2 → 4 → recompute + bump
    expect(sci.getClusterMarkersVersion()).toBeGreaterThan(v1)
  })

  it('clear resets to empty', () => {
    sci.rebuild(spread(50))
    expect(total(-6)).toBe(50)
    sci.clear()
    expect(sci.getClusters(full, -6)).toEqual([])
  })
})
