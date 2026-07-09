// T-151.4 — Forest mass for wgpu: TBDD viewport stream (mirror forestMassStore, no LRU).
// Session cache of density chunks; composite grows with exploration (N11 P2b).

import { useEffect } from 'react'
import type { RefObject } from 'react'
import { decode_tbdd, forest_mass, density_iso, class_visible } from '@/wasm/pkg/map_engine_wasm'
import { chunkIdsForViewport, type Bbox } from '../worldmap/chunkMath'
import { forestFillAlpha } from '../worldmap/forestMass'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'

const ROLE_FOREST_FILL = 5
const ROLE_FOREST_OUTLINE = 6
const FETCH_CONCURRENCY = 12
const MOVE_DEBOUNCE_MS = 120
const CHUNK_SIZE_M = 512

interface ComposedChunk {
  fillPositions: Float32Array
  fillColors: Float32Array
  fillIndices: Uint32Array
  fillPolyCount: number
  outlineVerts: Float32Array
  outlineSegCount: number
}

async function httpFetchBytes(url: string, signal: AbortSignal): Promise<Uint8Array | null> {
  const res = await fetch(url, { signal })
  const type = res.headers.get('content-type') ?? ''
  if (!res.ok || type.includes('text/html')) return null
  return new Uint8Array(await res.arrayBuffer())
}

/**
 * Viewport-driven forest-mass controller for the wgpu mount.
 * No eviction — composite covers every hydrated chunk (Deck forestMassStore policy).
 */
export class WgpuForestMassController {
  private readonly engine: RenderEngine
  private readonly terrain: TerrainDef
  private disposed = false
  private assetBase = ''
  private ready = false
  private readonly cache = new Map<string, ComposedChunk | null>()
  private fetchAc: AbortController | null = null
  private moveTimer: ReturnType<typeof setTimeout> | null = null
  private lastKey = ''

  constructor(engine: RenderEngine, terrain: TerrainDef) {
    this.engine = engine
    this.terrain = terrain
  }

  async init(): Promise<void> {
    if (this.disposed || this.ready) return
    const url = this.terrain.manifestUrl
    if (!url) return
    this.assetBase = url.slice(0, url.lastIndexOf('/'))
    this.ready = true
    this.runViewport()
  }

  onCameraMoved(): void {
    if (this.disposed || !this.ready) return
    if (this.moveTimer) clearTimeout(this.moveTimer)
    this.moveTimer = setTimeout(() => this.runViewport(), MOVE_DEBOUNCE_MS)
  }

  dispose(): void {
    this.disposed = true
    this.fetchAc?.abort()
    if (this.moveTimer) clearTimeout(this.moveTimer)
  }

  private runViewport(): void {
    if (this.disposed || !this.ready) return
    const b = this.engine.visible_bounds()
    const zoom = this.engine.zoom
    const bbox: Bbox = [b[0], b[1], b[2], b[3]]
    const ids = chunkIdsForViewport(
      bbox,
      { width: this.terrain.width, height: this.terrain.height },
      { chunkSizeM: CHUNK_SIZE_M },
    )
    const key = ids.join(',')
    if (key !== this.lastKey) {
      this.lastKey = key
      const missing = ids.filter((id) => !this.cache.has(id))
      if (missing.length > 0) {
        void this.fetchMissing(missing, zoom)
        return
      }
    }
    this.pushComposite(zoom)
  }

  private async fetchMissing(ids: string[], zoom: number): Promise<void> {
    this.fetchAc?.abort()
    const ac = new AbortController()
    this.fetchAc = ac
    let cursor = 0
    const worker = async (): Promise<void> => {
      for (;;) {
        const i = cursor++
        if (i >= ids.length) break
        const id = ids[i]
        try {
          const bytes = await httpFetchBytes(
            `${this.assetBase}/objects/density/${id}.bin`,
            ac.signal,
          )
          if (ac.signal.aborted || this.disposed) return
          this.cache.set(id, bytes ? this.composeChunk(id, bytes) : null)
        } catch {
          if (ac.signal.aborted) return
          this.cache.set(id, null)
        }
      }
    }
    try {
      await Promise.all(
        Array.from({ length: Math.min(FETCH_CONCURRENCY, ids.length) }, () => worker()),
      )
    } catch {
      return
    }
    if (ac.signal.aborted || this.disposed) return
    this.pushComposite(zoom)
  }

  private composeChunk(id: string, bytes: Uint8Array): ComposedChunk | null {
    try {
      const tbdd = decode_tbdd(bytes)
      const tree = tbdd.channel(0)
      const parts = id.split('_').map(Number)
      // Rust owns DENSITY_ISO — never pass a TS iso (T-151.5.1).
      const mass = forest_mass(
        tree,
        tbdd.cols,
        tbdd.rows,
        (parts[0] ?? 0) * CHUNK_SIZE_M,
        (parts[1] ?? 0) * CHUNK_SIZE_M,
        tbdd.cell_m,
        density_iso(),
      )
      // Compose at alpha 1; apply zoom α by recolouring on push.
      const composed = mass.compose(1.0)
      const out: ComposedChunk = {
        fillPositions: composed.fill_positions,
        fillColors: composed.fill_colors,
        fillIndices: composed.fill_indices,
        fillPolyCount: composed.fill_polygon_count,
        outlineVerts: composed.outline_verts,
        outlineSegCount: composed.outline_segment_count,
      }
      composed.free()
      mass.free()
      tbdd.free()
      return out
    } catch {
      return null
    }
  }

  private pushComposite(zoom: number): void {
    if (this.disposed) return
    // Prefer Rust lod_gates via wasm (T-151.5.1 L2/L3).
    const fillVis = class_visible('forestFill', zoom)
    const outVis = class_visible('forestOutline', zoom)
    const alpha = forestFillAlpha(zoom)
    const chunks = this.loadedChunks()
    if (chunks.length === 0) {
      this.engine.clear_vector_lane(ROLE_FOREST_FILL)
      this.engine.clear_vector_lane(ROLE_FOREST_OUTLINE)
      return
    }
    this.pushFill(chunks, fillVis, alpha)
    this.pushOutline(chunks, outVis)
  }

  private loadedChunks(): ComposedChunk[] {
    const chunks: ComposedChunk[] = []
    for (const id of [...this.cache.keys()].sort()) {
      const c = this.cache.get(id)
      if (c) chunks.push(c)
    }
    return chunks
  }

  private pushFill(chunks: ComposedChunk[], fillVis: boolean, alpha: number): void {
    if (!fillVis || alpha <= 0) {
      this.engine.clear_vector_lane(ROLE_FOREST_FILL)
      return
    }
    let vTotal = 0
    let iTotal = 0
    let polyTotal = 0
    for (const c of chunks) {
      vTotal += c.fillPositions.length / 2
      iTotal += c.fillIndices.length
      polyTotal += c.fillPolyCount
    }
    const positions = new Float32Array(vTotal * 2)
    const colors = new Float32Array(vTotal * 4)
    const indices = new Uint32Array(iTotal)
    let vBase = 0
    let iBase = 0
    for (const c of chunks) {
      const nv = c.fillPositions.length / 2
      positions.set(c.fillPositions, vBase * 2)
      for (let i = 0; i < nv; i++) {
        const s = i * 4
        const d = (vBase + i) * 4
        colors[d] = c.fillColors[s]
        colors[d + 1] = c.fillColors[s + 1]
        colors[d + 2] = c.fillColors[s + 2]
        colors[d + 3] = c.fillColors[s + 3] * alpha
      }
      for (let k = 0; k < c.fillIndices.length; k++) {
        indices[iBase + k] = c.fillIndices[k] + vBase
      }
      vBase += nv
      iBase += c.fillIndices.length
    }
    this.engine.upload_polygon_mesh(ROLE_FOREST_FILL, positions, colors, indices, polyTotal, true)
  }

  private pushOutline(chunks: ComposedChunk[], outVis: boolean): void {
    if (!outVis) {
      this.engine.clear_vector_lane(ROLE_FOREST_OUTLINE)
      return
    }
    let sTotal = 0
    let segTotal = 0
    for (const c of chunks) {
      sTotal += c.outlineVerts.length
      segTotal += c.outlineSegCount
    }
    const verts = new Float32Array(sTotal)
    let off = 0
    for (const c of chunks) {
      verts.set(c.outlineVerts, off)
      off += c.outlineVerts.length
    }
    this.engine.upload_hairline_segments(ROLE_FOREST_OUTLINE, verts, segTotal, true)
  }
}

export function useWgpuForestMass(
  controllerRef: RefObject<WgpuForestMassController | null>,
  ready: boolean,
): void {
  useEffect(() => {
    if (!ready) return
    void controllerRef.current?.init()
  }, [ready, controllerRef])
}
