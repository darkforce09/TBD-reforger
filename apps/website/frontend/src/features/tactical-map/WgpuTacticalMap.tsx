// The wgpu `RenderEngine` editor mount — since T-151.9 this is the ONLY Mission Creator map
// engine (the Deck runtime is deleted; `MissionCreatorPage` mounts this unconditionally).
// Renders the full stack: basemap (satellite unified / pyramid + hillshade + grid), world lanes
// (buildings/roads/landcover/forest/glyphs), and mission lanes (slots/selection/drag/clusters/
// marquee). The T-151.0 calibration scene is hidden (`hide_calibration`); it survives only on
// the DEV-only /_spike/wgpu page. Interaction: useSelectTool + page callbacks on the ULP-0
// engine camera (T-151.7; no LMB pan steal).
//
// Wasm memory lifecycle invariants I2–I7 are reused VERBATIM from WgpuCanvas.tsx (the spike page):
//   I2 the engine handle is EFFECT-LOCAL — never useMemo/useState-persisted; `.free()` is NOT
//      idempotent (see the wasm-react-lifecycle memory).
//   I4 free exactly once on every path: cleanup frees iff committed; the async create path checks
//      `disposed` right after the await and frees-without-committing.
//   I5 no render-after-free: the rAF loop re-checks the flag; rAF is cancelled before free.
//   I6 errors surface into the banner, never swallowed.
//   I7 retry = a NEW canvas element (React key bump) forced to WebGL2 — a canvas permanently
//      commits to its first getContext kind, so same-canvas backend retry is impossible.
// (I1 module-init once + I3 creation mutex live in ./wgpu/wasmRender.) Terrain switches remount the
// whole component (MissionCreatorPage keys it on terrain) → a fresh canvas + engine.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from 'react'
import { WHEEL_ZOOM_PER_PX, createEngine, deviceSize, type RenderEngine } from './wgpu/wasmRender'
import { createTextLabelStore, type TextLabelStore } from './wgpu/wgpuTextLane'
import { WgpuBasemapController } from './wgpu/wgpuBasemap'
import { useWgpuBasemap } from './wgpu/useWgpuBasemap'
import { WgpuWorldController } from './wgpu/wgpuWorldLoader'
import { useWgpuWorldResidency } from './wgpu/useWgpuWorldResidency'
import { useWgpuDemVectors } from './wgpu/useWgpuDemVectors'
import { useWgpuHeightLabels } from './wgpu/useWgpuHeightLabels'
import { useWgpuTownLabels } from './wgpu/useWgpuTownLabels'
import { useWgpuRoadLabels } from './wgpu/useWgpuRoadLabels'
import {
  WgpuForestMassController,
  useWgpuForestMass,
} from './wgpu/useWgpuForestMass'
import { WgpuSlotsController } from './wgpu/wgpuSlots'
import { useWgpuSlots } from './wgpu/useWgpuSlots'
import { useMapStore } from './state/useMapStore'
import { useClassToggles } from './state/worldLayerPrefs'
import { getTerrain } from './coords/terrains'
import { terrainCenterPixel } from './coords/projection'
import { loadDemForTerrain, sampleElevation, isDemReady } from './dem'
import { useDemVersion } from './dem/useDemVersion'
import { ZOOM_CLUSTER_MAX, CLUSTER_SLOT_THRESHOLD } from './state/constants'
import * as slotSpatialIndex from './state/slotSpatialIndex'
import * as slotClusterIndex from './state/slotClusterIndex'
import * as slotIconCache from './state/slotIconCache'
import { useSelectTool } from './tools/useSelectTool'
import {
  MAP_MAX_ZOOM,
  MAP_MIN_ZOOM,
  applyViewState,
  clampMapZoom,
  viewportFromViewState,
  viewStateFromEngine,
} from './tools/mapCamera'
import { MapContextProvider, createMapContextValue } from './context/MapContext'
import {
  ASSET_DND_MIME,
  type AssetDropPayload,
  type MapViewState,
  type TacticalMapProps,
} from './types'
import type { WasmMissionDoc } from './state/wasmDoc'

const HUD_INTERVAL_MS = 250

export type WgpuTacticalMapProps = TacticalMapProps & {
  /** Live mission doc shell — SoA source for W6 slot lanes. */
  missionDoc?: WasmMissionDoc | null
}

/**
 * The wgpu engine mount for the editor (T-151.1 + T-151.6 slots + T-151.7 interaction).
 * Same gesture machine + page callbacks as Deck; camera is ULP-0 OrthoCamera via RenderEngine.
 */
export default function WgpuTacticalMap({
  terrain: terrainId = 'everon',
  className,
  showGrid = true,
  showHillshade = false,
  hillshadeOpacity = 0.4,
  onBasemapDegraded,
  onBasemapProgress,
  onCursorMove,
  onReady,
  onEntityActivate,
  onAssetDrop,
  onEntitiesMove,
  missionDoc = null,
}: WgpuTacticalMapProps) {
  const terrainDef = getTerrain(terrainId)
  const containerRef = useRef<HTMLDivElement | null>(null)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const controllerRef = useRef<WgpuBasemapController | null>(null)
  const worldControllerRef = useRef<WgpuWorldController | null>(null)
  const forestControllerRef = useRef<WgpuForestMassController | null>(null)
  const slotsControllerRef = useRef<WgpuSlotsController | null>(null)
  const engineRef = useRef<RenderEngine | null>(null)
  const textLabelStoreRef = useRef<TextLabelStore | null>(null)
  const [canvasKey, setCanvasKey] = useState(0)
  const [forceWebgl, setForceWebgl] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [backend, setBackend] = useState('initializing…')
  const [fps, setFps] = useState(0)
  const [basemapMode, setBasemapMode] = useState('…')
  const [ready, setReady] = useState(false)
  const marquee = useMapStore((s) => s.marquee)
  const classToggles = useClassToggles()

  // View state mirror. T-151.7.2: engine OrthoCamera is SoT; viewStateRef is updated
  // SYNCHRONOUSLY on every mutation so pick/pan never lag a frame behind wheel.
  const [viewState, setViewState] = useState<MapViewState>(() => ({
    target: terrainCenterPixel(terrainDef),
    zoom: -2,
    minZoom: MAP_MIN_ZOOM,
    maxZoom: MAP_MAX_ZOOM,
  }))
  const viewStateRef = useRef(viewState)
  useEffect(() => {
    viewStateRef.current = viewState
  }, [viewState])

  // Latest-callback refs: the controller is created ONCE (effect-local), so it must call the
  // current prop callbacks (MissionCreatorPage's are useCallback-stable, but this is robust anyway).
  const onDegradedRef = useRef(onBasemapDegraded)
  const onProgressRef = useRef(onBasemapProgress)
  useEffect(() => {
    onDegradedRef.current = onBasemapDegraded
    onProgressRef.current = onBasemapProgress
  })

  // Prop → controller effects (basemap view, hillshade, grid, opacity re-tints, paper tint).
  useWgpuBasemap(controllerRef, ready, {
    terrainId,
    showGrid,
    showHillshade,
    hillshadeOpacity,
  })
  // World-object residency (W3): kicks the manifest/prefab/chunk-index load once ready.
  useWgpuWorldResidency(worldControllerRef, ready, { terrainId })
  // W4 DEM vectors (sea + contours).
  useWgpuDemVectors(engineRef, ready, { terrain: terrainDef })
  useWgpuHeightLabels(engineRef, ready, { terrain: terrainDef })
  useWgpuTownLabels(engineRef, ready, { terrain: terrainDef })
  useWgpuRoadLabels(engineRef, ready, { terrain: terrainDef })
  // W4 forest mass (TBDD viewport stream).
  useWgpuForestMass(forestControllerRef, ready)
  // W6 mission slots / selection / drag / clusters.
  useWgpuSlots(slotsControllerRef, ready, missionDoc)
  // W5 glyph prefs (trees/props/buildings→badges) + T-152.20 world-layer toggles (roads/forest/
  // airfield). syncGlyphToggles re-pushes roads + landcover; the forest-mass lane resyncs alongside
  // (the landcover hulls + forest mass are the two halves of the Forest-mass toggle). airfield was
  // missing from the deps (T-152.15 set_airfield_toggle wouldn't take effect until a camera move).
  // Sea + Contours are owned by useWgpuDemVectors (its own prefs subscription), so not listed here.
  useEffect(() => {
    if (!ready) return
    worldControllerRef.current?.syncGlyphToggles()
    forestControllerRef.current?.resync()
  }, [
    ready,
    classToggles.trees,
    classToggles.props,
    classToggles.buildings,
    classToggles.fences,
    classToggles.airfield,
    classToggles.roads,
    classToggles.forest,
  ])

  // Keep cluster index + icon cache terrain aligned (same as Deck TacticalMap).
  useEffect(() => {
    slotClusterIndex.setTerrain(terrainDef)
    slotIconCache.setChunkTerrain(terrainDef)
  }, [terrainDef])

  // DEM for CUR z (T-091); useWgpuDemVectors also loads, but ensure terrain id is current.
  useEffect(() => {
    void loadDemForTerrain(terrainDef.id)
  }, [terrainDef.id])

  const demVersion = useDemVersion()

  // W4 marquee polygon (store-driven; select tool writes marquee).
  useEffect(() => {
    if (!ready) return
    const eng = engineRef.current
    if (!eng) return
    if (marquee) {
      const minX = Math.min(marquee.x0, marquee.x1)
      const maxX = Math.max(marquee.x0, marquee.x1)
      const minY = Math.min(marquee.y0, marquee.y1)
      const maxY = Math.max(marquee.y0, marquee.y1)
      eng.upload_marquee(minX, minY, maxX, maxY, true)
    } else {
      eng.clear_vector_lane(7)
    }
  }, [ready, marquee])

  const notifyCameraMoved = useCallback(() => {
    controllerRef.current?.onCameraMoved()
    worldControllerRef.current?.onCameraMoved()
    forestControllerRef.current?.onCameraMoved()
    slotsControllerRef.current?.onCameraMoved()
  }, [])

  const clampViewState = useCallback(
    (next: MapViewState): MapViewState => {
      const zoom = clampMapZoom(next.zoom)
      const [tx, ty] = next.target
      return {
        target: [
          Math.min(Math.max(tx, 0), terrainDef.width),
          Math.min(Math.max(ty, 0), terrainDef.height),
        ],
        zoom,
        minZoom: MAP_MIN_ZOOM,
        maxZoom: MAP_MAX_ZOOM,
      }
    },
    [terrainDef.width, terrainDef.height],
  )

  /**
   * Pan path (useSelectTool): only target moves. Preserve live zoom from viewStateRef so a
   * mid-pan wheel's zoom_at is not clobbered by stale React viewState (T-151.7.2 B2).
   */
  const onViewStateChange = useCallback(
    ({ viewState: next }: { viewState: MapViewState }) => {
      const live = viewStateRef.current
      const merged = clampViewState({
        ...next,
        // Pan never owns zoom — keep whatever wheel/flyTo last committed.
        zoom: live.zoom,
        target: next.target,
      })
      viewStateRef.current = merged
      setViewState(merged)
      const eng = engineRef.current
      if (eng) {
        applyViewState(eng, merged)
        notifyCameraMoved()
      }
    },
    [clampViewState, notifyCameraMoved],
  )

  const slotCount = useMapStore((s) => s.slotCount)
  const clusterMode = slotCount > CLUSTER_SLOT_THRESHOLD && viewState.zoom <= ZOOM_CLUSTER_MAX

  const flyToInternal = useCallback(
    (target: [number, number], zoomDelta?: number) => {
      const prev = viewStateRef.current
      const next = clampViewState({
        ...prev,
        target: [target[0], target[1]],
        zoom: zoomDelta ? clampMapZoom(prev.zoom + zoomDelta) : prev.zoom,
      })
      viewStateRef.current = next
      setViewState(next)
      const eng = engineRef.current
      if (eng) {
        applyViewState(eng, next)
        notifyCameraMoved()
      }
    },
    [clampViewState, notifyCameraMoved],
  )

  const drillIntoCluster = useCallback(
    (world: { x: number; y: number }) => flyToInternal([world.x, world.y], 1),
    [flyToInternal],
  )

  /** Snapshot the LIVE engine camera when ready so pick/pan match GPU (T-151.7.2 B3).
   *  Deliberately a FROZEN snapshot (`viewportFromViewState`), not a live engine viewport —
   *  gestures (pan/move/marquee) freeze the camera at gesture start; a live unproject would
   *  feedback-loop as the pan itself moves the target (T-151.11.2 review note). */
  const getViewport = useCallback(() => {
    const el = containerRef.current
    if (!el) return null
    const r = el.getBoundingClientRect()
    const eng = engineRef.current
    if (eng) {
      return viewportFromViewState(r.width, r.height, viewStateFromEngine(eng))
    }
    return viewportFromViewState(r.width, r.height, viewStateRef.current)
  }, [])

  const noopMove = useCallback(() => {
    // Host may omit onEntitiesMove; gesture SM still needs a stable handler.
  }, [])

  const getLiveViewState = useCallback(() => viewStateRef.current, [])

  // Pass live ref so pan/wheel never freeze a stale React-prop camera (T-151.7.2).
  const selectTool = useSelectTool({
    containerRef,
    getViewport,
    viewState,
    getLiveViewState,
    onViewStateChange,
    onEntitiesMove: onEntitiesMove ?? noopMove,
    clusterMode,
    onClusterDrill: drillIntoCluster,
  })
  // Wheel lives in the engine effect (stable listener); keep rebasePan current via ref.
  const rebasePanRef = useRef(selectTool.rebasePan)
  useEffect(() => {
    rebasePanRef.current = selectTool.rebasePan
  }, [selectTool.rebasePan])

  // Cursor rAF channel (T-057) + DEM z (T-091.2) — same contract as Deck TacticalMap.
  const cursorRaf = useRef(0)
  const lastClientPt = useRef<{ x: number; y: number } | null>(null)
  const recomputeCursor = useCallback(() => {
    if (!onCursorMove) return
    const el = containerRef.current
    const pt = lastClientPt.current
    if (!el || !pt) return
    const rect = el.getBoundingClientRect()
    // Prefer live engine snapshot so CUR matches rings after wheel (T-151.7.2).
    const eng = engineRef.current
    const vs = eng ? viewStateFromEngine(eng) : viewStateRef.current
    const vp = viewportFromViewState(rect.width, rect.height, vs)
    const [x, y] = vp.unproject([pt.x - rect.left, pt.y - rect.top])
    onCursorMove({ x, y, z: isDemReady() ? sampleElevation(x, y) : 0 })
  }, [onCursorMove])

  const emitCursor = useCallback(
    (e: React.PointerEvent) => {
      if (!onCursorMove) return
      lastClientPt.current = { x: e.clientX, y: e.clientY }
      if (cursorRaf.current) return
      cursorRaf.current = requestAnimationFrame(() => {
        cursorRaf.current = 0
        recomputeCursor()
      })
    },
    [onCursorMove, recomputeCursor],
  )

  useEffect(() => {
    recomputeCursor()
  }, [demVersion, recomputeCursor])

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      selectTool.onPointerMove(e)
      emitCursor(e)
    },
    [selectTool, emitCursor],
  )

  const onPointerLeave = useCallback(() => {
    if (cursorRaf.current) {
      cancelAnimationFrame(cursorRaf.current)
      cursorRaf.current = 0
    }
    onCursorMove?.(null)
  }, [onCursorMove])

  useEffect(
    () => () => {
      if (cursorRaf.current) cancelAnimationFrame(cursorRaf.current)
    },
    [],
  )

  const onDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      const el = containerRef.current
      if (!el) return
      const r = el.getBoundingClientRect()
      const vp = viewportFromViewState(r.width, r.height, viewStateRef.current)
      const px: [number, number] = [e.clientX - r.left, e.clientY - r.top]
      if (clusterMode) {
        const marker = slotClusterIndex.pickClusterAt(px, vp, viewStateRef.current.zoom)
        if (marker) drillIntoCluster({ x: marker.x, y: marker.y })
        return
      }
      const id = slotSpatialIndex.pickNearest(px, vp)
      if (id) onEntityActivate?.(id)
    },
    [onEntityActivate, clusterMode, drillIntoCluster],
  )

  const flyTo = useCallback(
    (world: { x: number; y: number }) => flyToInternal([world.x, world.y]),
    [flyToInternal],
  )

  useEffect(() => onReady?.({ flyTo }), [onReady, flyTo])

  const onDragOver = useCallback((e: React.DragEvent) => {
    if (!e.dataTransfer.types.includes(ASSET_DND_MIME)) return
    e.preventDefault()
    e.dataTransfer.dropEffect = 'copy'
  }, [])

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      const raw = e.dataTransfer.getData(ASSET_DND_MIME)
      const el = containerRef.current
      if (!raw || !el) return
      e.preventDefault()
      let payload: AssetDropPayload
      try {
        payload = JSON.parse(raw) as AssetDropPayload
      } catch {
        return
      }
      const rect = el.getBoundingClientRect()
      const vp = viewportFromViewState(rect.width, rect.height, viewStateRef.current)
      const [x, y] = vp.unproject([e.clientX - rect.left, e.clientY - rect.top])
      onAssetDrop?.(payload, { x, y })
    },
    [onAssetDrop],
  )

  // flyTo closes over viewStateRef but only reads it when invoked (Space / outliner), not during render.
  // eslint-disable-next-line react-hooks/refs -- createMapContextValue stores the callback; does not call it
  const ctx = useMemo(() => createMapContextValue(terrainDef, flyTo), [terrainDef, flyTo])

  useEffect(() => {
    const container = containerRef.current
    const canvas = canvasRef.current
    if (!container || !canvas) return

    let engine: RenderEngine | null = null // I2
    let disposed = false // I4
    let raf = 0
    let lastDpr = window.devicePixelRatio
    let lastHud = 0
    let frames = 0

    const applySize = () => {
      const rect = container.getBoundingClientRect()
      const dpr = window.devicePixelRatio
      lastDpr = dpr
      const [dw, dh] = deviceSize(rect.width, rect.height, dpr)
      canvas.width = dw
      canvas.height = dh
      engine?.resize(rect.width, rect.height, dpr)
    }

    // The backing store must be sized BEFORE create (the engine reads canvas.width/height).
    {
      const rect = container.getBoundingClientRect()
      const [dw, dh] = deviceSize(rect.width, rect.height, window.devicePixelRatio)
      canvas.width = dw
      canvas.height = dh
    }

    void createEngine(canvas, forceWebgl)
      .then((created) => {
        if (disposed) {
          created.free() // I4: the effect died while create was in flight
          return
        }
        engine = created
        engineRef.current = created
        setBackend(created.backend())
        applySize()
        // T-151.11.2 (X-02): engine-side clamp must match the mounted terrain (create-time
        // default is Everon); TS clampViewState stays as the synchronous mirror/backstop.
        created.set_camera_bounds(0, 0, terrainDef.width, terrainDef.height)
        const vs = viewStateRef.current
        created.set_view(vs.target[0], vs.target[1], vs.zoom)
        created.hide_calibration() // W1: no calibration scene in the editor (L1)
        // T-151.11.2 (P-06): prod runs damage-driven (idle frames skip acquire/encode/submit —
        // the T-151.8 contract); DEV keeps continuous submits so the fps HUD stays truthful.
        created.set_continuous_render(import.meta.env.DEV)
        controllerRef.current = new WgpuBasemapController(created, terrainDef, {
          onProgress: (f) => onProgressRef.current?.(f),
          onDegraded: (v) => onDegradedRef.current?.(v),
        })
        worldControllerRef.current = new WgpuWorldController(created, terrainDef)
        forestControllerRef.current = new WgpuForestMassController(created, terrainDef)
        slotsControllerRef.current = new WgpuSlotsController(created)
        // T-152.1: cartographic text store (declutter in Rust; empty until .7+ feed labels).
        textLabelStoreRef.current?.free()
        textLabelStoreRef.current = createTextLabelStore()
        setReady(true) // fires the basemap + world + slots hook effects
        notifyCameraMoved()
        const loop = (now: number) => {
          if (disposed || !engine) return // I5
          if (window.devicePixelRatio !== lastDpr) applySize()
          try {
            engine.render()
          } catch (err) {
            setError(String(err)) // I6
            return
          }
          frames += 1
          // T-151.11.2 (P-05): HUD state churn is DEV-only — prod renders no debug panel.
          if (import.meta.env.DEV && now - lastHud > HUD_INTERVAL_MS) {
            if (lastHud > 0) setFps(Math.round((frames * 1000) / (now - lastHud)))
            lastHud = now
            frames = 0
            try {
              setBasemapMode(String((JSON.parse(engine.stats()) as { basemap_mode: string }).basemap_mode))
            } catch {
              /* stats parse is best-effort HUD only */
            }
          }
          raf = requestAnimationFrame(loop)
        }
        raf = requestAnimationFrame(loop)
      })
      .catch((err: unknown) => {
        if (!disposed) setError(String(err)) // I6
      })

    const ro = new ResizeObserver(() => {
      if (!disposed && engine) applySize()
    })
    ro.observe(container)

    return () => {
      disposed = true // I4
      cancelAnimationFrame(raf) // I5 — before free
      ro.disconnect()
      controllerRef.current?.dispose()
      controllerRef.current = null
      worldControllerRef.current?.dispose() // frees the WorldResidency wasm handle (once)
      worldControllerRef.current = null
      forestControllerRef.current?.dispose()
      forestControllerRef.current = null
      slotsControllerRef.current?.dispose()
      slotsControllerRef.current = null
      textLabelStoreRef.current?.free()
      textLabelStoreRef.current = null
      engineRef.current = null
      setReady(false)
      engine?.free() // I4 — exactly once
      engine = null
    }
  }, [forceWebgl, canvasKey, terrainDef, notifyCameraMoved])

  // T-151.7.2 hotfix: wheel lives OUTSIDE the engine-create effect so it cannot be torn down
  // by unrelated dep churn, and does NOT call resize every tick (that raced render + threw).
  useEffect(() => {
    if (!ready) return
    const container = containerRef.current
    if (!container) return

    const onWheel = (ev: WheelEvent) => {
      const eng = engineRef.current
      if (!eng) return
      ev.preventDefault()
      const rect = container.getBoundingClientRect()
      // CSS origin = container (same as pan/cursor). No resize here — ResizeObserver owns size.
      eng.zoom_at(
        -ev.deltaY * WHEEL_ZOOM_PER_PX,
        ev.clientX - rect.left,
        ev.clientY - rect.top,
      )
      // Immediate mirror so pick/pan see the new zoom before React paints.
      const next = clampViewState(viewStateFromEngine(eng))
      if (
        next.target[0] !== eng.target_x ||
        next.target[1] !== eng.target_y ||
        next.zoom !== eng.zoom
      ) {
        eng.set_view(next.target[0], next.target[1], next.zoom)
      }
      viewStateRef.current = next
      setViewState(next)
      // T-151.11.6: an in-flight RMB-pan is RE-ANCHORED to the post-zoom camera (frozen
      // viewport + start target + start px all refreshed) so zooming mid-pan keeps panning.
      // The predecessor here aborted the gesture, forcing a re-press (operator bug).
      try {
        rebasePanRef.current?.([ev.clientX - rect.left, ev.clientY - rect.top])
      } catch {
        /* ignore */
      }
      controllerRef.current?.onCameraMoved()
      worldControllerRef.current?.onCameraMoved()
      forestControllerRef.current?.onCameraMoved()
      slotsControllerRef.current?.onCameraMoved()
    }

    // Capture phase so we see the event even if a child stops bubble; passive:false for preventDefault.
    container.addEventListener('wheel', onWheel, { passive: false, capture: true })
    return () => container.removeEventListener('wheel', onWheel, { capture: true })
  }, [ready, clampViewState])

  const retryWebgl = useCallback(() => {
    setError(null)
    setBackend('initializing…')
    setForceWebgl(true)
    setCanvasKey((k) => k + 1) // I7 — fresh canvas element
  }, [])

  return (
    <MapContextProvider value={ctx}>
      <div
        ref={containerRef}
        className={className}
        // Host supplies position (e.g. absolute inset-0). Canvas is absolute inset:0 against
        // that positioned box — wheel/pan/pick all use container.getBoundingClientRect() (B3).
        style={{ background: '#0b0f14', cursor: 'crosshair' }}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onPointerDown={selectTool.onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={selectTool.onPointerUp}
        onPointerLeave={onPointerLeave}
        onDoubleClick={onDoubleClick}
        onContextMenu={selectTool.onContextMenu}
      >
        <canvas
          key={canvasKey}
          ref={canvasRef}
          style={{ position: 'absolute', inset: 0, width: '100%', height: '100%' }}
        />
        {/* T-151.11.2 (P-05): debug readouts are DEV-only; the error banner is its own
            always-rendered element — init failures must surface in production (I6). */}
        {import.meta.env.DEV && (
          <div style={PANEL}>
            <div style={{ fontWeight: 600, letterSpacing: 0.3 }}>wgpu · {backend}</div>
            <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 18, margin: '2px 0 4px' }}>
              {fps} FPS
            </div>
            <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 12, opacity: 0.9 }}>
              basemap: {basemapMode}
            </div>
          </div>
        )}
        {error !== null && (
          <div style={{ ...PANEL, top: import.meta.env.DEV ? 132 : 60 }}>
            <div style={{ color: '#ff9c9c', maxWidth: 420, whiteSpace: 'pre-wrap' }}>{error}</div>
            <button onClick={retryWebgl} style={BTN}>
              Retry with WebGL2 (fresh canvas)
            </button>
          </div>
        )}
      </div>
    </MapContextProvider>
  )
}

const PANEL: CSSProperties = {
  position: 'absolute',
  top: 60,
  left: '50%',
  transform: 'translateX(-50%)',
  padding: '10px 14px',
  borderRadius: 10,
  background: 'rgba(14,20,28,0.78)',
  backdropFilter: 'blur(8px)',
  border: '1px solid rgba(140,198,255,0.18)',
  color: '#e6ebf2',
  font: '13px/1.3 ui-sans-serif, system-ui, sans-serif',
  maxWidth: 520,
  textAlign: 'center',
  pointerEvents: 'none',
}

const BTN: CSSProperties = {
  padding: '5px 10px',
  marginTop: 6,
  borderRadius: 7,
  cursor: 'pointer',
  color: '#cdd7e4',
  background: 'rgba(140,198,255,0.12)',
  border: '1px solid rgba(140,198,255,0.25)',
  fontWeight: 600,
  fontSize: 12,
  pointerEvents: 'auto',
}
