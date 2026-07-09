// T-151.6 — dedicated slot/cluster atlas (procedural ring + solid disc).
// Not the world-glyphs atlas. Mirrors Deck canvas markers in useIconLayer / useClusterIconLayer.

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

  // UV in top-left origin (WebGPU texture): row 0 = top.
  // glyph0: [0,0]..[0.5,1], glyph1: [0.5,0]..[1,1]
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

/** Pure pack helpers mirrored from Rust `slots_gpu` (for unit tests without GPU). */
export const SLOT_ICON_STRIDE = 20
export const SLOT_RING_PX = 20
export const SLOT_SELECTED_PX = 28
export const SLOT_PRIMARY_RGBA: [number, number, number, number] = [173, 198, 255, 255]
export const SLOT_SELECTED_RGBA: [number, number, number, number] = [250, 204, 21, 255]
export const CLUSTER_DISC_RGBA: [number, number, number, number] = [173, 198, 255, 235]

export function packRgbaU32(rgba: readonly [number, number, number, number]): number {
  return (rgba[0] | (rgba[1] << 8) | (rgba[2] << 16) | (rgba[3] << 24)) >>> 0
}

export function packIconInstance(
  out: Uint8Array,
  offset: number,
  x: number,
  y: number,
  sizePx: number,
  glyph: number,
  tint: number,
): void {
  const dv = new DataView(out.buffer, out.byteOffset + offset, SLOT_ICON_STRIDE)
  dv.setFloat32(0, x, true)
  dv.setFloat32(4, y, true)
  dv.setFloat32(8, sizePx, true)
  dv.setInt16(12, 0, true)
  dv.setUint16(14, glyph, true)
  dv.setUint32(16, tint >>> 0, true)
}

/** Pack slot rings from interleaved xy Float32Array + selected flags. */
export function packSlotInstances(xy: Float32Array, selected: boolean[]): Uint8Array {
  const n = (xy.length / 2) | 0
  const out = new Uint8Array(n * SLOT_ICON_STRIDE)
  const primary = packRgbaU32(SLOT_PRIMARY_RGBA)
  const sel = packRgbaU32(SLOT_SELECTED_RGBA)
  for (let i = 0; i < n; i++) {
    const isSel = selected[i] === true
    const x = xy[i * 2] ?? 0
    const y = xy[i * 2 + 1] ?? 0
    packIconInstance(
      out,
      i * SLOT_ICON_STRIDE,
      x,
      y,
      isSel ? SLOT_SELECTED_PX : SLOT_RING_PX,
      0,
      isSel ? sel : primary,
    )
  }
  return out
}

export function clusterDiscSizePx(count: number): number {
  const c = Math.max(count, 1)
  return 22 + Math.min(26, Math.log10(c) * 12)
}

export function packClusterInstances(
  xs: number[],
  ys: number[],
  counts: number[],
): Uint8Array {
  const n = Math.min(xs.length, ys.length, counts.length)
  const out = new Uint8Array(n * SLOT_ICON_STRIDE)
  const tint = packRgbaU32(CLUSTER_DISC_RGBA)
  for (let i = 0; i < n; i++) {
    packIconInstance(
      out,
      i * SLOT_ICON_STRIDE,
      xs[i] ?? 0,
      ys[i] ?? 0,
      clusterDiscSizePx(counts[i] ?? 1),
      1,
      tint,
    )
  }
  return out
}

export function pxToMAtZoom(deckZoom: number): number {
  if (!Number.isFinite(deckZoom)) return 1
  return 2 ** -deckZoom
}
