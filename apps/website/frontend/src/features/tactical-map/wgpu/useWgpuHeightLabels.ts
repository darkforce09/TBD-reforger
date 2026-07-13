// T-152.7 — height marker controller: load peaks → declutter → text lane upload.

import { useEffect, useRef } from 'react'
import type { RefObject } from 'react'
import { getClassToggles, subscribeWorldLayerPrefs } from '../state/worldLayerPrefs'
import { getDemRasterForOverlay, loadDemForTerrain, subscribeDem } from '../dem/DemController'
import { fetchTerrainManifest } from '../dem/terrainManifest'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'
import {
  declutterHeightLabels,
  findPeaksFromMeters,
  packHeightLabelBytes,
  type HeightLabelRow,
} from './wgpuHeightLabels'

export class WgpuHeightLabelController {
  private disposed = false
  private allPeaks: HeightLabelRow[] = []
  private peaksReady = false
  private lastZoom = Number.NaN
  private lastVisible: boolean | null = null
  private engine: RenderEngine
  private terrain: TerrainDef

  constructor(engine: RenderEngine, terrain: TerrainDef) {
    this.engine = engine
    this.terrain = terrain
  }

  dispose(): void {
    this.disposed = true
    this.engine.upload_text_labels(new Uint8Array(0), false)
  }

  async init(): Promise<void> {
    if (this.disposed || this.peaksReady) return
    const manifestUrl = this.terrain.manifestUrl
    if (!manifestUrl) return

    // Prefer committed sidecar when present.
    const sidecarUrl = manifestUrl.replace(/manifest\.json$/, 'height-labels.json')
    try {
      const res = await fetch(sidecarUrl)
      if (res.ok) {
        const rows = (await res.json()) as HeightLabelRow[]
        if (Array.isArray(rows) && rows.length > 0) {
          this.allPeaks = rows
          this.peaksReady = true
          return
        }
      }
    } catch {
      /* fall through to DEM detect */
    }

    await loadDemForTerrain(this.terrain.id)
    const raster = getDemRasterForOverlay()
    if (!raster?.metersCache || this.disposed) return
    const manifest = await fetchTerrainManifest(manifestUrl)
    const dem = manifest.dem
    const flip = dem.axisFlip ?? {}
    this.allPeaks = findPeaksFromMeters(raster.metersCache as Float32Array, raster.width, raster.height, {
      minX: 0,
      minY: 0,
      maxX: this.terrain.width,
      maxY: this.terrain.height,
      flipX: Boolean(flip.x),
      flipZ: Boolean(flip.z),
    })
    this.peaksReady = true
  }

  sync(engine: RenderEngine, deckZoom: number): void {
    if (this.disposed || !this.peaksReady) return
    const heightsOn = getClassToggles().heights
    if (heightsOn === this.lastVisible && deckZoom === this.lastZoom) return
    this.lastVisible = heightsOn
    this.lastZoom = deckZoom
    if (!heightsOn) {
      engine.upload_text_labels(new Uint8Array(0), false)
      return
    }
    void engine.ensure_text_atlas()
    const drawn = declutterHeightLabels(this.allPeaks, deckZoom)
    const bytes = packHeightLabelBytes(drawn, deckZoom)
    engine.upload_text_labels(bytes, true)
  }
}

export function useWgpuHeightLabels(
  engineRef: RefObject<RenderEngine | null>,
  ready: boolean,
  opts: { terrain: TerrainDef },
): void {
  const ctrlRef = useRef<WgpuHeightLabelController | null>(null)
  const zoomRef = useRef(-2)

  useEffect(() => {
    if (!ready) return
    const eng = engineRef.current
    if (!eng) return
    const ctrl = new WgpuHeightLabelController(eng, opts.terrain)
    ctrlRef.current = ctrl
    void ctrl.init().then(() => {
      const e = engineRef.current
      if (e) ctrl.sync(e, zoomRef.current)
    })
    const unsubDem = subscribeDem(() => {
      const e = engineRef.current
      if (e) void ctrl.init().then(() => ctrl.sync(e, zoomRef.current))
    })
    const unsubPrefs = subscribeWorldLayerPrefs(() => {
      const e = engineRef.current
      if (e) ctrl.sync(e, zoomRef.current)
    })
    return () => {
      unsubDem()
      unsubPrefs()
      ctrl.dispose()
      ctrlRef.current = null
    }
  }, [ready, opts.terrain, engineRef])

  useEffect(() => {
    if (!ready) return
    let raf = 0
    const tick = () => {
      const eng = engineRef.current
      const ctrl = ctrlRef.current
      if (eng && ctrl) {
        const z = eng.zoom
        if (z !== zoomRef.current) {
          zoomRef.current = z
          ctrl.sync(eng, z)
        }
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [ready, engineRef])
}
