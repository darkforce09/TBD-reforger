// Phase 3.0.d spike harness (operator-verified) — the browser half of the Phase 3.0 gate:
//   criterion 6: a deck.gl layer reads a Float32Array view onto wasm linear memory (zero copy) and
//                sustains ≥ 60 fps panning at 500,000 slots (+ a 1,000,000 stress toggle), and
//   persistence: the yrs update stream round-trips through IndexedDB (Save → Reload).
// Not routed in the app nav; reachable only at /_spike/doc-core. See the on-screen checklist.

import { useCallback, useRef, useState, type CSSProperties } from 'react'
import DeckGL from '@deck.gl/react'
import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { ScatterplotLayer } from '@deck.gl/layers'
import { MissionDoc } from '@/wasm/pkg/map_engine_wasm'
// `memory` lives on the internal *_bg.wasm module (ESM-deduped to the same instance MissionDoc uses),
// so a Float32Array over its buffer aliases the live SoA — the zero-copy feed.
import * as wasmBg from '@/wasm/pkg/map_engine_wasm_bg.wasm'
import { useOrthographicView } from '@/features/tactical-map/view/useOrthographicView'
import { getTerrain } from '@/features/tactical-map/coords/terrains'
import type { MapViewState } from '@/features/tactical-map/types'
import { saveState, loadState, clearState } from './yrsIndexeddb'
import { useFps } from './useFps'

const TERRAIN = getTerrain('everon')
const PERSIST_ID = 'spike-doc'
const COUNTS = [100_000, 500_000, 1_000_000]

interface LayerData {
  n: number
  xy: Float32Array
  version: number
}

export default function DocCoreSpikePage() {
  const { view, viewState, onViewStateChange } = useOrthographicView(TERRAIN)
  const docRef = useRef<MissionDoc | null>(null)
  const [data, setData] = useState<LayerData | null>(null)
  const [count, setCount] = useState(500_000)
  const [busy, setBusy] = useState(false)
  const [status, setStatus] = useState('Pick a size, Generate, then pan/zoom and watch FPS.')
  const fps = useFps()

  // Rebuild the zero-copy view AFTER any (re)materialization — memory may have grown, detaching a
  // prior view; `version` re-keys the deck layer so it picks up the fresh buffer.
  const rebuildView = useCallback((doc: MissionDoc) => {
    doc.refresh()
    const n = doc.slot_len
    const xy = new Float32Array(wasmBg.memory.buffer, doc.slot_xy_ptr, n * 2)
    setData({ n, xy, version: Date.now() })
  }, [])

  const generate = useCallback(async () => {
    setBusy(true)
    setStatus(`Generating ${count.toLocaleString()} slots…`)
    await new Promise((r) => setTimeout(r, 15)) // let the status paint first
    const t0 = performance.now()
    const doc = new MissionDoc()
    doc.seed_random(count, TERRAIN.width, TERRAIN.height, Date.now())
    docRef.current = doc
    rebuildView(doc)
    setStatus(
      `Generated ${count.toLocaleString()} slots in ${Math.round(performance.now() - t0)} ms. Pan/zoom — FPS should hold ≥ 60.`,
    )
    setBusy(false)
  }, [count, rebuildView])

  const save = useCallback(async () => {
    if (!docRef.current) return
    setBusy(true)
    setStatus('Encoding + saving the yrs update stream to IndexedDB…')
    const bytes = docRef.current.encode_state()
    await saveState(PERSIST_ID, bytes)
    setStatus(`Saved ${(bytes.length / 1e6).toFixed(1)} MB to IndexedDB. Now try Reload.`)
    setBusy(false)
  }, [])

  const reload = useCallback(async () => {
    setBusy(true)
    setStatus('Loading from IndexedDB + applying into a fresh MissionDoc…')
    const bytes = await loadState(PERSIST_ID)
    if (!bytes) {
      setStatus('Nothing saved yet — Generate then Save first.')
      setBusy(false)
      return
    }
    const t0 = performance.now()
    const doc = new MissionDoc()
    doc.apply_update(bytes)
    docRef.current = doc
    rebuildView(doc)
    setStatus(
      `Reloaded ${doc.slot_len.toLocaleString()} slots from IndexedDB in ${Math.round(performance.now() - t0)} ms — should render identically.`,
    )
    setBusy(false)
  }, [rebuildView])

  const layers = data
    ? [
        new ScatterplotLayer({
          id: `slots-${data.version}`,
          coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
          data: { length: data.n, attributes: { getPosition: { value: data.xy, size: 2 } } },
          getRadius: 5,
          radiusUnits: 'pixels',
          radiusMinPixels: 1,
          radiusMaxPixels: 5,
          getFillColor: [140, 198, 255],
          stroked: false,
        }),
      ]
    : []

  return (
    <div style={{ position: 'fixed', inset: 0, background: '#0b0f14', color: '#e6ebf2' }}>
      <DeckGL
        views={view}
        viewState={viewState}
        onViewStateChange={(params) =>
          onViewStateChange({ viewState: params.viewState as MapViewState })
        }
        controller
        layers={layers}
      />
      <div style={PANEL}>
        <div style={{ fontWeight: 600, letterSpacing: 0.3 }}>T-145 · doc-core spike</div>
        <div style={{ fontVariantNumeric: 'tabular-nums', fontSize: 24, margin: '2px 0 8px' }}>
          {fps} FPS · {(data?.n ?? 0).toLocaleString()} slots
        </div>
        <div style={{ display: 'flex', gap: 6, marginBottom: 6 }}>
          {COUNTS.map((c) => (
            <button key={c} onClick={() => setCount(c)} style={BTN(count === c)} disabled={busy}>
              {c / 1000}k
            </button>
          ))}
        </div>
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          <button onClick={generate} disabled={busy} style={BTN(false)}>
            Generate
          </button>
          <button onClick={save} disabled={busy || !data} style={BTN(false)}>
            Save→IDB
          </button>
          <button onClick={reload} disabled={busy} style={BTN(false)}>
            Reload←IDB
          </button>
          <button onClick={() => void clearState(PERSIST_ID)} disabled={busy} style={BTN(false)}>
            Clear IDB
          </button>
        </div>
        <div style={{ marginTop: 8, maxWidth: 380, opacity: 0.82, lineHeight: 1.4 }}>{status}</div>
      </div>
    </div>
  )
}

const PANEL: CSSProperties = {
  position: 'absolute',
  top: 16,
  left: 16,
  padding: '12px 14px',
  borderRadius: 10,
  background: 'rgba(14,20,28,0.72)',
  backdropFilter: 'blur(8px)',
  border: '1px solid rgba(140,198,255,0.18)',
  font: '13px/1.3 ui-sans-serif, system-ui, sans-serif',
}

function BTN(active: boolean): CSSProperties {
  return {
    padding: '5px 10px',
    borderRadius: 7,
    cursor: 'pointer',
    color: active ? '#0b0f14' : '#cdd7e4',
    background: active ? '#8cc6ff' : 'rgba(140,198,255,0.12)',
    border: '1px solid rgba(140,198,255,0.25)',
    fontWeight: 600,
  }
}
