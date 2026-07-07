import { describe, it, expect } from 'vitest'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { createWasmMissionDoc, type ChangeOrigin } from './wasmDoc'

// T-145 Phase 3.2 flip F2 — the stable-shell contract that F3 depends on: attach/detach frees the
// wasm handle (StrictMode-safe), the origin-tagged change signal fires, and reads reflect the doc.

describe('WasmMissionDoc shell (F2)', () => {
  it('attach/detach lifecycle; detach is shell-idempotent', () => {
    const md = createWasmMissionDoc()
    expect(md.alive).toBe(false)
    expect(md.wasm).toBeNull()

    md.attach(new wasm.MissionDoc())
    expect(md.alive).toBe(true)
    expect(md.wasm).not.toBeNull()

    md.detach()
    expect(md.alive).toBe(false)
    md.detach() // must not double-free (wasm .free() is not idempotent)
    expect(md.alive).toBe(false)
  })

  it('notifyChange fires listeners with the origin; unsubscribe stops them; version bumps', () => {
    const md = createWasmMissionDoc()
    const seen: ChangeOrigin[] = []
    const unsub = md.subscribe((o) => seen.push(o))

    md.notifyChange('local')
    md.notifyChange('init')
    unsub()
    md.notifyChange('local') // after unsubscribe → not seen, but still bumps version

    expect(seen).toEqual(['local', 'init'])
    expect(md.changeVersion).toBe(3)
  })

  it('snapshot / encodeState / hasContent reflect the attached doc; empty when detached', () => {
    const md = createWasmMissionDoc()
    expect(md.hasContent()).toBe(false)
    expect(md.encodeState().length).toBe(0)
    expect(md.snapshot().slotsById).toEqual({})

    const handle = new wasm.MissionDoc()
    handle.add_faction('f1', 'BLUFOR', 'US')
    handle.add_squad('sq1', 'f1', 'Alpha', undefined)
    handle.add_editor_layer('l1', 'Default Layer', undefined)
    handle.add_slot('s1', 'sq1', 'l1', 0, 'Rifleman', undefined, undefined, 100.5, 200.25, 0, 0)
    md.attach(handle)

    expect(md.hasContent()).toBe(true)
    expect(md.encodeState().length).toBeGreaterThan(0)
    const snap = md.snapshot()
    expect(Object.keys(snap.slotsById)).toEqual(['s1'])
    expect(snap.slotsById['s1']?.position.x).toBe(100.5)
    expect(snap.factionsById['f1']?.name).toBe('US')

    md.detach()
    expect(md.snapshot().slotsById).toEqual({})
    expect(md.hasContent()).toBe(false)
  })

  it('undo/redo passthrough notifies local + bumps version', () => {
    const md = createWasmMissionDoc()
    const handle = new wasm.MissionDoc()
    md.attach(handle)
    const seen: ChangeOrigin[] = []
    md.subscribe((o) => seen.push(o))

    handle.add_editor_layer('l1', 'Alpha', undefined) // a LOCAL gesture (default origin)
    expect(md.canUndo()).toBe(true)
    const v0 = md.changeVersion

    expect(md.undo()).toBe(true)
    expect(md.canUndo()).toBe(false)
    expect(md.canRedo()).toBe(true)
    expect(md.changeVersion).toBe(v0 + 1)
    expect(seen).toEqual(['local'])

    md.detach()
  })
})
