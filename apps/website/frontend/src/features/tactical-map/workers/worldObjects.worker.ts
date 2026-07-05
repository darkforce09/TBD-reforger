// World-objects Web Worker (T-090.5.3). Thin Comlink shell: all fetch + gunzip
// (DecompressionStream) + chunk parse + world rbush logic lives in worldObjectsCore.ts (a
// pure factory, node-testable — W1/W2/W4 run against it directly). This file only supplies
// the HTTP fetchBytes dep and marks result typed arrays as transferables so the worker→main
// hop moves buffers instead of copying them (plan §6 W-transfer rule).
//
// Worker-safety: comlink + pure modules only (worldObjectsCore → chunkMath/lodGates/
// terrains/worldSpatialIndex) — no DOM, no React, no deck.gl, no barrel imports
// (pattern: mission-creator/compiler/compiler.worker.ts).

import * as Comlink from 'comlink'
import {
  createWorldObjectsCore,
  type Bbox,
  type ChunkLoadResult,
  type ContourResult,
  type DemVectorGrid,
  type ForestMassResult,
  type LoadChunksOpts,
  type ResolvedWorldObject,
  type SeaBandGeometry,
  type VisibleSet,
  type WorldManifestLite,
  type WorldObjectsStatus,
} from './worldObjectsCore'

export type { WorldObjectsStatus }

/** Fetch a static asset to bytes. Vite dev SPA-fallbacks unknown paths to index.html with
 *  200, so an HTML content-type reads as missing (same rule the T-090.5.2 loader used). */
async function httpFetchBytes(url: string): Promise<Uint8Array | null> {
  const res = await fetch(url)
  const type = res.headers.get('content-type') ?? ''
  if (!res.ok || type.includes('text/html')) return null
  return new Uint8Array(await res.arrayBuffer())
}

const core = createWorldObjectsCore({ fetchBytes: httpFetchBytes })

/** Collect every typed-array buffer in a chunk payload set for a zero-copy transfer. The
 *  casts are sound: every group array is allocated here on a plain ArrayBuffer (the
 *  TypedArray.buffer type is ArrayBufferLike only because SharedArrayBuffer exists). */
function chunkBuffers(result: ChunkLoadResult): ArrayBuffer[] {
  const buffers: ArrayBuffer[] = []
  for (const chunk of result.chunks) {
    for (const group of Object.values(chunk.groups)) {
      buffers.push(
        group.positions.buffer as ArrayBuffer,
        group.prefabIdx.buffer as ArrayBuffer,
        group.rotations.buffer as ArrayBuffer,
        group.z.buffer as ArrayBuffer,
      )
    }
  }
  return buffers
}

const api = {
  /** Liveness probe for the client harness + smoke tests. */
  ping(): string {
    return 'world-objects-worker'
  },
  /** Capability report: ready once a terrain manifest is loaded. */
  getStatus(): WorldObjectsStatus {
    return core.getStatus()
  },

  loadManifest(terrainId: string): Promise<WorldManifestLite | null> {
    return core.loadManifest(terrainId)
  },

  async loadChunksInBbox(bbox: Bbox, marginCells: number, opts: LoadChunksOpts): Promise<ChunkLoadResult> {
    const result = await core.loadChunksInBbox(bbox, marginCells, opts)
    return Comlink.transfer(result, chunkBuffers(result))
  },

  /** Forest mass geometry (T-090.8.1) — buffers are fresh per call (core recomputes from
   *  its corner cache), so transferring them can never detach worker-side state. */
  async loadForestMass(ids: string[], iso?: number): Promise<ForestMassResult> {
    const result = await core.loadForestMass(ids, iso)
    const buffers: ArrayBuffer[] = []
    for (const chunk of result.chunks) {
      buffers.push(
        chunk.fillPositions.buffer as ArrayBuffer,
        chunk.fillStartIndices.buffer as ArrayBuffer,
        chunk.outlineSegments.buffer as ArrayBuffer,
      )
    }
    return Comlink.transfer(result, buffers)
  },

  /** Sea-band + contour source grid (T-090.5.4). The grid.data buffer arrives transferred from
   *  the main thread (a move) — core takes ownership; no reply payload. */
  setDemGrid(grid: DemVectorGrid): void {
    core.setDemGrid(grid)
  },

  /** Sea-band fill geometry (fresh arrays → transfer; null when the grid isn't loaded). */
  buildSeaBand(): SeaBandGeometry | null {
    const result = core.buildSeaBand()
    if (!result) return null
    return Comlink.transfer(result, [
      result.fillPositions.buffer as ArrayBuffer,
      result.fillStartIndices.buffer as ArrayBuffer,
      result.fillColors.buffer as ArrayBuffer,
    ])
  },

  /** Contour segments for an interval (fresh arrays → transfer; null when gridless). */
  buildContours(intervalM: number): ContourResult | null {
    const result = core.buildContours(intervalM)
    if (!result) return null
    return Comlink.transfer(result, [result.segments.buffer as ArrayBuffer])
  },

  async visibleInstances(bbox: Bbox, deckZoom: number): Promise<VisibleSet> {
    const result = await core.visibleInstances(bbox, deckZoom)
    return Comlink.transfer(result, [
      result.positions.buffer as ArrayBuffer,
      result.prefabIdx.buffer as ArrayBuffer,
      result.rotations.buffer as ArrayBuffer,
      result.classes.buffer as ArrayBuffer,
    ])
  },

  pickNearest(worldXY: [number, number], radiusM: number, deckZoom?: number): Promise<string | null> {
    return core.pickNearest(worldXY, radiusM, deckZoom)
  },

  pickRect(bbox: Bbox, deckZoom?: number): Promise<string[]> {
    return core.pickRect(bbox, deckZoom)
  },

  resolve(id: string): Promise<ResolvedWorldObject | null> {
    return core.resolve(id)
  },

  unload(): Promise<void> {
    return core.unload()
  },
}

/** RPC surface mirrored by the main-thread client (worldObjectsClient.ts). */
export type WorldObjectsWorkerApi = typeof api

Comlink.expose(api)
