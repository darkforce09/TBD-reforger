import { describe, it, expect, beforeEach } from 'vitest'
import * as si from './slotSpatialIndex'
import type { SlotIcon } from './selectors'

// T-145 Phase 3.1 — the Rust-backed slotSpatialIndex (wasm SlotIndex). The grid math is pinned by
// slotIndex.parity (SlotIndex ≡ rbush); this locks the incremental glue + the exact pick semantics
// (nearest-in-box, no circular cutoff) through the wrapper.

const icon = (id: string, x: number, y: number): SlotIcon => ({ id, x, y, selected: false })
// Identity viewport: unproject([x,y]) = [x,y], so the pixel hit-radius maps 1:1 to world meters.
const vp = { unproject: (xy: number[]) => [xy[0], xy[1]] }

beforeEach(() => si.clear())

describe('slotSpatialIndex (Rust-backed pick index)', () => {
  it('pickRect returns ids inside the box (empty → []; reversed corners normalized)', () => {
    expect(si.pickRect(0, 0, 100, 100)).toEqual([])
    si.rebuild([icon('a', 10, 10), icon('b', 50, 50), icon('c', 500, 500)])
    expect(si.pickRect(0, 0, 100, 100).sort()).toEqual(['a', 'b'])
    expect(si.pickRect(100, 100, 1, 1).sort()).toEqual(['a', 'b']) // reversed → normalized
    expect(si.pickRect(400, 400, 600, 600)).toEqual(['c'])
  })

  it('pickNearest returns the nearest id in the box, else null', () => {
    si.rebuild([icon('a', 10, 10), icon('b', 12, 10), icon('c', 500, 500)])
    expect(si.pickNearest([10, 10], vp)).toBe('a')
    expect(si.pickNearest([12, 10], vp)).toBe('b')
    expect(si.pickNearest([300, 300], vp)).toBe(null) // empty area within the 4 m box
  })

  it('insert (dedup) + updatePositions + remove (swap-pop) keep queries correct', () => {
    si.rebuild([icon('a', 10, 10)])
    si.insert([icon('b', 20, 20), icon('a', 999, 999)]) // 'a' already present → skipped
    expect(si.pickRect(0, 0, 100, 100).sort()).toEqual(['a', 'b'])
    expect(si.pickNearest([999, 999], vp)).toBe(null) // 'a' was not moved by the skipped insert

    si.updatePositions({ b: { x: 900, y: 900 } })
    expect(si.pickRect(0, 0, 100, 100)).toEqual(['a']) // b moved out of the box
    expect(si.pickNearest([900, 900], vp)).toBe('b')

    si.remove(['a'])
    expect(si.pickRect(0, 0, 100, 100)).toEqual([])
    si.remove(['missing']) // no-op
    expect(si.pickNearest([900, 900], vp)).toBe('b')
  })

  it('clear resets to empty', () => {
    si.rebuild([icon('a', 10, 10), icon('b', 50, 50)])
    expect(si.pickRect(0, 0, 100, 100)).toHaveLength(2)
    si.clear()
    expect(si.pickRect(0, 0, 100, 100)).toEqual([])
  })
})
