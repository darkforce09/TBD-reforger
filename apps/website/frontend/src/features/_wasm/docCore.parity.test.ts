import { describe, it, expect } from 'vitest'
import * as Y from 'yjs'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
// The bundler-target pkg keeps `memory` on the internal *_bg.wasm module (the top-level index
// re-exports only the classes/fns). ESM dedup makes this the SAME instance MissionDoc mutates, so a
// Float32Array over its buffer aliases live wasm memory — the criterion-6 zero-copy mechanism.
import * as wasmBg from '@/wasm/pkg/map_engine_wasm_bg.wasm'
import {
  createMissionDoc,
  seedDefaultLayer,
  addEditorLayer,
  addSlot,
  moveEntities,
  updateSlotPosition,
  pasteSlots,
  removeEntities,
} from '@/features/tactical-map/state/ydoc'

// Phase 3.0 spike (plan §9.1) — the yrs-backed wasm MissionDoc must match the JS Yjs Y.Doc:
//   (2) applying a Yjs-wire update byte-stream materializes the identical slots,
//   (3) the yrs update stream round-trips (encode → apply → same SoA; re-encode stable),
//   (4) the yrs UndoManager reproduces the Y.UndoManager sequence.
// Class S: set-equality keyed by slot id. Numeric columns cross the f32 store boundary, so a JS f64
// compares as `Math.fround(js) === col` (Class R). Undo uses captureTimeout 0 on both sides so every
// op is one step.

const NONE = 0xffffffff // SlotSoa NONE_IDX (u32::MAX): no tag / unfiled layer.
const STANCE = ['stand', 'crouch', 'prone'] as const

interface SlotView {
  x: number
  y: number
  z: number
  rotation: number
  role: string
  tag: string | null
  stance: string
  squadId: string
  layer: string | null
}

/** Materialize the wasm SoA into an id-keyed map (the join key; row order is arbitrary). */
function readWasm(doc: wasm.MissionDoc): Map<string, SlotView> {
  doc.refresh()
  const ids = doc.slot_ids()
  const xs = doc.slot_xs()
  const ys = doc.slot_ys()
  const zs = doc.slot_zs()
  const rot = doc.slot_rotations()
  const stance = doc.slot_stance()
  const roleIdx = doc.slot_role_idx()
  const tagIdx = doc.slot_tag_idx()
  const squadIdx = doc.slot_squad_idx()
  const layerIdx = doc.slot_layer_idx()
  const roles = doc.roles()
  const tags = doc.tags()
  const squads = doc.squads()
  const layers = doc.layers()
  const out = new Map<string, SlotView>()
  for (let i = 0; i < ids.length; i++) {
    out.set(ids[i], {
      x: xs[i],
      y: ys[i],
      z: zs[i],
      rotation: rot[i],
      role: roles[roleIdx[i]],
      tag: tagIdx[i] === NONE ? null : tags[tagIdx[i]],
      stance: STANCE[stance[i]],
      squadId: squads[squadIdx[i]],
      layer: layerIdx[i] === NONE ? null : layers[layerIdx[i]],
    })
  }
  return out
}

/** Materialize a JS Y.Doc's `slots` map into the same id-keyed shape (the parity oracle). */
function readYDoc(doc: Y.Doc): Map<string, SlotView> {
  const slots = doc.getMap('slots') as Y.Map<Y.Map<unknown>>
  const layers = doc.getMap('editorLayers') as Y.Map<Y.Map<unknown>>
  const layerOf = new Map<string, string>()
  layers.forEach((layer, lid) => {
    for (const sid of (layer.get('entityIds') as string[] | undefined) ?? []) {
      if (!layerOf.has(sid)) layerOf.set(sid, lid)
    }
  })
  const out = new Map<string, SlotView>()
  slots.forEach((slot, id) => {
    const p = (slot.get('position') as Record<string, number>) ?? {}
    out.set(id, {
      x: p.x ?? 0,
      y: p.y ?? 0,
      z: p.z ?? 0,
      rotation: p.rotation ?? 0,
      role: (slot.get('role') as string) ?? '',
      tag: (slot.get('tag') as string | undefined) ?? null,
      stance: (slot.get('stance') as string) ?? 'stand',
      squadId: (slot.get('squadId') as string) ?? '',
      layer: layerOf.get(id) ?? null,
    })
  })
  return out
}

/** Assert two id-keyed slot maps are set-equal (Class S) with f32-boundary numerics (Class R). */
function expectSlotsEqual(
  got: Map<string, SlotView>,
  want: Map<string, SlotView>,
  opts: { checkLayer?: boolean; checkTag?: boolean } = {},
) {
  expect([...got.keys()].sort()).toEqual([...want.keys()].sort())
  for (const [id, w] of want) {
    const g = got.get(id)
    if (!g) throw new Error(`wasm missing slot ${id}`)
    expect(Math.fround(w.x), `x ${id}`).toBe(g.x)
    expect(Math.fround(w.y), `y ${id}`).toBe(g.y)
    expect(Math.fround(w.z), `z ${id}`).toBe(g.z)
    expect(Math.fround(w.rotation), `rot ${id}`).toBe(g.rotation)
    expect(g.role, `role ${id}`).toBe(w.role)
    expect(g.stance, `stance ${id}`).toBe(w.stance)
    expect(g.squadId, `squad ${id}`).toBe(w.squadId)
    if (opts.checkTag !== false) expect(g.tag, `tag ${id}`).toBe(w.tag)
    if (opts.checkLayer !== false) expect(g.layer, `layer ${id}`).toBe(w.layer)
  }
}

describe('MissionDoc — criterion 2: Yjs-wire update apply → identical SoA', () => {
  it('materializes a mission authored through the real ydoc.ts actions', () => {
    const md = createMissionDoc()
    seedDefaultLayer(md)
    const l2 = addEditorLayer(md, { name: 'Bravo' })

    const s1 = addSlot(md, { x: 100.5, y: 200.25 }, { role: 'Squad Leader', tag: 'CMD' })
    addSlot(md, { x: 1500.75, y: 900.125 }, { role: 'Rifleman' })
    const s3 = addSlot(md, { x: 3000, y: 4000 }, { role: 'Medic', tag: 'MED', layerId: l2 })
    const s2 = addSlot(md, { x: 640, y: 640 }, { role: 'Grenadier' })

    // Exercise bulk move, a numeric transform edit, a paste, and a delete.
    moveEntities(md, [s1, s3], { x: 12.5, y: -7.25 })
    updateSlotPosition(md, s3, { rotation: 270, x: 3100.5 })
    const s1Slot = md.entities.slots.get(s1)
    if (!s1Slot) throw new Error('s1 slot missing')
    const sq = s1Slot.get('squadId') as string
    pasteSlots(
      md,
      [
        {
          role: 'AR',
          stance: 'crouch',
          position: { x: 500, y: 500, z: 0, rotation: 45 },
          squadId: sq,
        },
      ],
      { anchorAt: { x: 800, y: 800 } },
    )
    removeEntities(md, 'slots', [s2])

    const update = Y.encodeStateAsUpdate(md.doc)
    const doc = new wasm.MissionDoc()
    doc.apply_update(update)

    expectSlotsEqual(readWasm(doc), readYDoc(md.doc))
    // Sanity: the doc is non-trivial and s2 was really removed.
    expect(doc.slot_len).toBe(4)
    expect(readWasm(doc).has(s2)).toBe(false)
  })
})

describe('MissionDoc — criterion 3: yrs update-stream round-trip', () => {
  it('encode → apply into a fresh doc → identical SoA; re-encode is byte-stable', () => {
    const src = new wasm.MissionDoc()
    src.add_slot('s1', 'sq1', 'lyr', 0, 'Rifleman', undefined, undefined, 1.5, 2.25, 0, 0)
    src.add_slot('s2', 'sq1', 'lyr', 1, 'Medic', undefined, undefined, 5, 6, 7, 90)
    src.add_slot('s3', 'sq2', 'lyr', 0, 'AR', undefined, undefined, 100.75, 200.5, 0, 180)

    const bytes = src.encode_state()
    const dst = new wasm.MissionDoc()
    dst.apply_update(bytes)
    expectSlotsEqual(readWasm(dst), readWasm(src), { checkLayer: false })

    // Deterministic encode (fixed client id): re-encoding the same doc is byte-identical.
    expect(Array.from(src.encode_state())).toEqual(Array.from(bytes))
  })
})

describe('MissionDoc — criterion 4: UndoManager parity vs Y.UndoManager', () => {
  it('reproduces the undo/redo sequence step-for-step (captureTimeout 0)', () => {
    // Identical minimal op script on a bare Y.Doc + Y.UndoManager and the wasm MissionDoc.
    const script: [string, string, string, number, number][] = [
      ['s1', 'sq1', 'Rifleman', 0, 0],
      ['s2', 'sq1', 'Medic', 100.5, 200.25],
      ['s3', 'sq2', 'AR', 300, 400.75],
    ]

    const jsDoc = new Y.Doc()
    const jsSlots = jsDoc.getMap('slots') as Y.Map<Y.Map<unknown>>
    const um = new Y.UndoManager(jsSlots, { captureTimeout: 0 })
    const wDoc = new wasm.MissionDoc()

    for (const [id, sq, role, x, y] of script) {
      jsDoc.transact(() => {
        const ym = new Y.Map()
        ym.set('id', id)
        ym.set('squadId', sq)
        ym.set('role', role)
        ym.set('stance', 'stand')
        ym.set('position', { x, y, z: 0, rotation: 0 })
        jsSlots.set(id, ym)
      })
      wDoc.add_slot(id, sq, 'lyr', 0, role, undefined, undefined, x, y, 0, 0)
    }

    const cmp = () =>
      expectSlotsEqual(readWasm(wDoc), readYDoc(jsDoc), { checkLayer: false, checkTag: false })
    cmp() // all three present

    um.undo()
    wDoc.undo() // removes s3 on both
    cmp()
    um.undo()
    wDoc.undo() // removes s2
    cmp()
    um.redo()
    wDoc.redo() // restores s2
    cmp()

    expect([...readWasm(wDoc).keys()].sort()).toEqual(['s1', 's2'])
  })
})

describe('MissionDoc — zero-copy Float32Array view (criterion 6 mechanism, headless)', () => {
  it('a view onto slot_xs_ptr aliases wasm memory and survives a grow via rebuild', () => {
    const doc = new wasm.MissionDoc()
    for (let i = 0; i < 16; i++)
      doc.add_slot(
        `z${i}`,
        'sq',
        'lyr',
        i,
        'Rifleman',
        undefined,
        undefined,
        i * 10 + 0.5,
        i * 20,
        0,
        0,
      )
    doc.refresh()
    const n = doc.slot_len

    const view1 = new Float32Array(wasmBg.memory.buffer, doc.slot_xs_ptr, n)
    // Zero-copy: the view equals the copy getter, element for element.
    expect(Array.from(view1)).toEqual(Array.from(doc.slot_xs()))

    // Force linear-memory growth with a large second doc; the old ArrayBuffer detaches.
    const big = new wasm.MissionDoc()
    for (let i = 0; i < 40000; i++)
      big.add_slot(`b${i}`, 'sq', 'lyr', i, 'R', undefined, undefined, i, i, 0, 0)
    big.refresh()

    // Rebuild the view over the current buffer at the (stable) ptr — the documented "re-materialize
    // views after any allocation" rule. Data is intact.
    const view2 = new Float32Array(wasmBg.memory.buffer, doc.slot_xs_ptr, n)
    expect(Array.from(view2)).toEqual(Array.from(doc.slot_xs()))
    // Row order is arbitrary (yrs map iteration), so assert a known value is present, not its index.
    expect(Array.from(view2)).toContain(Math.fround(0.5))
  })
})
