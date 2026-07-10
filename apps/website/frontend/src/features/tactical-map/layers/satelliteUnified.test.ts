// T-090.1.2.8 — tbd-sat v1 container parse + base-level pick contracts. The parser is the
// frontend's only defense before indexing into a ~200 MB ArrayBuffer, so every structural
// violation must throw (the basemap hook catches → pyramid fallback). Decode/GPU upload is
// browser-only and covered by the manual U1–U4 pass.
import { describe, it, expect } from 'vitest'
import {
  parseTbdSat,
  pickBaseLevel,
  parseTbdSatIndexOnly,
  pickPreviewLevel,
  type TbdSatIndex,
} from './satelliteUnified'

/** Serialize an index into a container, appending `payload` bytes after the JSON. */
function makeBundle(index: unknown, payloadLength: number, magic = 'TBDS', version = 1): ArrayBuffer {
  const json = new TextEncoder().encode(JSON.stringify(index))
  const buf = new Uint8Array(12 + json.length + payloadLength)
  buf.set(new TextEncoder().encode(magic), 0)
  new DataView(buf.buffer).setUint32(4, version, true)
  new DataView(buf.buffer).setUint32(8, json.length, true)
  buf.set(json, 12)
  return buf.buffer
}

/** Lay block offsets out contiguously after the JSON (iterated, same trick as the builder —
 *  offsets change the JSON length). Call again after mutating a fixture. */
function layoutOffsets(index: TbdSatIndex): TbdSatIndex {
  for (let jsonLen = 0; ; ) {
    let offset = 12 + jsonLen
    for (const mip of index.mips) {
      for (const t of mip.tiles) {
        t.offset = offset
        offset += t.length
      }
    }
    const encoded = new TextEncoder().encode(JSON.stringify(index)).length
    if (encoded === jsonLen) break
    jsonLen = encoded
  }
  return index
}

/** Minimal valid 2-level index (2×2 base → 1×1) with offsets laid out after the JSON. */
function validIndex(): TbdSatIndex {
  return layoutOffsets({
    formatVersion: 1,
    terrainId: 'everon',
    worldBounds: [0, 0, 12800, 12800],
    baseWidthPx: 2,
    baseHeightPx: 2,
    mipCount: 2,
    mips: [
      { level: 0, width: 2, height: 2, tiles: [{ x: 0, y: 0, width: 2, height: 2, offset: 0, length: 8 }] },
      { level: 1, width: 1, height: 1, tiles: [{ x: 0, y: 0, width: 1, height: 1, offset: 0, length: 8 }] },
    ],
  })
}

const payloadBytes = (index: TbdSatIndex) =>
  index.mips.reduce((s, m) => s + m.tiles.reduce((a, t) => a + t.length, 0), 0)

describe('parseTbdSat', () => {
  it('parses a valid container round-trip', () => {
    const index = validIndex()
    const parsed = parseTbdSat(makeBundle(index, payloadBytes(index)))
    expect(parsed.terrainId).toBe('everon')
    expect(parsed.mipCount).toBe(2)
    expect(parsed.mips[0].tiles[0].offset).toBeGreaterThanOrEqual(12)
  })

  it('rejects a bad magic (e.g. a git-lfs pointer or html error page)', () => {
    const index = validIndex()
    expect(() => parseTbdSat(makeBundle(index, payloadBytes(index), 'vers'))).toThrow(/bad magic/)
  })

  it('rejects an unsupported container version', () => {
    const index = validIndex()
    expect(() => parseTbdSat(makeBundle(index, payloadBytes(index), 'TBDS', 2))).toThrow(
      /formatVersion 2/,
    )
  })

  it('rejects a truncated file (jsonLength overruns)', () => {
    const index = validIndex()
    const whole = makeBundle(index, payloadBytes(index))
    expect(() => parseTbdSat(whole.slice(0, 20))).toThrow(/overruns/)
    expect(() => parseTbdSat(whole.slice(0, 8))).toThrow(/too small/)
  })

  it('rejects a mip chain that does not follow the GL halving rule', () => {
    const index = validIndex()
    index.baseWidthPx = 4 // chain 2→1 no longer matches a base of 4
    expect(() => parseTbdSat(makeBundle(index, payloadBytes(index)))).toThrow(/GL rule/)
  })

  it('rejects a block whose offset/length runs past the file', () => {
    const index = validIndex()
    // Drop the payload bytes the offsets point into.
    expect(() => parseTbdSat(makeBundle(index, 0))).toThrow(/out of range/)
  })

  it('rejects tile grids that do not cover the level', () => {
    const index = validIndex()
    index.mips[0].tiles[0].width = 1 // covers 1×2 of a 2×2 level
    index.mips[0].tiles[0].length = 4
    expect(() => parseTbdSat(makeBundle(index, payloadBytes(index)))).toThrow(/cover/)
  })

  it('rejects a chain that does not end at 1×1', () => {
    const index = validIndex()
    index.mipCount = 1
    index.mips = [index.mips[0]]
    layoutOffsets(index)
    expect(() => parseTbdSat(makeBundle(index, payloadBytes(index)))).toThrow(/end at 1x1/)
  })
})

describe('pickBaseLevel', () => {
  // Everon-shaped chain: 12800 → 1 (14 levels), tiles irrelevant for the pick.
  const chain = (() => {
    const mips = []
    for (let w = 12800, l = 0; ; w = Math.max(1, Math.floor(w / 2)), l++) {
      mips.push({ level: l, width: w, height: w, tiles: [] })
      if (w === 1) break
    }
    return {
      formatVersion: 1,
      terrainId: 'everon',
      worldBounds: [0, 0, 12800, 12800],
      baseWidthPx: 12800,
      baseHeightPx: 12800,
      mipCount: mips.length,
      mips,
    } as TbdSatIndex
  })()

  it('desktop 16384 limit → full 12800 base', () => {
    expect(pickBaseLevel(chain, 16384)).toBe(0)
  })

  it('8192 limit → 6400 base (level 1), skipping the 655 MB decode', () => {
    expect(pickBaseLevel(chain, 8192)).toBe(1)
  })

  it('256 limit → first level ≤256 (200px, level 6)', () => {
    expect(pickBaseLevel(chain, 256)).toBe(6)
  })

  it('degenerate limit → coarsest level still returned', () => {
    expect(pickBaseLevel(chain, 1)).toBe(chain.mipCount - 1)
  })
})

// T-151.11.4 (audit P-03) — Range-preview pure helpers.
describe('parseTbdSatIndexOnly (Range head)', () => {
  it('parses an index from a head buffer, validating block ranges against the FULL file size', () => {
    const index = validIndex()
    const whole = makeBundle(index, payloadBytes(index))
    // Head = header + JSON only (no payload) — exactly what the Range fetch returns.
    const headLen = 12 + new TextEncoder().encode(JSON.stringify(index)).length
    const head = whole.slice(0, headLen)
    const parsed = parseTbdSatIndexOnly(head, whole.byteLength)
    expect(parsed.mipCount).toBe(index.mipCount)
    expect(parsed.mips[0].tiles[0].offset).toBe(index.mips[0].tiles[0].offset)
  })

  it('throws when a block range exceeds the reported file size', () => {
    const index = validIndex()
    const whole = makeBundle(index, payloadBytes(index))
    const headLen = 12 + new TextEncoder().encode(JSON.stringify(index)).length
    const head = whole.slice(0, headLen)
    expect(() => parseTbdSatIndexOnly(head, whole.byteLength - 1)).toThrow(/out of file range/)
  })
})

describe('pickPreviewLevel', () => {
  it('returns the first (largest) level whose long edge fits the cap — Everon-shaped chain', () => {
    // 12800-shaped synthetic chain metadata (tiles irrelevant for the pick).
    const mips = []
    let w = 12800
    for (let level = 0; w >= 1; level++) {
      mips.push({ level, width: w, height: w, tiles: [] })
      if (w === 1) break
      w = Math.max(1, Math.floor(w / 2))
    }
    const index = { mips, mipCount: mips.length } as unknown as TbdSatIndex
    expect(pickPreviewLevel(index, 1024).width).toBe(800) // level 4: 12800/2^4
    expect(pickPreviewLevel(index, 800).width).toBe(800) // inclusive fit
    expect(pickPreviewLevel(index, 799).width).toBe(400)
  })

  it('falls back to the 1×1 tail when nothing fits', () => {
    const index = {
      mips: [{ level: 0, width: 4, height: 4, tiles: [] }, { level: 1, width: 2, height: 2, tiles: [] }, { level: 2, width: 1, height: 1, tiles: [] }],
      mipCount: 3,
    } as unknown as TbdSatIndex
    expect(pickPreviewLevel(index, 0).width).toBe(1)
  })
})
