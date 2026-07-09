// T-090.1.2.8 / T-151.9 — Unified satellite TBDS parse + device-fit mip pick.
//
// The bundle is a GLB-style container: "TBDS" magic + u32 version + u32 jsonLength + JSON
// index + concatenated lossless-WebP (VP8L) blocks. wgpuBasemap uses parseTbdSat +
// pickBaseLevel; GPU upload lives in the Rust/wgpu engine (Deck loadUnifiedSatTexture removed).

export interface TbdSatTile {
  /** Pixel offset of this block in level image space (row 0 = top = north). */
  x: number
  y: number
  width: number
  height: number
  /** Absolute byte offset/length of the VP8L block in the bundle. */
  offset: number
  length: number
}

export interface TbdSatMip {
  level: number
  width: number
  height: number
  tiles: TbdSatTile[]
}

export interface TbdSatIndex {
  formatVersion: number
  terrainId: string
  worldBounds: [number, number, number, number]
  baseWidthPx: number
  baseHeightPx: number
  mipCount: number
  mips: TbdSatMip[]
}

const MAGIC = 0x53444254 // "TBDS" little-endian u32

function bail(msg: string): never {
  throw new Error(`tbd-sat: ${msg}`)
}

/** Container header: magic + version + JSON index bounds + parse. */
function parseHeader(buf: ArrayBuffer): { index: TbdSatIndex; payloadStart: number } {
  if (buf.byteLength < 12) bail('file too small for header')
  const dv = new DataView(buf)
  if (dv.getUint32(0, true) !== MAGIC) bail('bad magic (expected "TBDS")')
  const version = dv.getUint32(4, true)
  if (version !== 1) bail(`unsupported formatVersion ${version}`)
  const jsonLen = dv.getUint32(8, true)
  if (12 + jsonLen > buf.byteLength) bail('JSON index overruns file')
  try {
    const index = JSON.parse(
      new TextDecoder().decode(new Uint8Array(buf, 12, jsonLen)),
    ) as TbdSatIndex
    return { index, payloadStart: 12 + jsonLen }
  } catch {
    bail('JSON index unparseable')
  }
}

/** One mip's blocks: byte ranges inside the file, tiles inside the level, full coverage. */
function validateMipTiles(mip: TbdSatMip, payloadStart: number, byteLength: number): void {
  let covered = 0
  for (const t of mip.tiles) {
    if (t.offset < payloadStart || t.offset + t.length > byteLength)
      bail(`level ${mip.level} block out of range`)
    if (t.x < 0 || t.y < 0 || t.x + t.width > mip.width || t.y + t.height > mip.height)
      bail(`level ${mip.level} tile exceeds level bounds`)
    covered += t.width * t.height
  }
  if (covered !== mip.width * mip.height) bail(`level ${mip.level} tiles do not cover the level`)
}

/**
 * Parse + structurally validate a tbd-sat v1 container. Defensive: any malformed field
 * throws (the basemap hook catches and falls back to the tile pyramid). Deep pixel checks
 * (VP8L headers, byte-exact dims) live in scripts/map-assets/verify-unified-satellite.mjs —
 * here we only guarantee we won't index out of bounds or build an impossible mip chain.
 */
export function parseTbdSat(buf: ArrayBuffer): TbdSatIndex {
  const { index, payloadStart } = parseHeader(buf)
  if (index.formatVersion !== 1) bail(`index formatVersion ${index.formatVersion} !== 1`)
  if (!Number.isInteger(index.baseWidthPx) || index.baseWidthPx < 1) bail('bad baseWidthPx')
  if (!Number.isInteger(index.baseHeightPx) || index.baseHeightPx < 1) bail('bad baseHeightPx')
  if (!Array.isArray(index.mips) || index.mips.length !== index.mipCount || index.mipCount < 1)
    bail('mips[] does not match mipCount')

  // The chain must follow the GL mip rule from base down to 1×1.
  let w = index.baseWidthPx
  let h = index.baseHeightPx
  for (const [i, mip] of index.mips.entries()) {
    if (mip.level !== i) bail(`mips[${i}].level = ${mip.level}`)
    if (mip.width !== w || mip.height !== h)
      bail(`level ${i} is ${mip.width}x${mip.height}, GL rule expects ${w}x${h}`)
    validateMipTiles(mip, payloadStart, buf.byteLength)
    w = Math.max(1, Math.floor(w / 2))
    h = Math.max(1, Math.floor(h / 2))
  }
  const last = index.mips[index.mips.length - 1]
  if (last.width !== 1 || last.height !== 1) bail('mip chain must end at 1x1')
  return index
}

/**
 * First mip level whose dimensions fit the device texture limit — level 0 on any GPU with
 * maxTextureDimension2D ≥ 12800 (desktop default 16384), level 1 (6400²) on 8192-limit
 * devices, and so on. The chain ends at 1×1 so a fitting level always exists.
 */
export function pickBaseLevel(index: TbdSatIndex, maxTextureDimension2D: number): number {
  for (const mip of index.mips) {
    if (Math.max(mip.width, mip.height) <= maxTextureDimension2D) return mip.level
  }
  return index.mipCount - 1
}
