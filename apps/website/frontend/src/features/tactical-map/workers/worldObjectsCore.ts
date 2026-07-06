// T-090.5.3 — World-objects worker CORE: the whole chunk-streaming brain as a pure factory so
// vitest (node env) drives it without a Worker/Comlink runtime (W1/W2/W4 run off the DOM
// thread by construction). worldObjects.worker.ts is a thin Comlink shell over this.
//
// Owns: manifest + prefab + chunk-index fetch, chunk fetch + gunzip (DecompressionStream) +
// parse to SoA typed arrays, the world rbush (state/worldSpatialIndex — factory instance, W3),
// worker-side chunk LRU, and the query API (visibleInstances / pickNearest / pickRect /
// resolve). Every hop back to the main thread is typed arrays — never per-instance JS objects
// (plan §6 W-transfer rule); the worker shell marks the buffers as transferables.
//
// Worker-safety: imports are pure modules only (chunkMath, lodGates, coords/terrains,
// state/worldSpatialIndex). Deliberately NOT imported: roadLayer/buildingLayer — they import
// deck.gl at module scope and would drag it into the worker bundle. Roads therefore stay a
// main-thread one-shot in worldData.ts (sanctioned: 888 segments, small), and building OBB
// geometry is derived main-side in chunkStore from the prefab rows this core returns.

import { TERRAINS, type TerrainDef } from '../coords/terrains'
import {
  DEFAULT_CHUNK_SIZE_M,
  chunkId as chunkIdOf,
  chunkIdsForRect,
  chunkRectForBbox,
  expandChunkRect,
  type Bbox,
} from '../worldmap/chunkMath'
import { INSTANCE_BUDGET, classVisible, type WorldRenderClass } from '../worldmap/lodGates'
import { DENSITY_ISO } from '../worldmap/forestMass'
import { type DemVectorGrid, reduceGrid2x } from '../worldmap/demGrid'
import { type SeaBandGeometry } from '../worldmap/seaBand'
import { contourGridReductions, contourLevels } from '../worldmap/contours'
import { createWorldSpatialIndex } from '../state/worldSpatialIndex'
// T-145 Phase 1: DEM + geometry math runs in the Rust/wasm core (byte-identical to the TS
// oracles in worldmap/{seaBand,contours,forestMass}.ts, which the differential harness pins).
// The TS grid + reduction pyramid stay here; only the leaf compute crosses into wasm.
import * as wasm from '@/wasm/pkg/map_engine_wasm'

export type { Bbox, DemVectorGrid, SeaBandGeometry }

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

/** One existing chunk cell (from the export's chunk index, or the full-grid fallback). */
export interface WorldChunkCell {
  id: string
  cx: number
  cy: number
  path: string
  instanceCount?: number
}

/** loadManifest result — everything the main thread needs to plan streaming. */
export interface WorldManifestLite {
  terrainId: string
  chunkSizeM: number
  /** Existing chunk cells (null ⇒ export shipped no index; ids are swept from the grid). */
  cells: WorldChunkCell[] | null
  prefabRows: WorldPrefabRow[]
  /** Relative paths (informational; the worker resolves URLs itself). */
  roadsPath: string | null
  /** Density-grid dir (T-090.8.1 forest mass); null ⇒ export shipped no TBDD grids. */
  densityPath: string | null
  instanceCount: number | null
  hasOversized: boolean
}

/** Per-class slice of one chunk, SoA typed arrays (transferables). */
export interface ChunkClassGroup {
  count: number
  /** [x0, y0, x1, y1, …] world meters. */
  positions: Float32Array
  prefabIdx: Uint16Array
  rotations: Float32Array
  z: Float32Array
}

export interface ChunkPayload {
  id: string
  cx: number
  cy: number
  totalInstances: number
  groups: Partial<Record<InstanceRenderClass, ChunkClassGroup>>
}

export interface ChunkLoadResult {
  chunkSizeM: number
  chunks: ChunkPayload[]
}

export interface LoadChunksOpts {
  deckZoom: number
  /** Render classes the caller can draw this slice — intersected with lodGates gates. */
  classes: InstanceRenderClass[]
  /** Exact chunk ids to hydrate (main thread already ran chunkMath); falls back to bbox. */
  ids?: string[]
  /** Chunks the caller already holds — parsed/pinned worker-side but not re-delivered. */
  excludeIds?: string[]
}

/** One chunk's forest mass geometry (T-090.8.1) — marching-squares output in world meters,
 *  shaped for Deck binary data (forestMass.ForestMassGeometry + chunk identity). Arrays are
 *  freshly allocated per delivery (the worker-side cache keeps only the decoded corner
 *  grids), so the worker shell can transfer the buffers without detaching any cache. */
export interface ForestMassChunk {
  id: string
  cx: number
  cy: number
  fillPositions: Float32Array
  fillStartIndices: Uint32Array
  outlineSegments: Float32Array
  /** Max tree corner count in the chunk (stats/debug; 0 never appears — empties are ids). */
  treeMax: number
}

export interface ForestMassResult {
  chunks: ForestMassChunk[]
  /** Requested ids with nothing to draw (missing/undecodable file or all-below-iso grid) —
   *  the main store caches these as hydrated-empty and never re-requests them. */
  emptyIds: string[]
}

/** Contour geometry for one interval (T-090.5.4). segments = interleaved [x0,y0,x1,y1] per
 *  iso segment; intervalM echoes the request so the store matches the async reply. */
export interface ContourResult {
  intervalM: number
  segments: Float32Array
}

/** visibleInstances result — flat arrays across chunks, budget-capped (transferables). */
export interface VisibleSet {
  count: number
  positions: Float32Array
  prefabIdx: Uint16Array
  rotations: Float32Array
  /** RENDER_CLASS_CODES index per instance. */
  classes: Uint8Array
}

export interface ResolvedWorldObject {
  id: string
  prefabId: number
  resourceName: string | null
  kind: string
  class: string
  label: string | null
  renderClass: InstanceRenderClass | null
  position: [number, number]
  z: number
  rotationDeg: number
}

export interface WorldObjectsStatus {
  ready: boolean
}

export interface WorldObjectsCoreDeps {
  /** Fetch a URL to bytes; null = missing (404 / SPA-fallback HTML). Worker: HTTP; tests: fs. */
  fetchBytes: (url: string) => Promise<Uint8Array | null>
  /** Test override for the visibleInstances hard cap (defaults to contract INSTANCE_BUDGET). */
  instanceBudget?: number
  /** Test override for concurrent chunk fetches. */
  fetchConcurrency?: number
}

export interface WorldObjectsCoreApi {
  loadManifest(terrainId: string): Promise<WorldManifestLite | null>
  loadChunksInBbox(bbox: Bbox, marginCells: number, opts: LoadChunksOpts): Promise<ChunkLoadResult>
  loadForestMass(ids: string[], iso?: number): Promise<ForestMassResult>
  setDemGrid(grid: DemVectorGrid): void
  buildSeaBand(): SeaBandGeometry | null
  buildContours(intervalM: number): ContourResult | null
  visibleInstances(bbox: Bbox, deckZoom: number): Promise<VisibleSet>
  pickNearest(worldXY: [number, number], radiusM: number, deckZoom?: number): Promise<string | null>
  pickRect(bbox: Bbox, deckZoom?: number): Promise<string[]>
  resolve(id: string): Promise<ResolvedWorldObject | null>
  unload(): Promise<void>
  getStatus(): WorldObjectsStatus
}

/** Worker-side LRU floor — mirrors the main-thread chunkStore formula (plan §6 cache policy):
 *  cap = max(WORKER_LRU_MIN_CHUNKS, 3 × last-requested chunk count); the last-requested set
 *  itself is never evicted (it is the pinned visible set's superset). */
export const WORKER_LRU_MIN_CHUNKS = 64

const DEFAULT_FETCH_CONCURRENCY = 12

/** Gunzip-or-plain JSON parse. Static .gz files are served raw (no Content-Encoding), so
 *  sniff the gzip magic and run DecompressionStream (available in workers + Node ≥18). */
async function bytesToJson(buf: Uint8Array): Promise<unknown> {
  if (buf.length >= 2 && buf[0] === 0x1f && buf[1] === 0x8b) {
    const stream = new Blob([buf as BlobPart]).stream().pipeThrough(new DecompressionStream('gzip'))
    return JSON.parse(await new Response(stream).text()) as unknown
  }
  return JSON.parse(new TextDecoder().decode(buf)) as unknown
}

interface ParsedChunk {
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

/** The manifest `objects` block fields this core consumes (terrain-manifest schema). */
interface ObjectsBlock {
  prefabsPath?: string
  chunksPath?: string
  chunkSizeM?: number
  roadsPath?: string
  densityPath?: string
  instanceCount?: number
}

/** Worker-side density cache entry: the decoded tree-channel corner grid (never leaves the
 *  worker — geometry is recomputed per delivery so transferred buffers can't detach it). */
interface DensityCorners {
  corners: Uint16Array
  cols: number
  rows: number
  cellM: number
  treeMax: number
}

/** Narrow untyped prefab rows (prefabs.json.gz) to the clone-safe subset we ship + join on. */
function narrowPrefabRows(raw: unknown): WorldPrefabRow[] {
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

/** Narrow the export's chunk-index rows; null ⇒ no index shipped (full-grid sweep mode). */
function narrowCells(indexRaw: unknown): WorldChunkCell[] | null {
  const rawCells = (indexRaw as { cells?: unknown } | null)?.cells
  if (!Array.isArray(rawCells)) return null
  const cells: WorldChunkCell[] = []
  for (const c of rawCells) {
    const cell = c as { cx?: unknown; cy?: unknown; path?: unknown; instanceCount?: unknown }
    if (typeof cell.cx !== 'number' || typeof cell.cy !== 'number' || typeof cell.path !== 'string')
      continue
    cells.push({
      id: chunkIdOf(cell.cx, cell.cy),
      cx: cell.cx,
      cy: cell.cy,
      path: cell.path,
      instanceCount: typeof cell.instanceCount === 'number' ? cell.instanceCount : undefined,
    })
  }
  return cells
}

/** prefabId → render-class code + row, and the oversized flag (plan §6 oversized ring). */
function buildPrefabMaps(prefabRows: WorldPrefabRow[]): {
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

export function createWorldObjectsCore(deps: WorldObjectsCoreDeps): WorldObjectsCoreApi {
  const budget = deps.instanceBudget ?? INSTANCE_BUDGET
  const concurrency = deps.fetchConcurrency ?? DEFAULT_FETCH_CONCURRENCY

  const index = createWorldSpatialIndex()
  let manifest: WorldManifestLite | null = null
  /** In-flight loadManifest dedupe (T-090.5.5): chunkStore + forestMassStore + treeStore all
   *  call loadManifest at startup. Without this each concurrent caller hits the manifest-null
   *  branch, runs reset(), and re-fetches — a reset/refetch storm that can strand a later
   *  caller's promise unresolved (the tree glyph-0 bug). All callers share one load. */
  let manifestPromise: Promise<WorldManifestLite | null> | null = null
  let terrain: TerrainDef | null = null
  let assetBase = ''
  /** prefabId → [render class code, prefab row] for parse + resolve joins. */
  let prefabById = new Map<number, { code: number; row: WorldPrefabRow }>()
  let cellById = new Map<string, WorldChunkCell>()
  const chunks = new Map<string, ParsedChunk | null>() // null = known-missing/empty
  const inflight = new Map<string, Promise<ParsedChunk | null>>()
  // Density grids (T-090.8.1): tiny (≤ 625 × 289 u16 ≈ 0.4 MB) → cached for the session,
  // no LRU; null caches a missing/undecodable file so misses never refetch.
  const density = new Map<string, DensityCorners | null>()
  const densityInflight = new Map<string, Promise<DensityCorners | null>>()
  let densityPath: string | null = null
  let lastRequested = new Set<string>()
  let useTick = 0
  // DEM vector grid (T-090.5.4): the downsampled meters grid the sea-band + contour geometry
  // marches over. Pushed once per terrain by the main thread (demVectorStore); fully orthogonal
  // to the objects manifest — sea/contours work on DEM-only terrains where loadManifest returns
  // null. demPyramid caches 2×-reduced grids by reduction count (0 = base) for coarse intervals.
  let demBaseGrid: DemVectorGrid | null = null
  const demPyramid = new Map<number, DemVectorGrid>()

  async function fetchJson(url: string): Promise<unknown | null> {
    const bytes = await deps.fetchBytes(url)
    if (!bytes) return null
    return bytesToJson(bytes)
  }

  function reset(): void {
    index.clear()
    manifest = null
    terrain = null
    assetBase = ''
    prefabById = new Map()
    cellById = new Map()
    chunks.clear()
    inflight.clear()
    density.clear()
    densityInflight.clear()
    densityPath = null
    lastRequested = new Set()
    demBaseGrid = null
    demPyramid.clear()
  }

  /** Base grid reduced `n` times (memoized). n=0 is the pushed grid. */
  function reducedGrid(n: number): DemVectorGrid | null {
    if (!demBaseGrid) return null
    if (n <= 0) return demBaseGrid
    const cached = demPyramid.get(n)
    if (cached) return cached
    const prev = reducedGrid(n - 1)
    if (!prev) return null
    const g = reduceGrid2x(prev)
    demPyramid.set(n, g)
    return g
  }

  /** Deduping entry point: concurrent callers share the single in-flight load; a resolved
   *  manifest is cached in `manifest` so later calls short-circuit. reset() (inside doLoadManifest)
   *  deliberately does NOT clear manifestPromise — it is owned here (cleared when the load ends). */
  async function loadManifest(terrainId: string): Promise<WorldManifestLite | null> {
    if (manifest && manifest.terrainId === terrainId) return manifest
    manifestPromise ??= doLoadManifest(terrainId)
    try {
      return await manifestPromise
    } finally {
      manifestPromise = null
    }
  }

  async function doLoadManifest(terrainId: string): Promise<WorldManifestLite | null> {
    reset()
    const t = (TERRAINS as Record<string, TerrainDef | undefined>)[terrainId]
    if (!t?.manifestUrl) return null
    const root = (await fetchJson(t.manifestUrl).catch(() => null)) as {
      objects?: ObjectsBlock
    } | null
    const objects = root?.objects
    // No export for this terrain (Arland/custom) → v2 layers cleanly absent (plan R11).
    if (!objects?.prefabsPath || !objects.chunksPath) return null
    assetBase = t.manifestUrl.slice(0, t.manifestUrl.lastIndexOf('/'))

    const [prefabsRaw, indexRaw] = await Promise.all([
      fetchJson(`${assetBase}/${objects.prefabsPath}`),
      fetchJson(`${assetBase}/${objects.chunksPath}/manifest.json`).catch(() => null),
    ])
    const prefabRows = narrowPrefabRows(prefabsRaw)
    if (prefabRows.length === 0) return null
    const cells = narrowCells(indexRaw)
    const maps = buildPrefabMaps(prefabRows)

    prefabById = maps.byId
    cellById = new Map((cells ?? []).map((c) => [c.id, c]))
    terrain = t
    densityPath = typeof objects.densityPath === 'string' ? objects.densityPath : null
    manifest = {
      terrainId,
      chunkSizeM: objects.chunkSizeM ?? DEFAULT_CHUNK_SIZE_M,
      cells,
      prefabRows,
      roadsPath: objects.roadsPath ?? null,
      densityPath,
      instanceCount: typeof objects.instanceCount === 'number' ? objects.instanceCount : null,
      hasOversized: maps.hasOversized,
    }
    return manifest
  }

  /** Fetch + decode one chunk's TBDD grid (tree channel), joined on the in-flight map.
   *  Any failure (404, SPA HTML, bad magic/truncation) caches null — a known-empty. */
  async function ensureDensity(id: string): Promise<DensityCorners | null> {
    const cached = density.get(id)
    if (cached !== undefined) return cached
    let p = densityInflight.get(id)
    if (!p) {
      p = (async () => {
        let entry: DensityCorners | null = null
        try {
          const bytes = densityPath
            ? await deps.fetchBytes(`${assetBase}/${densityPath}/${id}.bin`)
            : null
          if (bytes) {
            const grid = wasm.decode_tbdd(bytes) // throws on bad magic/truncation → caught below
            try {
              const corners = grid.channel(0) // fresh Uint16Array copy (survives grid.free)
              let treeMax = 0
              for (const v of corners) if (v > treeMax) treeMax = v
              // All-zero grids are as empty as missing files — cache the null.
              if (treeMax > 0) {
                entry = { corners, cols: grid.cols, rows: grid.rows, cellM: grid.cell_m, treeMax }
              }
            } finally {
              grid.free()
            }
          }
        } catch {
          entry = null
        }
        density.set(id, entry)
        densityInflight.delete(id)
        return entry
      })()
      densityInflight.set(id, p)
    }
    return p
  }

  function chunkUrl(id: string): string {
    const cell = cellById.get(id)
    if (cell) return `${assetBase}/${cell.path}`
    // Full-grid sweep fallback (export without a chunk index): misses read as empty chunks.
    return `${assetBase}/objects/chunks/${id}.json.gz`
  }

  function parseChunk(id: string, raw: unknown): ParsedChunk | null {
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
    for (const raw of instances) {
      const row = narrowInstanceRow(raw)
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
      lastUsed: ++useTick,
    }
  }

  function indexChunk(chunk: ParsedChunk): void {
    const entries = new Array<{ id: string; x: number; y: number; cls: string }>(chunk.count)
    let n = 0
    for (let i = 0; i < chunk.count; i++) {
      const code = chunk.clsCodes[i]
      if (code === NO_CLASS) continue
      entries[n++] = {
        id: `${chunk.id}:${i}`,
        x: chunk.positions[2 * i],
        y: chunk.positions[2 * i + 1],
        cls: RENDER_CLASS_CODES[code],
      }
    }
    entries.length = n
    index.insertChunk(chunk.id, entries)
  }

  async function ensureChunk(id: string): Promise<ParsedChunk | null> {
    const cached = chunks.get(id)
    if (cached !== undefined) {
      if (cached) cached.lastUsed = ++useTick
      return cached
    }
    let p = inflight.get(id)
    if (!p) {
      p = (async () => {
        const raw = await fetchJson(chunkUrl(id)).catch(() => null)
        const parsed = raw ? parseChunk(id, raw) : null
        chunks.set(id, parsed) // parsed === null caches the miss (no refetch storm)
        if (parsed) indexChunk(parsed)
        inflight.delete(id)
        return parsed
      })()
      inflight.set(id, p)
    }
    return p
  }

  /** Evict least-recently-used parsed chunks beyond the cap; the most recent request set is
   *  never evicted (plan §6: refcounted visible set is a subset of it). */
  function evictBeyondCap(): void {
    const cap = Math.max(WORKER_LRU_MIN_CHUNKS, 3 * lastRequested.size)
    let loaded = 0
    for (const c of chunks.values()) if (c) loaded++
    if (loaded <= cap) return
    const evictable: ParsedChunk[] = []
    for (const c of chunks.values()) {
      if (c && !lastRequested.has(c.id)) evictable.push(c)
    }
    evictable.sort((a, b) => a.lastUsed - b.lastUsed)
    for (const c of evictable) {
      if (loaded <= cap) break
      index.removeChunk(c.id)
      chunks.delete(c.id)
      loaded--
    }
  }

  function sliceGroup(chunk: ParsedChunk, rows: Uint32Array): ChunkClassGroup {
    const n = rows.length
    const positions = new Float32Array(2 * n)
    const prefabIdx = new Uint16Array(n)
    const rotations = new Float32Array(n)
    const z = new Float32Array(n)
    for (let k = 0; k < n; k++) {
      const i = rows[k]
      positions[2 * k] = chunk.positions[2 * i]
      positions[2 * k + 1] = chunk.positions[2 * i + 1]
      prefabIdx[k] = chunk.prefabIdx[i]
      rotations[k] = chunk.rotations[i]
      z[k] = chunk.z[i]
    }
    return { count: n, positions, prefabIdx, rotations, z }
  }

  async function loadChunksInBbox(
    bbox: Bbox,
    marginCells: number,
    opts: LoadChunksOpts,
  ): Promise<ChunkLoadResult> {
    if (!manifest || !terrain) return { chunkSizeM: DEFAULT_CHUNK_SIZE_M, chunks: [] }
    const chunkSizeM = manifest.chunkSizeM

    let ids: string[]
    if (opts.ids) {
      ids = opts.ids
    } else {
      let rect = chunkRectForBbox(bbox, terrain, chunkSizeM)
      if (marginCells > 0) rect = expandChunkRect(rect, marginCells, terrain, chunkSizeM)
      ids = chunkIdsForRect(rect)
    }
    // Only chunks that exist on disk (chunk index authoritative when present).
    if (manifest.cells) ids = ids.filter((id) => cellById.has(id))
    lastRequested = new Set(ids)

    // Gate the delivered classes: caller's render set ∩ lodGates visibility (LOD5/W4 — trees
    // never cross the boundary below their band even if requested).
    const deliverClasses = opts.classes.filter((c) => classVisible(c, opts.deckZoom))
    const exclude = new Set(opts.excludeIds ?? [])

    const results: ChunkPayload[] = []
    let cursor = 0
    const workerLoop = async (): Promise<void> => {
      while (cursor < ids.length) {
        const id = ids[cursor++]
        const chunk = await ensureChunk(id)
        if (!chunk || exclude.has(id)) continue
        // Always deliver a payload (even when no requested class is present) so the main
        // store can cache "hydrated, nothing to draw" and never re-request the chunk.
        const groups: ChunkPayload['groups'] = {}
        for (const cls of deliverClasses) {
          const rows = chunk.rowsByClass[cls]
          if (rows && rows.length > 0) groups[cls] = sliceGroup(chunk, rows)
        }
        results.push({ id, cx: chunk.cx, cy: chunk.cy, totalInstances: chunk.count, groups })
      }
    }
    await Promise.all(
      Array.from({ length: Math.min(concurrency, Math.max(1, ids.length)) }, workerLoop),
    )
    evictBeyondCap()
    // Stable order for deterministic tests + apply order (fetch pool completes out of order).
    results.sort((a, b) => a.cy - b.cy || a.cx - b.cx)
    return { chunkSizeM, chunks: results }
  }

  function rowByGlobalId(id: string): { chunk: ParsedChunk; row: number } | null {
    const sep = id.lastIndexOf(':')
    if (sep <= 0) return null
    const chunk = chunks.get(id.slice(0, sep))
    const row = Number(id.slice(sep + 1))
    if (!chunk || !Number.isInteger(row) || row < 0 || row >= chunk.count) return null
    return { chunk, row }
  }

  return {
    loadManifest,
    loadChunksInBbox,

    /** Forest mass for the requested density chunks (T-090.8.1): TBDD fetch/decode (cached)
     *  → marching squares (forestMass.ts) at the chunk's world origin. Geometry arrays are
     *  fresh per call — safe for the worker shell to transfer. iso is a test/tuning knob;
     *  prod always passes the DENSITY_ISO default (the corner cache is iso-agnostic). */
    async loadForestMass(ids: string[], iso: number = DENSITY_ISO): Promise<ForestMassResult> {
      if (!manifest || !terrain || !densityPath) return { chunks: [], emptyIds: [...ids] }
      const chunkSizeM = manifest.chunkSizeM
      const results: ForestMassChunk[] = []
      const emptyIds: string[] = []
      let cursor = 0
      const workerLoop = async (): Promise<void> => {
        while (cursor < ids.length) {
          const id = ids[cursor++]
          const entry = await ensureDensity(id)
          if (!entry) {
            emptyIds.push(id)
            continue
          }
          const [cxStr, cyStr] = id.split('_')
          const cx = Number(cxStr)
          const cy = Number(cyStr)
          const r = wasm.forest_mass(
            entry.corners,
            entry.cols,
            entry.rows,
            cx * chunkSizeM,
            cy * chunkSizeM,
            entry.cellM,
            iso,
          )
          const geo = {
            fillPositions: r.fill_positions,
            fillStartIndices: r.fill_start_indices,
            outlineSegments: r.outline_segments,
          }
          r.free()
          if (geo.fillPositions.length === 0 && geo.outlineSegments.length === 0) {
            emptyIds.push(id)
            continue
          }
          results.push({ id, cx, cy, ...geo, treeMax: entry.treeMax })
        }
      }
      await Promise.all(
        Array.from({ length: Math.min(concurrency, Math.max(1, ids.length)) }, workerLoop),
      )
      // Stable order (fetch pool completes out of order) — deterministic composites + tests.
      results.sort((a, b) => a.cy - b.cy || a.cx - b.cx)
      emptyIds.sort()
      return { chunks: results, emptyIds }
    },

    /** Store the downsampled DEM grid the sea-band/contour geometry marches over (T-090.5.4).
     *  Replaces any prior grid + drops the reduction pyramid. The buffer arrives transferred
     *  from the main thread (a move — the main store never retains it). */
    setDemGrid(grid: DemVectorGrid): void {
      demBaseGrid = grid
      demPyramid.clear()
    },

    /** Sea-band fill geometry over the base grid, or null when no grid is loaded (the worker
     *  was restarted on mission unmount — the store re-pushes and retries). Fresh arrays. */
    buildSeaBand(): SeaBandGeometry | null {
      if (!demBaseGrid) return null
      const g = wasm.DemGrid.from_parts(
        demBaseGrid.data,
        demBaseGrid.cols,
        demBaseGrid.rows,
        demBaseGrid.cellX,
        demBaseGrid.cellY,
        demBaseGrid.originX,
        demBaseGrid.originY,
        demBaseGrid.maxElevM,
      )
      const r = g.sea_band()
      const out: SeaBandGeometry = {
        fillPositions: r.fill_positions,
        fillStartIndices: r.fill_start_indices,
        fillColors: r.fill_colors,
        polygonCount: r.polygon_count,
      }
      r.free()
      g.free()
      return out
    },

    /** Contour segments for an interval (coarse intervals march a reduced grid — plan R8), or
     *  null when no grid is loaded. Fresh arrays — safe for the worker shell to transfer. */
    buildContours(intervalM: number): ContourResult | null {
      const grid = reducedGrid(contourGridReductions(intervalM))
      if (!grid) return null
      const levels = contourLevels(intervalM, grid.maxElevM)
      const g = wasm.DemGrid.from_parts(
        grid.data,
        grid.cols,
        grid.rows,
        grid.cellX,
        grid.cellY,
        grid.originX,
        grid.originY,
        grid.maxElevM,
      )
      const segments = g.contours(new Float64Array(levels))
      g.free()
      return { intervalM, segments }
    },

    /** Instances visible in a bbox at a zoom (W4-v2 — the tree/prop glyph driver, T-090.5.5).
     *  Self-hydrates the covering chunks so the rbush holds this viewport before it's queried —
     *  no dependency on the building chunkStore (both drive the same worker core; ensureChunk is
     *  idempotent-cached). Skip-when-invisible: if no instance class draws at this zoom the answer
     *  is empty anyway → return without hydrating (a low-zoom bbox must never fetch the island).
     *  Gates + INSTANCE_BUDGET cap unchanged; LRU stays owned by loadChunksInBbox (never evict
     *  here — ensureChunk bumps lastUsed so a freshly hydrated viewport isn't the eviction victim). */
    async visibleInstances(bbox: Bbox, deckZoom: number): Promise<VisibleSet> {
      const anyClassVisible = RENDER_CLASS_CODES.some((c) => classVisible(c, deckZoom))
      if (anyClassVisible && manifest && terrain) {
        const chunkSizeM = manifest.chunkSizeM
        const rect = expandChunkRect(
          chunkRectForBbox(bbox, terrain, chunkSizeM),
          1,
          terrain,
          chunkSizeM,
        )
        let hydrateIds = chunkIdsForRect(rect)
        if (manifest.cells) hydrateIds = hydrateIds.filter((id) => cellById.has(id))
        await Promise.all(hydrateIds.map((id) => ensureChunk(id)))
      }
      const visibleCls = (cls: string) => classVisible(cls as WorldRenderClass, deckZoom)
      const ids = index.pickRect(bbox, visibleCls)
      const count = Math.min(ids.length, budget)
      const positions = new Float32Array(2 * count)
      const prefabIdx = new Uint16Array(count)
      const rotations = new Float32Array(count)
      const classes = new Uint8Array(count)
      for (let k = 0; k < count; k++) {
        const hit = rowByGlobalId(ids[k])
        if (!hit) continue
        const { chunk, row } = hit
        positions[2 * k] = chunk.positions[2 * row]
        positions[2 * k + 1] = chunk.positions[2 * row + 1]
        prefabIdx[k] = chunk.prefabIdx[row]
        rotations[k] = chunk.rotations[row]
        classes[k] = chunk.clsCodes[row]
      }
      return { count, positions, prefabIdx, rotations, classes }
    },

    async pickNearest(
      worldXY: [number, number],
      radiusM: number,
      deckZoom?: number,
    ): Promise<string | null> {
      const filter =
        deckZoom === undefined
          ? undefined
          : (cls: string) => classVisible(cls as WorldRenderClass, deckZoom)
      return index.pickNearest(worldXY[0], worldXY[1], radiusM, filter)
    },

    async pickRect(bbox: Bbox, deckZoom?: number): Promise<string[]> {
      const filter =
        deckZoom === undefined
          ? undefined
          : (cls: string) => classVisible(cls as WorldRenderClass, deckZoom)
      return index.pickRect(bbox, filter)
    },

    async resolve(id: string): Promise<ResolvedWorldObject | null> {
      const hit = rowByGlobalId(id)
      if (!hit) return null
      const { chunk, row } = hit
      const prefab = prefabById.get(chunk.prefabIdx[row])
      const code = chunk.clsCodes[row]
      return {
        id,
        prefabId: chunk.prefabIdx[row],
        resourceName: prefab?.row.resourceName ?? null,
        kind: prefab?.row.kind ?? 'unknown',
        class: prefab?.row.class ?? 'unknown',
        label: prefab?.row.label ?? null,
        renderClass: code === NO_CLASS ? null : RENDER_CLASS_CODES[code],
        position: [chunk.positions[2 * row], chunk.positions[2 * row + 1]],
        z: chunk.z[row],
        rotationDeg: chunk.rotations[row],
      }
    },

    async unload(): Promise<void> {
      reset()
    },

    getStatus(): WorldObjectsStatus {
      return { ready: manifest !== null }
    },
  }
}
