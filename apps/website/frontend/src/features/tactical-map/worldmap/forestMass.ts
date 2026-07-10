// ORACLE-ONLY (T-151.11.3, audit B-02): the live path calls wasm `forest_fill_alpha`;
// this file feeds parity/unit tests only.
// T-090.8.1 — Forest mass geometry: TBDD density decode + per-cell marching squares
// (A3 DrawForestsNew model, uiMap.cpp:2390 analogue — T-144.1). Pure + worker-safe: no
// deck.gl/React/DOM; worldObjectsCore runs this off the main thread and ships typed arrays
// (plan §6 W-transfer rule). Layer styling lives in forestMassLayer.ts; the only style
// decision here is the N3 fill-α ladder, kept pure for vitest spot checks.
//
// TBDD wire format (locked @ T-090.3.2 — scripts/map-assets/lib/density-grid.mjs is the
// encoder/reference): little-endian, 16 B header (u32 magic 'TBDD', u16 version=1,
// u16 cellM=32, u16 cols=17, u16 rows=17, u8 channelCount=2, 3 B pad), then per channel
// (0=tree, 1=rock) u16[cols·rows] corner counts, row-major (j·cols + i). Corner (i,j) of
// chunk (cx,cy) sits at world (cx·512 + i·cellM, cy·512 + j·cellM). Rock channel is
// decode-only this slice — rock mass styling is deferred to the P4 export phase.

/** Decoded TBDD density grid (one export chunk). */
export interface TbddGrid {
  version: number
  cellM: number
  cols: number
  rows: number
  /** Per-channel corner counts, DENSITY_CHANNEL_NAMES order (0=tree, 1=rock). */
  channels: Uint16Array[]
}

export const TBDD_HEADER_BYTES = 16
export const DENSITY_CHANNEL_NAMES = ['tree', 'rock'] as const

/** Marching-squares iso threshold in trees per 32 m corner cell.
 *  **Deck Class R mirror only** of Rust `forest_mass::DENSITY_ISO` (source of truth, T-151.5.1).
 *  Path B region export floor is 2 — corners with count ≥ iso are inside; count-1 lone trees
 *  are not forest mass. wgpu must use wasm `density_iso()`, not this constant. */
export const DENSITY_ISO = 2

/** Forest mass fill color (locked — t090_8 §Render / N3): rgba(34,120,60,α). */
export const FOREST_FILL_RGB: [number, number, number] = [34, 120, 60]

/** Decode one TBDD buffer. Throws on bad magic/version/size — the caller (worker core)
 *  maps a throw to "no density for this chunk" and caches the miss. */
export function decodeTBDD(bytes: Uint8Array): TbddGrid {
  if (bytes.length < TBDD_HEADER_BYTES) throw new Error(`TBDD: short buffer (${bytes.length} B)`)
  const magic = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3])
  if (magic !== 'TBDD') throw new Error(`TBDD: bad magic '${magic}'`)
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const version = dv.getUint16(4, true)
  const cellM = dv.getUint16(6, true)
  const cols = dv.getUint16(8, true)
  const rows = dv.getUint16(10, true)
  const channelCount = bytes[12]
  const need = TBDD_HEADER_BYTES + channelCount * cols * rows * 2
  if (bytes.length < need) throw new Error(`TBDD: truncated (${bytes.length} B, want ${need})`)
  const channels: Uint16Array[] = []
  for (let c = 0; c < channelCount; c++) {
    const base = TBDD_HEADER_BYTES + c * cols * rows * 2
    const ch = new Uint16Array(cols * rows)
    for (let k = 0; k < ch.length; k++) ch[k] = dv.getUint16(base + 2 * k, true)
    channels.push(ch)
  }
  return { version, cellM, cols, rows, channels }
}

/** Marching-squares output for one chunk, shaped for Deck binary data (transferable).
 *  fillPositions = closed rings ([x,y]·vertex, first vertex repeated last — SolidPolygonLayer
 *  `_normalize:false` contract); fillStartIndices = per-ring start VERTEX index, no trailing
 *  sentinel; outlineSegments = iso-contour segment pairs [x0,y0,x1,y1]·segment. */
export interface ForestMassGeometry {
  fillPositions: Float32Array
  fillStartIndices: Uint32Array
  outlineSegments: Float32Array
}

export const EMPTY_FOREST_GEOMETRY: ForestMassGeometry = {
  fillPositions: new Float32Array(0),
  fillStartIndices: new Uint32Array(0),
  outlineSegments: new Float32Array(0),
}

/** One walk point: a cell corner (inside) or an interpolated iso crossing on a cell edge. */
interface WalkPoint {
  x: number
  y: number
  crossing: boolean
}

/** One cell corner in perimeter-walk order, with its count and world position. */
interface WalkCorner {
  v: number
  inside: boolean
  x: number
  y: number
}

/** Dedupe consecutive identical points and unclose the ring (iso exactly on a corner
 *  collapses its crossing onto the corner) — a merged point keeps the crossing flag so the
 *  contour pairing still sees corner-hugging iso edges. */
function dedupeRing(pts: WalkPoint[]): WalkPoint[] {
  const ring: WalkPoint[] = []
  for (const p of pts) {
    const last = ring[ring.length - 1]
    if (last && last.x === p.x && last.y === p.y) {
      last.crossing = last.crossing || p.crossing
      continue
    }
    ring.push({ ...p })
  }
  while (ring.length > 1) {
    const first = ring[0]
    const last = ring[ring.length - 1]
    if (first.x !== last.x || first.y !== last.y) break
    first.crossing = first.crossing || last.crossing
    ring.pop()
  }
  return ring
}

/**
 * Per-cell marching squares over a corner-count grid (16 cases via a boundary walk):
 * walk the cell perimeter corner→corner; emit inside corners as ring vertices and an
 * interpolated crossing wherever the inside state flips. That yields the correct fill
 * polygon for every case except the two saddles (opposite corners inside), which the
 * center average disambiguates — center ≥ iso keeps the connected hexagon the walk
 * produces, center < iso splits into two corner triangles (deterministic, F6).
 * Contour segments fall out of the same walk: every consecutive crossing→crossing pair
 * is an iso edge. Cell-local polygons only — no global ring assembly, no hole topology.
 */
export function forestMassFromCorners(
  corners: ArrayLike<number>,
  cols: number,
  rows: number,
  originX: number,
  originY: number,
  cellM: number,
  iso: number = DENSITY_ISO,
): ForestMassGeometry {
  const positions: number[] = []
  const startIndices: number[] = []
  const segments: number[] = []
  let vertexCount = 0

  const emitRing = (pts: WalkPoint[]): void => {
    // Degenerate rings (a lone count≥iso corner with no area) drop entirely.
    const ring = dedupeRing(pts)
    if (ring.length < 3) return
    startIndices.push(vertexCount)
    for (const p of ring) positions.push(p.x, p.y)
    positions.push(ring[0].x, ring[0].y) // close the loop (_normalize:false contract)
    vertexCount += ring.length + 1
    for (let k = 0; k < ring.length; k++) {
      const a = ring[k]
      const b = ring[(k + 1) % ring.length]
      if (a.crossing && b.crossing && (a.x !== b.x || a.y !== b.y)) {
        segments.push(a.x, a.y, b.x, b.y)
      }
    }
  }

  const crossingOn = (a: WalkCorner, b: WalkCorner): WalkPoint => {
    const t = (iso - a.v) / (b.v - a.v) // states differ ⇒ denominator ≠ 0
    return { x: a.x + t * (b.x - a.x), y: a.y + t * (b.y - a.y), crossing: true }
  }

  /** Disconnected saddle: two corner triangles around the two inside corners. */
  const emitSaddleTriangles = (walkCorners: WalkCorner[]): void => {
    for (let k = 0; k < 4; k++) {
      const c = walkCorners[k]
      if (!c.inside) continue
      const prev = walkCorners[(k + 3) % 4]
      const next = walkCorners[(k + 1) % 4]
      emitRing([{ x: c.x, y: c.y, crossing: false }, crossingOn(c, next), crossingOn(c, prev)])
    }
  }

  const marchCell = (i: number, j: number, v00: number, v10: number, v11: number, v01: number): void => {
    const in00 = v00 >= iso
    const in10 = v10 >= iso
    const in11 = v11 >= iso
    const in01 = v01 >= iso
    const x0 = originX + i * cellM
    const y0 = originY + j * cellM
    const x1 = x0 + cellM
    const y1 = y0 + cellM

    // Perimeter in walk order c00 → c10 → c11 → c01 with each corner's world position.
    const walkCorners: WalkCorner[] = [
      { v: v00, inside: in00, x: x0, y: y0 },
      { v: v10, inside: in10, x: x1, y: y0 },
      { v: v11, inside: in11, x: x1, y: y1 },
      { v: v01, inside: in01, x: x0, y: y1 },
    ]

    const saddle = in00 === in11 && in10 === in01 && in00 !== in10
    if (saddle && (v00 + v10 + v11 + v01) / 4 < iso) {
      emitSaddleTriangles(walkCorners)
      return
    }
    const walk: WalkPoint[] = []
    for (let k = 0; k < 4; k++) {
      const a = walkCorners[k]
      const b = walkCorners[(k + 1) % 4]
      if (a.inside) walk.push({ x: a.x, y: a.y, crossing: false })
      if (a.inside !== b.inside) walk.push(crossingOn(a, b))
    }
    emitRing(walk) // case 15 falls through here too: four corners, no crossings, no contour
  }

  for (let j = 0; j < rows - 1; j++) {
    for (let i = 0; i < cols - 1; i++) {
      const v00 = corners[j * cols + i]
      const v10 = corners[j * cols + i + 1]
      const v11 = corners[(j + 1) * cols + i + 1]
      const v01 = corners[(j + 1) * cols + i]
      // Case 0 — the common fast path (most of a 16×16 chunk is not forest boundary).
      if (v00 < iso && v10 < iso && v11 < iso && v01 < iso) continue
      marchCell(i, j, v00, v10, v11, v01)
    }
  }

  return {
    fillPositions: Float32Array.from(positions),
    fillStartIndices: Uint32Array.from(startIndices),
    outlineSegments: Float32Array.from(segments),
  }
}

/** N3 forest fill-α ladder (contract master band table; band edges belong to the finer
 *  band). NOTE: the render gate is still lodGates.classVisible('forestFill') (≤ +1) — the
 *  0.12 band matches the N3 table but stays latent behind the shipped gate; it activates
 *  if the contract ever loosens FOREST_FILL_MAX_ZOOM (single-authority rule, LOD5). */
export function forestFillAlpha(deckZoom: number): number {
  if (deckZoom < -2.5) return 0.45
  if (deckZoom <= 1) return 0.35
  if (deckZoom <= 3) return 0.12
  return 0
}
