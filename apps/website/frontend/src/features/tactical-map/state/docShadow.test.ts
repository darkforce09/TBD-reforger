import { describe, it, expect } from 'vitest'
import {
  createMissionDoc,
  seedDefaultLayer,
  addEditorLayer,
  addSlot,
  moveEntities,
  updateSlotPosition,
  updateSlot,
  moveSlotToLayer,
  pasteSlots,
  removeEntities,
} from './ydoc'
import { createDocShadow, checkDocShadowParity } from './docShadow'

// T-145 Phase 3.2 Stage 1 — the shadow yrs doc, fed the Y.Doc update stream, must stay in SoA parity
// across real editor mutators (the live gate's mechanism, headless). docCore.parity covers the wasm
// core over synthetic ops; this covers the update-stream wiring + the checkDocShadowParity diagnostic.

describe('docShadow — live yrs↔Y.Doc parity via the update stream', () => {
  it('stays in parity across real mutators (add/move/update/relayer/paste/remove)', () => {
    const md = createMissionDoc()
    const shadow = createDocShadow()
    md.doc.on('update', (u: Uint8Array) => shadow.apply_update(u))

    seedDefaultLayer(md)
    const l2 = addEditorLayer(md, { name: 'Bravo' })
    const s1 = addSlot(md, { x: 100.5, y: 200.25 }, { role: 'Squad Leader', tag: 'CMD' })
    addSlot(md, { x: 1500.75, y: 900.125 }, { role: 'Rifleman' })
    const s3 = addSlot(md, { x: 3000, y: 4000 }, { role: 'Medic', tag: 'MED', layerId: l2 })
    const s2 = addSlot(md, { x: 640, y: 640 }, { role: 'Grenadier' })
    moveEntities(md, [s1, s3], { x: 12.5, y: -7.25 })
    updateSlotPosition(md, s3, { rotation: 270, x: 3100.5 })
    updateSlot(md, s1, { stance: 'prone', tag: 'HQ' })
    moveSlotToLayer(md, s1, l2)

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

    expect(checkDocShadowParity(md, shadow)).toBeNull()
    shadow.free()
  })

  it('detects divergence when the shadow misses an update', () => {
    const md = createMissionDoc()
    const shadow = createDocShadow()
    let live = true
    md.doc.on('update', (u: Uint8Array) => {
      if (live) shadow.apply_update(u)
    })

    seedDefaultLayer(md)
    addSlot(md, { x: 10, y: 20 }, { role: 'Rifleman' })
    expect(checkDocShadowParity(md, shadow)).toBeNull()

    live = false // shadow stops receiving updates
    addSlot(md, { x: 30, y: 40 }, { role: 'Medic' })
    expect(checkDocShadowParity(md, shadow)).not.toBeNull() // count mismatch caught

    shadow.free()
  })
})
