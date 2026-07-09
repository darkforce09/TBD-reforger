import { describe, it, expect } from 'vitest'
import {
  decodeTBDD,
  forestMassFromCorners,
  DENSITY_ISO,
} from '@/features/tactical-map/worldmap/forestMass'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { f32BytesEqual, intArrayEqual } from './parity'

/** Encode a TBDD buffer (mirror of scripts/map-assets/lib/density-grid.mjs / the worker test). */
function makeTBDD(cellM: number, cols: number, rows: number, channels: number[][]): Uint8Array {
  const plane = cols * rows
  const buf = new Uint8Array(16 + channels.length * plane * 2)
  const dv = new DataView(buf.buffer)
  buf[0] = 0x54 // 'T'
  buf[1] = 0x42 // 'B'
  buf[2] = 0x44 // 'D'
  buf[3] = 0x44 // 'D'
  dv.setUint16(4, 1, true) // version
  dv.setUint16(6, cellM, true)
  dv.setUint16(8, cols, true)
  dv.setUint16(10, rows, true)
  buf[12] = channels.length
  for (let c = 0; c < channels.length; c++) {
    const base = 16 + c * plane * 2
    for (let k = 0; k < plane; k++) dv.setUint16(base + 2 * k, channels[c][k], true)
  }
  return buf
}

describe('map-engine-wasm geometry::tbdd — Class R (bit-identical)', () => {
  it('decode_tbdd matches decodeTBDD', () => {
    const tree = [0, 1, 2, 3, 4, 5, 6, 7, 8]
    const rock = [9, 8, 7, 6, 5, 4, 3, 2, 1]
    const buf = makeTBDD(32, 3, 3, [tree, rock])
    const ts = decodeTBDD(buf)
    const w = wasm.decode_tbdd(buf)
    expect(w.cell_m).toBe(ts.cellM)
    expect(w.cols).toBe(ts.cols)
    expect(w.rows).toBe(ts.rows)
    expect(w.channel_count).toBe(ts.channels.length)
    expect(intArrayEqual(w.channel(0), ts.channels[0])).toBe(true)
    expect(intArrayEqual(w.channel(1), ts.channels[1])).toBe(true)
  })

  it('decode_tbdd throws on bad magic like the TS', () => {
    const buf = makeTBDD(32, 2, 2, [[1, 2, 3, 4]])
    buf[0] = 0x58 // 'X'
    expect(() => decodeTBDD(buf)).toThrow()
    expect(() => wasm.decode_tbdd(buf)).toThrow()
  })
})

describe('map-engine-wasm geometry::forest_mass — Class R (byte-identical)', () => {
  // Row-major corners (j*cols+i): index 0=v00, 1=v10, 2=v01, 3=v11 for a 2×2 cell.
  // Densities chosen so default DENSITY_ISO=2 still exercises non-empty cases.
  const patterns: { cols: number; rows: number; corners: number[]; note: string }[] = [
    { cols: 2, rows: 2, corners: [3, 0, 0, 3], note: 'saddle split (centre < iso 4)' },
    { cols: 2, rows: 2, corners: [5, 0, 0, 5], note: 'saddle connected (centre ≥ iso)' },
    { cols: 2, rows: 2, corners: [3, 3, 3, 3], note: 'full cell (case 15)' },
    { cols: 2, rows: 2, corners: [5, 0, 0, 0], note: 'single inside corner' },
    { cols: 2, rows: 2, corners: [1, 0, 0, 0], note: 'below iso empty' },
    { cols: 3, rows: 3, corners: [0, 0, 0, 0, 9, 0, 0, 0, 0], note: 'central peak (all boundary)' },
    {
      cols: 4,
      rows: 4,
      corners: [0, 1, 2, 3, 1, 4, 5, 2, 2, 5, 4, 1, 3, 2, 1, 0],
      note: 'ramp / mixed',
    },
  ]

  for (const p of patterns) {
    it(`forest_mass matches forestMassFromCorners — ${p.note}`, () => {
      const corners = Uint16Array.from(p.corners)
      const ts = forestMassFromCorners(corners, p.cols, p.rows, 100, 200, 32, DENSITY_ISO)
      const w = wasm.forest_mass(corners, p.cols, p.rows, 100, 200, 32, DENSITY_ISO)
      expect(f32BytesEqual(w.fill_positions, ts.fillPositions)).toBe(true)
      expect(intArrayEqual(w.fill_start_indices, ts.fillStartIndices)).toBe(true)
      expect(f32BytesEqual(w.outline_segments, ts.outlineSegments)).toBe(true)
    })
  }
})
