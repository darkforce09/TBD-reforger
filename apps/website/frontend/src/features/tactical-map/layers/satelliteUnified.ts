// T-090.1.2.8 — Unified satellite texture loader (tbd-sat v1).
//
// The bundle is a GLB-style container: "TBDS" magic + u32 version + u32 jsonLength + JSON
// index + concatenated lossless-WebP (VP8L) blocks, one block grid per mip level (built by
// scripts/map-assets/build-unified-satellite.mjs). We fetch it ONCE, decode each block with
// createImageBitmap (native VP8L decode, worker pool), and upload the whole mip chain into
// a single luma.gl texture with trilinear sampling — the Deck BitmapLayer then samples GPU
// mips on zoom instead of mounting/unmounting per-viewport tile layers (the pyramid-mode
// pop-in this slice removes).
//
// Device adaptivity: when the GPU's maxTextureDimension2D is below the 12800² base, the
// loader starts the texture at the first mip level that fits (e.g. 6400² on an 8192-limit
// device) and NEVER decodes the skipped levels — the bytes are fetched (one bundle) but the
// ~655 MB base RGBA decode is avoided exactly on the machines that can't use it.
//
// Orientation contract: block row 0 = top = north (same as the pyramid/full.webp path);
// no flips anywhere — BitmapLayer maps the image top scanline to bounds maxY.

import type { Device, Texture } from '@luma.gl/core'

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

  // The chain must follow the GL mip rule from base down to 1×1 so mipLevels lines up with
  // luma.gl's getMipLevelCount for any base level we pick.
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

export interface UnifiedSatResult {
  texture: Texture
  index: TbdSatIndex
  baseLevel: number
}

/**
 * Fetch the bundle (streaming, with fraction progress), decode every mip level ≥ the
 * device-fitting base, and upload them into one immutable rgba8unorm texture with trilinear
 * sampling. Resolves only when EVERY allocated mip is populated — texStorage2D levels start
 * zeroed, so returning early would let far zooms sample black. Fractions: fetch 0→0.8,
 * decode+upload 0.8→1 (weighted by block bytes); a final 1 is emitted right before resolve.
 */
export async function loadUnifiedSatTexture(
  device: Device,
  url: string,
  opts: { onProgress?: (fraction: number) => void; signal?: AbortSignal } = {},
): Promise<UnifiedSatResult> {
  const { onProgress, signal } = opts
  let lastPct = -1
  const emit = (f: number) => {
    const pct = Math.floor(f * 50) // throttle to 2% steps
    if (pct === lastPct) return
    lastPct = pct
    onProgress?.(Math.min(f, 1))
  }

  const resp = await fetch(url, { signal })
  if (!resp.ok) bail(`fetch ${resp.status} for ${url}`)
  let buf: ArrayBuffer
  const total = Number(resp.headers.get('content-length') ?? 0)
  if (resp.body && total > 0) {
    const reader = resp.body.getReader()
    const chunks: Uint8Array[] = []
    let received = 0
    for (;;) {
      const { done, value } = await reader.read()
      if (done) break
      chunks.push(value)
      received += value.byteLength
      emit((received / total) * 0.8)
    }
    const out = new Uint8Array(received)
    let pos = 0
    for (const c of chunks) {
      out.set(c, pos)
      pos += c.byteLength
    }
    buf = out.buffer
  } else {
    buf = await resp.arrayBuffer()
    emit(0.8)
  }

  const index = parseTbdSat(buf)
  const baseLevel = pickBaseLevel(index, device.limits.maxTextureDimension2D)
  const base = index.mips[baseLevel]
  const texture = device.createTexture({
    id: `unified-sat-${index.terrainId}`,
    format: 'rgba8unorm',
    width: base.width,
    height: base.height,
    mipLevels: index.mipCount - baseLevel,
    sampler: {
      minFilter: 'linear',
      magFilter: 'linear',
      mipmapFilter: 'linear',
      addressModeU: 'clamp-to-edge',
      addressModeV: 'clamp-to-edge',
    },
  })

  try {
    const uploadBytes = index.mips
      .slice(baseLevel)
      .reduce((s, m) => s + m.tiles.reduce((a, t) => a + t.length, 0), 0)
    let doneBytes = 0
    for (let level = baseLevel; level < index.mipCount; level++) {
      const mip = index.mips[level]
      // Decode the level's blocks in parallel (browser image decode runs on a worker pool —
      // the 2×2 base grid is what makes the dominant decode ~parallel), then upload + close
      // each bitmap so peak RAM stays ~one level, not the whole chain.
      const bitmaps = await Promise.all(
        mip.tiles.map((t) =>
          createImageBitmap(new Blob([new Uint8Array(buf, t.offset, t.length)], { type: 'image/webp' })),
        ),
      )
      if (signal?.aborted) {
        bitmaps.forEach((b) => b.close())
        throw new DOMException('aborted', 'AbortError')
      }
      mip.tiles.forEach((t, i) => {
        const image = bitmaps[i]
        if (image.width !== t.width || image.height !== t.height) {
          bail(`level ${level} block decoded ${image.width}x${image.height}, index says ${t.width}x${t.height}`)
        }
        texture.copyExternalImage({
          image,
          x: t.x,
          y: t.y,
          width: t.width,
          height: t.height,
          mipLevel: level - baseLevel,
        })
        image.close()
        doneBytes += t.length
        emit(0.8 + (doneBytes / uploadBytes) * 0.2)
      })
    }
  } catch (e) {
    texture.destroy()
    throw e
  }
  lastPct = -1
  emit(1)
  return { texture, index, baseLevel }
}
