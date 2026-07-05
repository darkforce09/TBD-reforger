// T-090.8.1 — forestMass pure-core gates: TBDD decode round-trip (wire format locked @
// T-090.3.2), marching-squares geometry on fixture grids (F3 fill shapes, saddle
// determinism, F6 reproducibility), and the N3 fill-α ladder spot checks.
import { describe, it, expect } from 'vitest'
import {
  DENSITY_ISO,
  EMPTY_FOREST_GEOMETRY,
  FOREST_FILL_RGB,
  TBDD_HEADER_BYTES,
  decodeTBDD,
  forestFillAlpha,
  forestMassFromCorners,
} from './forestMass'

/** Test-local TBDD encoder — byte-for-byte the density-grid.mjs layout (16 B header + LE
 *  u16 channels) so decode is exercised against the real wire format. */
function encodeTbdd(channels: Uint16Array[], cols: number, rows: number, cellM = 32): Uint8Array {
  const bytes = new Uint8Array(TBDD_HEADER_BYTES + channels.length * cols * rows * 2)
  bytes.set([0x54, 0x42, 0x44, 0x44]) // 'TBDD'
  const dv = new DataView(bytes.buffer)
  dv.setUint16(4, 1, true)
  dv.setUint16(6, cellM, true)
  dv.setUint16(8, cols, true)
  dv.setUint16(10, rows, true)
  bytes[12] = channels.length
  for (const [c, ch] of channels.entries()) {
    const base = TBDD_HEADER_BYTES + c * cols * rows * 2
    for (let k = 0; k < ch.length; k++) dv.setUint16(base + 2 * k, ch[k], true)
  }
  return bytes
}

/** Slice one ring (closed) out of the flat geometry as [x,y][] for shape assertions. */
function ringAt(geo: { fillPositions: Float32Array; fillStartIndices: Uint32Array }, n: number): [number, number][] {
  const start = geo.fillStartIndices[n]
  const end = n + 1 < geo.fillStartIndices.length ? geo.fillStartIndices[n + 1] : geo.fillPositions.length / 2
  const out: [number, number][] = []
  for (let v = start; v < end; v++) out.push([geo.fillPositions[2 * v], geo.fillPositions[2 * v + 1]])
  return out
}

describe('decodeTBDD (T-090.3.2 wire format)', () => {
  it('round-trips a synthetic two-channel grid', () => {
    const tree = Uint16Array.from({ length: 17 * 17 }, (_, k) => k % 7)
    const rock = Uint16Array.from({ length: 17 * 17 }, (_, k) => (k * 3) % 5)
    const grid = decodeTBDD(encodeTbdd([tree, rock], 17, 17))
    expect(grid.version).toBe(1)
    expect(grid.cellM).toBe(32)
    expect(grid.cols).toBe(17)
    expect(grid.rows).toBe(17)
    expect(grid.channels).toHaveLength(2)
    expect([...grid.channels[0]]).toEqual([...tree])
    expect([...grid.channels[1]]).toEqual([...rock]) // rock decodes (styling deferred to P4)
  })

  it('throws on bad magic and truncated bodies', () => {
    const good = encodeTbdd([new Uint16Array(4), new Uint16Array(4)], 2, 2)
    const bad = Uint8Array.from(good)
    bad[0] = 0x58
    expect(() => decodeTBDD(bad)).toThrow(/magic/)
    expect(() => decodeTBDD(good.slice(0, 20))).toThrow(/truncated/)
    expect(() => decodeTBDD(good.slice(0, 8))).toThrow(/short/)
  })

  it('reads a non-zero-offset view correctly (worker slices arrive offset)', () => {
    const buf = new Uint8Array(encodeTbdd([Uint16Array.from([1, 2, 3, 4]), new Uint16Array(4)], 2, 2))
    const padded = new Uint8Array(buf.length + 8)
    padded.set(buf, 8)
    expect([...decodeTBDD(padded.subarray(8)).channels[0]]).toEqual([1, 2, 3, 4])
  })
})

describe('forestMassFromCorners (marching squares)', () => {
  it('empty grid → empty geometry', () => {
    const geo = forestMassFromCorners(new Uint16Array(9), 3, 3, 0, 0, 32)
    expect(geo.fillPositions).toHaveLength(0)
    expect(geo.fillStartIndices).toHaveLength(0)
    expect(geo.outlineSegments).toHaveLength(0)
  })

  it('all-dense grid → one closed full quad per cell, no contour', () => {
    const geo = forestMassFromCorners(Uint16Array.from({ length: 9 }, () => 4), 3, 3, 100, 200, 32)
    expect(geo.fillStartIndices).toHaveLength(4) // 2×2 cells
    expect(geo.outlineSegments).toHaveLength(0)
    const ring = ringAt(geo, 0)
    expect(ring).toHaveLength(5) // 4 corners + closing vertex
    expect(ring[0]).toEqual(ring[4])
    expect(ring).toContainEqual([100, 200])
    expect(ring).toContainEqual([132, 232])
  })

  it('single dense corner → interpolated triangle at the iso crossing', () => {
    // 2×2 grid = one cell; only c00 dense (4 trees). iso 1 ⇒ t = (1−4)/(0−4) = 0.75.
    const geo = forestMassFromCorners(Uint16Array.from([4, 0, 0, 0]), 2, 2, 0, 0, 32)
    expect(geo.fillStartIndices).toHaveLength(1)
    const ring = ringAt(geo, 0)
    expect(ring).toHaveLength(4) // triangle + close
    expect(ring[0]).toEqual([0, 0])
    expect(ring[1]).toEqual([24, 0]) // 0.75 · 32 toward c10
    expect(ring[2]).toEqual([0, 24]) // 0.75 · 32 toward c01
    expect(geo.outlineSegments).toHaveLength(4)
    expect([...geo.outlineSegments]).toEqual([24, 0, 0, 24])
  })

  it('count-exactly-iso corner collapses to zero area but keeps neighbours contoured', () => {
    // c00 = 1 (== iso): crossing sits on the corner → degenerate ring dropped.
    const lone = forestMassFromCorners(Uint16Array.from([1, 0, 0, 0]), 2, 2, 0, 0, 32)
    expect(lone.fillStartIndices).toHaveLength(0)
    expect(lone.outlineSegments).toHaveLength(0)
    // c00 = 1 next to dense c10: triangle survives and the corner-hugging contour edge stays.
    const edge = forestMassFromCorners(Uint16Array.from([1, 4, 0, 0]), 2, 2, 0, 0, 32)
    expect(edge.fillStartIndices).toHaveLength(1)
    expect(ringAt(edge, 0)).toHaveLength(4) // c00, c10, crossing + close
    expect(edge.outlineSegments).toHaveLength(4) // crossing → c00
  })

  it('saddle: connected hexagon when the centre average clears iso', () => {
    // Opposite corners dense; centre avg (4+4)/4 = 2 ≥ iso 1 → one hexagon, two contour edges.
    const geo = forestMassFromCorners(Uint16Array.from([4, 0, 0, 4]), 2, 2, 0, 0, 32)
    expect(geo.fillStartIndices).toHaveLength(1)
    expect(ringAt(geo, 0)).toHaveLength(7) // 6 vertices + close
    expect(geo.outlineSegments).toHaveLength(8) // 2 segments
  })

  it('saddle: two triangles when the centre average misses iso (deterministic)', () => {
    // Same corners, iso 3: centre avg 2 < 3 → disconnected corner triangles.
    const geo = forestMassFromCorners(Uint16Array.from([4, 0, 0, 4]), 2, 2, 0, 0, 32, 3)
    expect(geo.fillStartIndices).toHaveLength(2)
    expect(ringAt(geo, 0)).toHaveLength(4)
    expect(ringAt(geo, 1)).toHaveLength(4)
    expect(geo.outlineSegments).toHaveLength(8)
  })

  it('is reproducible — identical input yields byte-identical output (F6)', () => {
    const corners = Uint16Array.from({ length: 17 * 17 }, (_, k) => (k * 7919) % 6)
    const a = forestMassFromCorners(corners, 17, 17, 512, 1024, 32)
    const b = forestMassFromCorners(corners, 17, 17, 512, 1024, 32)
    expect([...a.fillPositions]).toEqual([...b.fillPositions])
    expect([...a.fillStartIndices]).toEqual([...b.fillStartIndices])
    expect([...a.outlineSegments]).toEqual([...b.outlineSegments])
  })

  it('every emitted ring is a closed loop (_normalize:false contract)', () => {
    const corners = Uint16Array.from({ length: 25 }, (_, k) => (k % 3 === 0 ? 5 : 0))
    const geo = forestMassFromCorners(corners, 5, 5, 0, 0, 32)
    expect(geo.fillStartIndices.length).toBeGreaterThan(0)
    for (let n = 0; n < geo.fillStartIndices.length; n++) {
      const ring = ringAt(geo, n)
      expect(ring[0]).toEqual(ring[ring.length - 1])
      expect(ring.length).toBeGreaterThanOrEqual(4)
    }
  })

  it('exports the locked defaults', () => {
    expect(DENSITY_ISO).toBe(1)
    expect(FOREST_FILL_RGB).toEqual([34, 120, 60])
    expect(EMPTY_FOREST_GEOMETRY.fillPositions).toHaveLength(0)
  })
})

describe('forestFillAlpha (N3 ladder)', () => {
  it('steps 0.45 / 0.35 / 0.12 / 0 at the contract band edges', () => {
    expect(forestFillAlpha(-6)).toBe(0.45)
    expect(forestFillAlpha(-2.6)).toBe(0.45)
    expect(forestFillAlpha(-2.5)).toBe(0.35) // band edge belongs to the finer band
    expect(forestFillAlpha(-2)).toBe(0.35) // default zoom — F3 experience
    expect(forestFillAlpha(1)).toBe(0.35)
    expect(forestFillAlpha(1.5)).toBe(0.12) // latent behind classVisible('forestFill')
    expect(forestFillAlpha(3)).toBe(0.12)
    expect(forestFillAlpha(3.5)).toBe(0)
    expect(forestFillAlpha(6)).toBe(0)
  })
})
