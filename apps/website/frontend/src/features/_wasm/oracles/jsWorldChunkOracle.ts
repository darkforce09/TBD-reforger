// T-151.9 — Deck-free JS oracle for world*.parity (extracted from worldObjectsCore / buildingLayer / roadLayer).

/** Chunk-instance render classes (kind/class → lodGates class). Codes index this array —
 *  the wire form of `VisibleSet.classes` (Uint8Array). Road/forest classes are not instance
 *  classes and never appear here. */
export const RENDER_CLASS_CODES = ['building', 'tree', 'vegetation', 'prop', 'rockLarge'] as const
export type InstanceRenderClass = (typeof RENDER_CLASS_CODES)[number]

/** L1/L2 taxonomy → render class. `water` pier/dock draw as building footprints
 *  (T-090.5.2.2); rocks map to the landmark gate until a small/large split is exported;
 *  unknown kinds → null (indexed for pick? no — never drawn, never picked). */
export function renderClassForPrefab(kind: string, cls: string): InstanceRenderClass | null {
  switch (kind) {
    case 'building':
      return 'building'
    case 'water':
      return cls === 'pier' || cls === 'dock' ? 'building' : null
    case 'tree':
      return 'tree'
    case 'vegetation':
      return 'vegetation'
    case 'rock':
      return 'rockLarge'
    case 'prop':
    case 'utility':
      return 'prop'
    default:
      return null
  }
}


/** Oversized-object threshold (plan §6): a prefab whose footprint half-extent reaches this
 *  can straddle a chunk boundary by more than the border preload covers (bridges, long
 *  piers), so the store asks for +1 chunk ring while any such prefab exists in the export. */
export const OVERSIZED_HALF_EXTENT_M = 64

/** Glyph render fields (prefabs.json.gz `render` block, T-090.5.5) — the tree/prop IconLayer
 *  keys off these: iconKey → atlas rect, baseSizePx → glyph size, defaultColor → tint. Per-prefab
 *  importanceZoom (landmark early-surface, contract N2) is carried but inert for the current Everon
 *  census (no tree declares it; the tree class gate is already 0) — flagged for Cursor doc sync. */
export interface WorldGlyphRender {
  iconKey?: string
  baseSizePx?: number
  defaultColor?: string
  importanceZoom?: number
}

/** Prefab row subset shipped to the main thread (structured-clone-safe). Shape mirrors the
 *  export's `prefabs.json.gz` rows closely enough that chunkStore can feed it to the existing
 *  buildingLayer.buildingPrefabLookup unchanged (single source for pier/dock + default rules).
 *  `render` + `spatial.heightM` (T-090.5.5) feed the tree/prop glyph layer. */
export interface WorldPrefabRow {
  prefabId: number
  kind: string
  class: string
  label?: string
  resourceName?: string
  spatial?: { halfExtentsM?: { x?: number; y?: number; z?: number }; heightM?: number }
  render?: WorldGlyphRender
}

export interface ParsedChunk {
  id: string
  cx: number
  cy: number
  count: number
  /** SoA master arrays (never transferred — per-delivery copies are sliced from these). */
  positions: Float32Array
  prefabIdx: Uint16Array
  rotations: Float32Array
  z: Float32Array
  clsCodes: Uint8Array // RENDER_CLASS_CODES index, 255 = unclassified (never drawn/picked)
  /** Row indices per render class present in this chunk (gather lists for group slicing). */
  rowsByClass: Partial<Record<InstanceRenderClass, Uint32Array>>
  lastUsed: number
}

const NO_CLASS = 255

/** Narrow untyped prefab rows (prefabs.json.gz) to the clone-safe subset we ship + join on.
 *  Exported for the T-151.2 parity harness (build `prefabById` for `parseChunkOracle`). */
export function narrowPrefabRows(raw: unknown): WorldPrefabRow[] {
  const rows = (raw as { prefabs?: unknown } | null)?.prefabs
  if (!Array.isArray(rows)) return []
  const out: WorldPrefabRow[] = []
  for (const r of rows) {
    const p = r as {
      prefabId?: unknown
      kind?: unknown
      class?: unknown
      label?: unknown
      resourceName?: unknown
      spatial?: { halfExtentsM?: { x?: unknown; y?: unknown; z?: unknown }; heightM?: unknown }
      render?: {
        iconKey?: unknown
        baseSizePx?: unknown
        defaultColor?: unknown
        importanceZoom?: unknown
      }
    }
    if (typeof p.prefabId !== 'number' || typeof p.kind !== 'string') continue
    out.push({
      prefabId: p.prefabId,
      kind: p.kind,
      class: typeof p.class === 'string' ? p.class : 'unknown',
      label: typeof p.label === 'string' ? p.label : undefined,
      resourceName: typeof p.resourceName === 'string' ? p.resourceName : undefined,
      spatial: narrowSpatial(p.spatial),
      render: narrowRender(p.render),
    })
  }
  return out
}

/** Narrow the spatial block to the clone-safe subset render layers join on: OBB half extents
 *  (building geometry) + heightM (tree glyph 1.5× size cap, T-090.5.5). */
function narrowSpatial(
  spatial:
    { halfExtentsM?: { x?: unknown; y?: unknown; z?: unknown }; heightM?: unknown } | undefined,
): WorldPrefabRow['spatial'] {
  if (!spatial) return undefined
  const he = spatial.halfExtentsM
  const out: NonNullable<WorldPrefabRow['spatial']> = {}
  if (he) {
    out.halfExtentsM = {
      x: typeof he.x === 'number' ? he.x : undefined,
      y: typeof he.y === 'number' ? he.y : undefined,
      z: typeof he.z === 'number' ? he.z : undefined,
    }
  }
  if (typeof spatial.heightM === 'number') out.heightM = spatial.heightM
  return out
}

/** Narrow the glyph render block (T-090.5.5): iconKey/baseSizePx/defaultColor feed the tree/prop
 *  IconLayer; importanceZoom is carried for the per-prefab landmark override (contract N2). */
function narrowRender(
  render:
    | { iconKey?: unknown; baseSizePx?: unknown; defaultColor?: unknown; importanceZoom?: unknown }
    | undefined,
): WorldGlyphRender | undefined {
  if (!render) return undefined
  return {
    iconKey: typeof render.iconKey === 'string' ? render.iconKey : undefined,
    baseSizePx: typeof render.baseSizePx === 'number' ? render.baseSizePx : undefined,
    defaultColor: typeof render.defaultColor === 'string' ? render.defaultColor : undefined,
    importanceZoom: typeof render.importanceZoom === 'number' ? render.importanceZoom : undefined,
  }
}

/** prefabId → render-class code + row, and the oversized flag (plan §6 oversized ring).
 *  Exported for the T-151.2 parity harness (build `prefabById` for `parseChunkOracle`). */
export function buildPrefabMaps(prefabRows: WorldPrefabRow[]): {
  byId: Map<number, { code: number; row: WorldPrefabRow }>
  hasOversized: boolean
} {
  const byId = new Map<number, { code: number; row: WorldPrefabRow }>()
  let hasOversized = false
  for (const row of prefabRows) {
    const cls = renderClassForPrefab(row.kind, row.class)
    const code = cls ? RENDER_CLASS_CODES.indexOf(cls) : NO_CLASS
    byId.set(row.prefabId, { code, row })
    const hx = row.spatial?.halfExtentsM?.x ?? 0
    const hy = row.spatial?.halfExtentsM?.y ?? 0
    if (cls && Math.max(hx, hy) >= OVERSIZED_HALF_EXTENT_M) hasOversized = true
  }
  return { byId, hasOversized }
}

/** Narrow one chunk instance row ([prefabId, x, y, z, rotationDeg]) or reject it. */
function narrowInstanceRow(row: unknown): [number, number, number, number, number] | null {
  if (!Array.isArray(row) || row.length < 3) return null
  const [pid, x, y, zv, rot] = row as number[]
  if (typeof pid !== 'number' || !Number.isFinite(x) || !Number.isFinite(y)) return null
  return [pid, x, y, Number.isFinite(zv) ? zv : 0, Number.isFinite(rot) ? rot : 0]
}

/**
 * Test-only oracle export of the chunk parse (T-151.2, L11): the exact body the worker's
 * `parseChunk` runs, lifted to module scope so the wasm differential harness can drive it
 * without the factory. `prefabById` is a `buildPrefabMaps().byId`; `lastUsed` is 0 (the live
 * worker overwrites it with its LRU tick). Pure — no worker behavior change.
 */
export function parseChunkOracle(
  id: string,
  raw: unknown,
  prefabById: Map<number, { code: number; row: WorldPrefabRow }>,
): ParsedChunk | null {
  const instances = (raw as { instances?: unknown } | null)?.instances
  if (!Array.isArray(instances)) return null
  const [cxStr, cyStr] = id.split('_')
  const n = instances.length
  const positions = new Float32Array(2 * n)
  const prefabIdx = new Uint16Array(n)
  const rotations = new Float32Array(n)
  const z = new Float32Array(n)
  const clsCodes = new Uint8Array(n)
  const rowLists = new Map<number, number[]>()
  let count = 0
  for (const inst of instances) {
    const row = narrowInstanceRow(inst)
    if (!row) continue
    const [pid, x, y, zv, rot] = row
    const i = count++
    positions[2 * i] = x
    positions[2 * i + 1] = y
    prefabIdx[i] = pid
    rotations[i] = rot
    z[i] = zv
    const code = prefabById.get(pid)?.code ?? NO_CLASS
    clsCodes[i] = code
    if (code !== NO_CLASS) {
      let list = rowLists.get(code)
      if (!list) rowLists.set(code, (list = []))
      list.push(i)
    }
  }
  const rowsByClass: ParsedChunk['rowsByClass'] = {}
  for (const [code, rows] of rowLists) {
    rowsByClass[RENDER_CLASS_CODES[code]] = Uint32Array.from(rows)
  }
  return {
    id,
    cx: Number(cxStr),
    cy: Number(cyStr),
    count,
    positions,
    prefabIdx,
    rotations,
    z,
    clsCodes,
    rowsByClass,
    lastUsed: 0,
  }
}

/** OBB footprint corners around (x, y): half extents ±halfX/±halfY rotated by `rotationDeg`
 *  clockwise from north (L2). Returns the 4-corner ring in world meters. */
export function obbCorners(
  x: number,
  y: number,
  halfX: number,
  halfY: number,
  rotationDeg: number,
): [number, number][] {
  const rad = (rotationDeg * Math.PI) / 180
  const cos = Math.cos(rad)
  const sin = Math.sin(rad)
  const rot = (dx: number, dy: number): [number, number] => [
    x + dx * cos + dy * sin,
    y - dx * sin + dy * cos,
  ]
  return [rot(-halfX, -halfY), rot(halfX, -halfY), rot(halfX, halfY), rot(-halfX, halfY)]
}

/** Consecutive centerline vertices closer than this are the collapsed duplicate cross-edges. */
const CENTERLINE_DEDUPE_M = 0.05

/** The export's road polylines are NOT centerlines — they are road-surface quad soup:
 *  alternating cross-edge point PAIRS (edge length = true road width; runway 20 m, paved 4 m,
 *  dirt 1.75 m on Everon), with every second cross-edge duplicated. Drawing them raw produces
 *  perpendicular "centipede" ticks (T-090.5.2.1 diagnosis: 41,758 of 169,346 steps are dups).
 *  Recover the drawable geometry: midpoint of each pair = centerline vertex, median pair
 *  length = measured width. Returns null when fewer than 2 distinct midpoints survive. */
export function extractRoadCenterline(
  points: [number, number][],
): { path: [number, number][]; widthM: number } | null {
  const path: [number, number][] = []
  const widths: number[] = []
  const pairCount = Math.floor(points.length / 2) // odd trailing point is dropped
  for (let k = 0; k < pairCount; k++) {
    const a = points[2 * k]
    const b = points[2 * k + 1]
    const mx = (a[0] + b[0]) / 2
    const my = (a[1] + b[1]) / 2
    const prev = path[path.length - 1]
    if (prev && Math.hypot(mx - prev[0], my - prev[1]) < CENTERLINE_DEDUPE_M) continue
    path.push([mx, my])
    widths.push(Math.hypot(b[0] - a[0], b[1] - a[1]))
  }
  if (path.length < 2) return null
  const sorted = [...widths].sort((x, y) => x - y)
  return { path, widthM: sorted[Math.floor(sorted.length / 2)] }
}

