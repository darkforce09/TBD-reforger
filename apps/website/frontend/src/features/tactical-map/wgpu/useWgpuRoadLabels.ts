// T-152.9 — road label controller: load road-names.json + roads.json.gz → declutter → text lane.

import { useEffect, useRef } from 'react'
import type { RefObject } from 'react'
import { getClassToggles, subscribeWorldLayerPrefs } from '../state/worldLayerPrefs'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'
import { buildRoadLabels, packRoadLabelBytes } from './wgpuRoadLabels'

export class WgpuRoadLabelController {
  private disposed = false
  private roadNamesJson = ''
  private roadsJson = ''
  private dataReady = false
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
    this.engine.upload_road_labels(new Uint8Array(0), false)
  }

  async init(): Promise<void> {
    if (this.disposed || this.dataReady) return
    const manifestUrl = this.terrain.manifestUrl
    if (!manifestUrl) return

    const base = manifestUrl.replace(/manifest\.json$/, '')
    const namesUrl = `${base}road-names.json`
    const roadsUrl = `${base}objects/roads.json.gz`
    try {
      const [namesRes, roadsRes] = await Promise.all([fetch(namesUrl), fetch(roadsUrl)])
      if (!namesRes.ok || !roadsRes.ok) return
      const namesText = await namesRes.text()
      const roadsBuf = await roadsRes.arrayBuffer()
      const roadsText = new TextDecoder().decode(
        roadsUrl.endsWith('.gz') ? await decompressGzip(roadsBuf) : roadsBuf,
      )
      if (namesText.trim() && roadsText.trim()) {
        this.roadNamesJson = namesText
        this.roadsJson = roadsText
        this.dataReady = true
      }
    } catch {
      /* optional until road-names sidecar present */
    }
  }

  sync(engine: RenderEngine, deckZoom: number): void {
    if (this.disposed || !this.dataReady) return
    const roadOn = getClassToggles().roadNames
    if (roadOn === this.lastVisible && deckZoom === this.lastZoom) return
    this.lastVisible = roadOn
    this.lastZoom = deckZoom
    if (!roadOn) {
      engine.upload_road_labels(new Uint8Array(0), false)
      return
    }
    void engine.ensure_text_atlas()
    const drawn = buildRoadLabels(this.roadNamesJson, this.roadsJson, deckZoom)
    const bytes = packRoadLabelBytes(drawn, deckZoom)
    engine.upload_road_labels(bytes, true)
  }
}

async function decompressGzip(buf: ArrayBuffer): Promise<Uint8Array> {
  const ds = new DecompressionStream('gzip')
  const stream = new Blob([buf]).stream().pipeThrough(ds)
  const out = await new Response(stream).arrayBuffer()
  return new Uint8Array(out)
}

export function useWgpuRoadLabels(
  engineRef: RefObject<RenderEngine | null>,
  ready: boolean,
  opts: { terrain: TerrainDef },
): void {
  const ctrlRef = useRef<WgpuRoadLabelController | null>(null)
  const zoomRef = useRef(-2)

  useEffect(() => {
    if (!ready) return
    const eng = engineRef.current
    if (!eng) return
    const ctrl = new WgpuRoadLabelController(eng, opts.terrain)
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
