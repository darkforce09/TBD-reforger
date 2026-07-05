// T-090.5.3 — World spatial index: separate-instance guarantee (W3) + pick semantics the
// worker relies on (W2 backing structure). The slot index must stay untouched by world
// inserts — they are different domains (authored slots vs streamed terrain objects).
import { describe, it, expect } from 'vitest'
import { createWorldSpatialIndex } from './worldSpatialIndex'
import * as slotSpatialIndex from './slotSpatialIndex'

const entries = (rows: [string, number, number, string][]) =>
  rows.map(([id, x, y, cls]) => ({ id, x, y, cls }))

describe('createWorldSpatialIndex (W3 — no shared singleton)', () => {
  it('two instances are fully independent', () => {
    const a = createWorldSpatialIndex()
    const b = createWorldSpatialIndex()
    a.insertChunk('0_0', entries([['0_0:0', 10, 10, 'building']]))
    expect(a.size()).toBe(1)
    expect(b.size()).toBe(0)
    expect(b.pickRect([0, 0, 100, 100])).toEqual([])
  })

  it('does not leak into the slot spatial index module (separate domain)', () => {
    const world = createWorldSpatialIndex()
    world.insertChunk('0_0', entries([['0_0:0', 50, 50, 'tree']]))
    // The slot module is a singleton for authored slots — world inserts must not reach it.
    expect(slotSpatialIndex.pickRect(0, 0, 12800, 12800)).toEqual([])
  })
})

describe('pick semantics', () => {
  const make = () => {
    const idx = createWorldSpatialIndex()
    idx.insertChunk(
      '1_1',
      entries([
        ['1_1:0', 600, 600, 'building'],
        ['1_1:1', 610, 600, 'tree'],
        ['1_1:2', 700, 700, 'building'],
      ]),
    )
    idx.insertChunk('2_1', entries([['2_1:0', 1030, 600, 'building']]))
    return idx
  }

  it('pickNearest returns the closest entry within radius, else null', () => {
    const idx = make()
    expect(idx.pickNearest(601, 600, 50)).toBe('1_1:0')
    expect(idx.pickNearest(609, 600, 50)).toBe('1_1:1')
  })

  it('rejects box-corner hits beyond the circular radius', () => {
    const idx = make()
    // (592, 608) is inside the 10 m search box of (600, 600) but 11.3 m away (and away
    // from the (610, 600) tree), so the circular-radius check must reject it.
    expect(idx.pickNearest(592, 608, 10)).toBeNull()
    expect(idx.pickNearest(0, 0, 5)).toBeNull()
  })

  it('honors the class filter (N4 pick gates)', () => {
    const idx = make()
    const buildingsOnly = (cls: string) => cls === 'building'
    expect(idx.pickNearest(609, 600, 50, buildingsOnly)).toBe('1_1:0')
    expect(idx.pickRect([590, 590, 620, 610], buildingsOnly)).toEqual(['1_1:0'])
  })

  it('pickRect normalizes reversed corners and spans chunks', () => {
    const idx = make()
    const ids = idx.pickRect([1100, 800, 500, 500]).sort()
    expect(ids).toEqual(['1_1:0', '1_1:1', '1_1:2', '2_1:0'])
  })

  it('removeChunk evicts only that chunk; clear drops everything', () => {
    const idx = make()
    idx.removeChunk('1_1')
    expect(idx.size()).toBe(1)
    expect(idx.pickRect([0, 0, 12800, 12800])).toEqual(['2_1:0'])
    idx.removeChunk('missing') // no-op
    idx.clear()
    expect(idx.size()).toBe(0)
    expect(idx.pickRect([0, 0, 12800, 12800])).toEqual([])
  })

  it('re-inserting a chunk replaces its previous entries (idempotent hydrate)', () => {
    const idx = make()
    idx.insertChunk('1_1', entries([['1_1:0', 900, 900, 'building']]))
    expect(idx.size()).toBe(2)
    expect(idx.pickRect([590, 590, 710, 710])).toEqual([])
    expect(idx.pickNearest(900, 900, 5)).toBe('1_1:0')
  })
})
