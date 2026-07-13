// T-152.8 — town label controller: load locations.json → declutter → text lane upload.

import { useEffect, useRef } from 'react'
import type { RefObject } from 'react'
import { getClassToggles, subscribeWorldLayerPrefs } from '../state/worldLayerPrefs'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'
import {
  declutterTownLabels,
  packTownLabelBytes,
  parseLocationsJson,
  type TownLabelRow,
} from './wgpuTownLabels'

export class WgpuTownLabelController {
  private disposed = false
  private allLocations: TownLabelRow[] = []
  private locationsReady = false
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
    this.engine.upload_town_labels(new Uint8Array(0), false)
  }

  async init(): Promise<void> {
    if (this.disposed || this.locationsReady) return
    const manifestUrl = this.terrain.manifestUrl
    if (!manifestUrl) return

    const locUrl = manifestUrl.replace(/manifest\.json$/, 'locations.json')
    try {
      const res = await fetch(locUrl)
      if (!res.ok) return
      const text = await res.text()
      const rows = parseLocationsJson(text)
      if (rows.length > 0) {
        this.allLocations = rows
        this.locationsReady = true
      }
    } catch {
      /* locations sidecar optional until T-152.6 lands on terrain */
    }
  }

  sync(engine: RenderEngine, deckZoom: number): void {
    if (this.disposed || !this.locationsReady) return
    const townOn = getClassToggles().townLabels
    if (townOn === this.lastVisible && deckZoom === this.lastZoom) return
    this.lastVisible = townOn
    this.lastZoom = deckZoom
    if (!townOn) {
      engine.upload_town_labels(new Uint8Array(0), false)
      return
    }
    void engine.ensure_text_atlas()
    const drawn = declutterTownLabels(this.allLocations, deckZoom)
    const bytes = packTownLabelBytes(drawn, deckZoom)
    engine.upload_town_labels(bytes, true)
  }
}

export function useWgpuTownLabels(
  engineRef: RefObject<RenderEngine | null>,
  ready: boolean,
  opts: { terrain: TerrainDef },
): void {
  const ctrlRef = useRef<WgpuTownLabelController | null>(null)
  const zoomRef = useRef(-2)

  useEffect(() => {
    if (!ready) return
    const eng = engineRef.current
    if (!eng) return
    const ctrl = new WgpuTownLabelController(eng, opts.terrain)
    ctrlRef.current = ctrl
    void ctrl.init().then(() => {
      const e = engineRef.current
      if (e) ctrl.sync(e, zoomRef.current)
    })
    const unsubPrefs = subscribeWorldLayerPrefs(() => {
      const e = engineRef.current
      if (e) ctrl.sync(e, zoomRef.current)
    })
    return () => {
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
