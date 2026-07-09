// T-151.6 / T-151.7.3 — procedural slot/cluster atlas pixels only (pack policy is Rust).

const CELL = 64
const ATLAS_W = CELL * 2
const ATLAS_H = CELL

export interface SlotAtlasData {
  rgba: Uint8Array
  width: number
  height: number
  /** Flat UV table: per-glyph minU,minV,maxU,maxV (2 glyphs; engine pads to 28). */
  uv: Float32Array
}

let cached: SlotAtlasData | null = null

/** One-time procedural atlas: glyph 0 = ring, glyph 1 = solid disc (white-on-alpha for tint). */
export function buildSlotAtlas(): SlotAtlasData {
  if (cached) return cached
  const canvas = document.createElement('canvas')
  canvas.width = ATLAS_W
  canvas.height = ATLAS_H
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('2d canvas context unavailable for slot atlas')

  // Glyph 0 — ring (useIconLayer)
  {
    const c = CELL / 2
    ctx.save()
    ctx.fillStyle = '#ffffff'
    ctx.beginPath()
    ctx.arc(c, c, c - 8, 0, Math.PI * 2)
    ctx.fill()
    ctx.globalCompositeOperation = 'destination-out'
    ctx.beginPath()
    ctx.arc(c, c, c - 22, 0, Math.PI * 2)
    ctx.fill()
    ctx.restore()
  }

  // Glyph 1 — solid disc (useClusterIconLayer)
  {
    const ox = CELL
    const c = CELL / 2
    ctx.fillStyle = '#ffffff'
    ctx.beginPath()
    ctx.arc(ox + c, c, c - 6, 0, Math.PI * 2)
    ctx.fill()
  }

  const img = ctx.getImageData(0, 0, ATLAS_W, ATLAS_H)
  const rgba = new Uint8Array(img.data.buffer.slice(0))

  const uv = new Float32Array(2 * 4)
  uv[0] = 0
  uv[1] = 0
  uv[2] = 0.5
  uv[3] = 1
  uv[4] = 0.5
  uv[5] = 0
  uv[6] = 1
  uv[7] = 1

  cached = { rgba, width: ATLAS_W, height: ATLAS_H, uv }
  return cached
}
