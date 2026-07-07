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
  setTitle,
  updateEnvironment,
  addFaction,
  addSquad,
  seedMeta,
  hydrateMissionDoc,
} from './ydoc'
import { docToSnapshot } from './bindings'
import { createDocShadow, checkDocShadowParity, snapshotFromShadow } from './docShadow'

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
    // Exercise the small maps too (meta + explicit faction/squad) — the whole-model gate (3.2.2).
    setTitle(md, 'Op Nightfall')
    updateEnvironment(md, { weather: 'overcast', time: '18:30' })
    const f2 = addFaction(md)
    addSquad(md, f2)

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

  it('stays in parity after a hydrate (objectives/vehicles/markers/loadouts)', () => {
    const md = createMissionDoc()
    const shadow = createDocShadow()
    md.doc.on('update', (u: Uint8Array) => shadow.apply_update(u))

    hydrateMissionDoc(md, {
      environment: { time: '06:00', weather: 'clear' },
      map: { terrain: 'everon' },
      objectives: [
        {
          id: 'o1',
          type: 'capture',
          factionId: 'f1',
          position: { x: 1, y: 2, z: 0 },
          radius: 50,
          triggers: [],
        },
      ],
      vehicles: [
        {
          id: 'v1',
          classname: 'car',
          factionId: 'f1',
          position: { x: 3, y: 4, z: 0, rotation: 90 },
          inventoryItemIds: [],
        },
      ],
      markers: [{ id: 'm1', kind: 'icon', points: [[5, 6]], color: '#ffffff' }],
      loadouts: { ld1: { id: 'ld1', containers: {}, weapons: {}, itemIds: [] } },
      editor: {
        factions: [{ id: 'f1', key: 'BLUFOR', name: 'BLUFOR', squadIds: ['sq1'] }],
        squads: [{ id: 'sq1', factionId: 'f1', name: 'Alpha', slotIds: ['s1'] }],
        slots: [
          {
            id: 's1',
            squadId: 'sq1',
            index: 0,
            role: 'Rifleman',
            position: { x: 10, y: 20, z: 0, rotation: 0 },
            stance: 'stand',
            loadoutId: null,
          },
        ],
        editorLayers: [{ id: 'l1', name: 'L', parentId: null, entityIds: ['s1'] }],
      },
    })

    expect(checkDocShadowParity(md, shadow)).toBeNull()
    shadow.free()
  })

  it('snapshotFromShadow reproduces the full MapSnapshot (deep-equals docToSnapshot)', () => {
    const md = createMissionDoc()
    const shadow = createDocShadow()
    md.doc.on('update', (u: Uint8Array) => shadow.apply_update(u))

    seedMeta(md, { id: 'm', title: 'Untitled Mission' })
    seedDefaultLayer(md)
    const l2 = addEditorLayer(md, { name: 'Bravo' })
    addSlot(
      md,
      { x: 100.5, y: 200.25 },
      { role: 'Squad Leader', tag: 'CMD', assetId: '{GUID}Rifleman.et' },
    )
    const s2 = addSlot(md, { x: 1500.75, y: 900.125 }, { role: 'Rifleman' })
    addSlot(md, { x: 3000, y: 4000 }, { role: 'Medic', tag: 'MED', layerId: l2 })
    updateSlot(md, s2, { stance: 'prone' })
    setTitle(md, 'Op Nightfall')
    updateEnvironment(md, { weather: 'overcast' })
    const f2 = addFaction(md)
    addSquad(md, f2)

    // The full snapshot from the shadow (small maps + exact-f64 slots) must equal docToSnapshot.
    expect(snapshotFromShadow(shadow)).toEqual(docToSnapshot(md))
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
