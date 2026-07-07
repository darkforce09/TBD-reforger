import { describe, it, expect } from 'vitest'
import * as Y from 'yjs'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import {
  createMissionDoc,
  seedDefaultLayer,
  addEditorLayer,
  renameEditorLayer,
  reparentEditorLayer,
  moveSlotToLayer,
  removeEditorLayer,
  addSlot,
  addFaction,
  addSquad,
  pasteSlots,
  updateSlot,
  updateSlotPosition,
  moveEntities,
  removeEntities,
  setTitle,
  updateEnvironment,
  applyMissionRowMeta,
  seedMeta,
  hydrateMissionDoc,
} from '@/features/tactical-map/state/ydoc'
import { docToSnapshot, snapshotFromShadow } from '@/features/tactical-map'
import { getTerrain } from '@/features/tactical-map/coords/terrains'
import type { ClipboardSlot, ID, Slot } from '@/features/tactical-map/state/schema'
import type { MissionDoc } from '@/features/tactical-map/state/ydoc'

const ZERO_POS: Slot['position'] = { x: 0, y: 0, z: 0, rotation: 0 }

/** Snapshot an existing slot into a serializable ClipboardSlot (Ctrl+C shape) for paste tests. */
function toClip(md: MissionDoc, id: ID): ClipboardSlot {
  const slot = md.entities.slots.get(id)
  return {
    role: (slot?.get('role') ?? '') as string,
    tag: slot?.get('tag') as string | undefined,
    assetId: slot?.get('assetId') as string | undefined,
    stance: (slot?.get('stance') ?? 'stand') as Slot['stance'],
    position: (slot?.get('position') ?? ZERO_POS) as Slot['position'],
    squadId: (slot?.get('squadId') ?? '') as ID,
  }
}

// T-145 Phase 3.2 batch 1 — differential parity for the Rust slot-lifecycle mutators. Build a base
// via the real ydoc.ts (Yjs) mutators, snapshot the base into a fresh yrs doc (so it holds the same
// ids), then run the SAME op on BOTH — the Yjs mutator and its Rust twin (fed the SAME id) — and
// assert snapshotFromShadow(yrs) deep-equals docToSnapshot(yjs). Ids match (base-synced), so the whole
// MapSnapshot compares — real byte-parity per mutator. The mutators operate on existing ids (no
// minting), so this is clean; add/paste (which mint) land in batch 2.

const T = getTerrain('everon') // 12800² — the default terrain a fresh doc opens with

/** A fresh yrs doc holding the same base state (+ ids) as `md`, then MUTATED DIRECTLY (not synced —
 *  so a subsequent Yjs op on `md` does not leak in via the update stream). */
function baseSync(md: ReturnType<typeof createMissionDoc>): wasm.MissionDoc {
  const yrs = new wasm.MissionDoc()
  yrs.apply_update(Y.encodeStateAsUpdate(md.doc))
  return yrs
}

/** A base doc with 3 slots across two layers + a squad, for the update/move/remove ops. */
function baseDoc() {
  const md = createMissionDoc()
  seedDefaultLayer(md)
  const defaultLayer = [...md.entities.editorLayers.keys()][0]
  const l2 = addEditorLayer(md, { name: 'Bravo' })
  const s1 = addSlot(md, { x: 100.5, y: 200.25 }, { role: 'Squad Leader', tag: 'CMD' })
  const s2 = addSlot(md, { x: 1500.75, y: 900.125 }, { role: 'Rifleman' })
  const s3 = addSlot(md, { x: 3000, y: 4000 }, { role: 'Medic', tag: 'MED', layerId: l2 })
  return { md, defaultLayer, l2, s1, s2, s3 }
}

describe('Rust mutator parity vs ydoc.ts (batch 1: slot lifecycle)', () => {
  it('update_slot (role/tag/stance)', () => {
    const { md, s1 } = baseDoc()
    const yrs = baseSync(md)
    updateSlot(md, s1, { role: 'Grenadier', tag: 'AR', stance: 'prone' })
    yrs.update_slot(s1, 'Grenadier', 'AR', 'prone')
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('update_slot_position (clamp x/y, normalize rotation, z-policy)', () => {
    const { md, s1 } = baseDoc()
    const yrs = baseSync(md)
    updateSlotPosition(md, s1, { x: 3100.5, rotation: 270 })
    yrs.update_slot_position(s1, 3100.5, undefined, undefined, 270, T.width, T.height)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('update_slot_position clamps out-of-bounds x/y to terrain', () => {
    const { md, s1 } = baseDoc()
    const yrs = baseSync(md)
    updateSlotPosition(md, s1, { x: -500, y: 999999 })
    yrs.update_slot_position(s1, -500, 999999, undefined, undefined, T.width, T.height)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('move_entities (shared delta over several slots)', () => {
    const { md, s1, s3 } = baseDoc()
    const yrs = baseSync(md)
    moveEntities(md, [s1, s3], { x: 12.5, y: -7.25 })
    // zs = JS-sampled DEM z at the new positions; DEM is off in vitest → zeros (byte-parity).
    yrs.move_entities([s1, s3], 12.5, -7.25, Float64Array.from([0, 0]))
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('remove_slots (cascade: squad.slotIds + layer.entityIds detach)', () => {
    const { md, s1, s2 } = baseDoc()
    const yrs = baseSync(md)
    removeEntities(md, 'slots', [s1, s2])
    yrs.remove_slots([s1, s2])
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })
})

describe('Rust mutator parity vs ydoc.ts (batch 2: editor layers)', () => {
  it('add_editor_layer (root + nested; JS mints the id)', () => {
    const { md, l2 } = baseDoc()
    const yrs = baseSync(md)
    const root = addEditorLayer(md, { name: 'Delta' })
    yrs.add_editor_layer(root, 'Delta', undefined)
    const nested = addEditorLayer(md, { name: 'Echo', parentId: l2 })
    yrs.add_editor_layer(nested, 'Echo', l2)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('rename_editor_layer', () => {
    const { md, l2 } = baseDoc()
    const yrs = baseSync(md)
    renameEditorLayer(md, l2, 'Bravo Renamed')
    yrs.rename_editor_layer(l2, 'Bravo Renamed')
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('reparent_editor_layer + cycle guard', () => {
    const { md, defaultLayer, l2 } = baseDoc()
    const yrs = baseSync(md)
    reparentEditorLayer(md, l2, defaultLayer) // valid: l2 under default
    yrs.reparent_editor_layer(l2, defaultLayer)
    reparentEditorLayer(md, defaultLayer, l2) // cycle: default under its own child → rejected
    yrs.reparent_editor_layer(defaultLayer, l2)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('move_slot_to_layer (detach everywhere + append to target)', () => {
    const { md, l2, s1 } = baseDoc()
    const yrs = baseSync(md)
    moveSlotToLayer(md, s1, l2)
    yrs.move_slot_to_layer(s1, l2)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })
})

describe('Rust mutator parity vs ydoc.ts (batch 3a: entity creation)', () => {
  const ASSET = '{6A3A3F62}Prefabs/Characters/Factions/BLUFOR/US/Character_US_Rifleman.et'

  it('add_slot on an empty doc (ensureDefault faction+squad+layer chain)', () => {
    // Yjs addSlot on an empty doc mints faction + squad + layer + slot in ONE transaction. Read the
    // four ids back, then replay the SAME graph on a fresh yrs doc via the create primitives (the
    // ensureDefault* string constants are the JS-side contract; positions/index read back so DEM /
    // rotation stay honest). Separate Rust transactions build the same final state.
    const md = createMissionDoc()
    const slotId = addSlot(md, { x: 100.5, y: 200.25 })
    const factionId = [...md.entities.factions.keys()][0]
    const squadId = [...md.entities.squads.keys()][0]
    const layerId = [...md.entities.editorLayers.keys()][0]
    const slot = md.entities.slots.get(slotId)
    const pos = (slot?.get('position') ?? ZERO_POS) as Slot['position']
    const index = (slot?.get('index') ?? 0) as number

    const yrs = new wasm.MissionDoc()
    yrs.add_faction(factionId, 'BLUFOR', 'BLUFOR')
    yrs.add_squad(squadId, factionId, 'Test Squad', 'Test')
    yrs.add_editor_layer(layerId, 'Default Layer', undefined)
    // prettier-ignore
    yrs.add_slot(slotId, squadId, layerId, index, 'Rifleman', undefined, undefined, pos.x, pos.y, pos.z, pos.rotation)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('add_slot into an existing squad+layer (tag + assetId; index continues)', () => {
    const { md, l2 } = baseDoc()
    const squadId = [...md.entities.squads.keys()][0]
    const yrs = baseSync(md) // base BEFORE the new slot
    const slotId = addSlot(
      md,
      { x: 555.5, y: 666.25 },
      { squadId, layerId: l2, role: 'Grenadier', tag: 'AR', assetId: ASSET },
    )
    const slot = md.entities.slots.get(slotId)
    const pos = (slot?.get('position') ?? ZERO_POS) as Slot['position']
    const index = (slot?.get('index') ?? 0) as number // squad already holds s1/s2/s3 → 3
    // prettier-ignore
    yrs.add_slot(slotId, squadId, l2, index, 'Grenadier', 'AR', ASSET, pos.x, pos.y, pos.z, pos.rotation)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('add_faction (key + generated name)', () => {
    const { md } = baseDoc()
    const yrs = baseSync(md)
    const fid = addFaction(md)
    const f = md.entities.factions.get(fid)
    const key = (f?.get('key') ?? '') as string
    const name = (f?.get('name') ?? '') as string
    yrs.add_faction(fid, key, name)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('add_squad (no callsign; appends to faction.squadIds)', () => {
    const { md } = baseDoc()
    const factionId = [...md.entities.factions.keys()][0]
    const yrs = baseSync(md)
    const sqId = addSquad(md, factionId)
    const sq = md.entities.squads.get(sqId)
    const name = (sq?.get('name') ?? '') as string
    const callsign = sq?.get('callsign') as string | undefined
    yrs.add_squad(sqId, factionId, name, callsign)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })
})

describe('Rust mutator parity vs ydoc.ts (batch 3b: bulk paste)', () => {
  /** Replay ydoc.pasteSlots on the yrs twin: JS mints the ids (returned) + resolves squad/layer
   *  (the source squads still exist, target layer is `l2`); Rust re-derives centroid/clamp/index. */
  function replayPaste(
    yrs: wasm.MissionDoc,
    newIds: ID[],
    clip: ClipboardSlot[],
    layerId: ID,
    anchor: { x: number; y: number } | null,
  ): void {
    yrs.paste_slots(
      newIds,
      clip.map((c) => c.squadId),
      newIds.map(() => layerId),
      Float64Array.from(clip.map((c) => c.position.x)),
      Float64Array.from(clip.map((c) => c.position.y)),
      Float64Array.from(clip.map((c) => c.position.rotation)),
      Float64Array.from(clip.map(() => 0)), // zs: DEM off in vitest → zeros (byte-parity)
      clip.map((c) => c.role),
      clip.map((c) => c.tag ?? ''),
      clip.map((c) => c.assetId ?? ''),
      clip.map((c) => c.stance),
      anchor ? anchor.x : undefined,
      anchor ? anchor.y : undefined,
      T.width,
      T.height,
    )
  }

  it('paste_slots (anchor: centroid → cursor; index continues per squad)', () => {
    const { md, l2, s1, s2, s3 } = baseDoc()
    const clip = [s1, s2, s3].map((id) => toClip(md, id))
    const yrs = baseSync(md) // base BEFORE the paste
    const anchor = { x: 5000, y: 6000 }
    const newIds = pasteSlots(md, clip, { anchorAt: anchor, layerId: l2 })
    replayPaste(yrs, newIds, clip, l2, anchor)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('paste_slots (no anchor → +20 nudge)', () => {
    const { md, l2, s1, s2 } = baseDoc()
    const clip = [s1, s2].map((id) => toClip(md, id))
    const yrs = baseSync(md)
    const newIds = pasteSlots(md, clip, { anchorAt: null, layerId: l2 })
    replayPaste(yrs, newIds, clip, l2, null)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })
})

describe('Rust mutator parity vs ydoc.ts (batch 3c: layer removal + meta)', () => {
  it('remove_editor_layer (non-reseed: subtree slot cascade-deleted)', () => {
    const { md, l2 } = baseDoc() // l2 holds s3; default holds s1/s2; two layers → no reseed
    const yrs = baseSync(md)
    removeEditorLayer(md, l2)
    yrs.remove_editor_layer(l2, 'unused-reseed')
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('set_title', () => {
    const md = createMissionDoc()
    seedMeta(md, { id: 'm1', title: 'Old' })
    const yrs = baseSync(md)
    setTitle(md, 'New Title')
    yrs.set_title('New Title')
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('update_environment (merge patch onto existing env; string + numeric fields)', () => {
    const md = createMissionDoc()
    seedMeta(md, { id: 'm1', title: 'Op' })
    const yrs = baseSync(md)
    const patch = { weather: 'overcast' as const, viewDistance: 3200 }
    updateEnvironment(md, patch)
    yrs.update_environment(JSON.stringify(patch))
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('apply_row_meta (valid terrain + time/weather env merge)', () => {
    const md = createMissionDoc()
    seedMeta(md, { id: 'm1', title: 'Old' })
    const yrs = baseSync(md)
    applyMissionRowMeta(md, {
      title: 'Loaded',
      terrain: 'arland',
      time_of_day: '14:30',
      weather: 'overcast',
    })
    yrs.apply_row_meta('Loaded', 'arland', '14:30', 'overcast')
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('apply_row_meta (invalid terrain ignored; env untouched)', () => {
    const md = createMissionDoc()
    seedMeta(md, { id: 'm1', title: 'Old' })
    const yrs = baseSync(md)
    applyMissionRowMeta(md, { title: 'X', terrain: 'bogus' })
    yrs.apply_row_meta('X', 'bogus', undefined, undefined)
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('seed_meta (DEFAULT_META on an empty doc)', () => {
    const md = createMissionDoc()
    const yrs = new wasm.MissionDoc()
    seedMeta(md, { id: 'm1', title: 'Operation X' })
    yrs.seed_meta('m1', 'Operation X')
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })
})

describe('Rust mutator parity vs ydoc.ts (batch 3d: hydrate)', () => {
  // A rich lossless payload: environment + terrain + the full editor graph (factions/squads/slots
  // with tag/assetId/varied positions + nested folders) + top-level objectives/vehicles/markers +
  // a loadouts object. Exercises every dict the Rust loader touches.
  const losslessPayload = {
    environment: { time: '08:00', weather: 'overcast', viewDistance: 3200, thermals: true },
    map: { terrain: 'arland' },
    editor: {
      factions: [{ id: 'f1', key: 'BLUFOR', name: 'US Army', squadIds: ['sq1'] }],
      squads: [
        { id: 'sq1', factionId: 'f1', callsign: 'Alpha', name: 'Alpha 1-1', slotIds: ['s1', 's2'] },
      ],
      slots: [
        {
          id: 's1',
          squadId: 'sq1',
          index: 0,
          role: 'Squad Leader',
          tag: 'CMD',
          position: { x: 100.5, y: 200.25, z: 0, rotation: 90 },
          stance: 'stand',
          loadoutId: null,
        },
        {
          id: 's2',
          squadId: 'sq1',
          index: 1,
          role: 'Rifleman',
          assetId: '{GUID}Prefabs/x.et',
          position: { x: 300, y: 400, z: 12.5, rotation: 0 },
          stance: 'prone',
          loadoutId: null,
        },
      ],
      editorLayers: [
        { id: 'l1', name: 'Default Layer', parentId: null, entityIds: ['s1', 's2'] },
        { id: 'l2', name: 'Bravo', parentId: 'l1', entityIds: [] },
      ],
    },
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
        position: { x: 5, y: 6, z: 0, rotation: 0 },
        inventoryItemIds: [],
      },
    ],
    markers: [{ id: 'm1', kind: 'icon', points: [[1, 2]], color: '#fff' }],
    loadouts: { ld1: { id: 'ld1', containers: {}, weapons: {}, itemIds: [] } },
  }

  it('hydrate (lossless editor block loaded verbatim)', () => {
    const md = createMissionDoc()
    hydrateMissionDoc(md, losslessPayload)
    const yrs = new wasm.MissionDoc()
    yrs.hydrate(JSON.stringify(losslessPayload), 'unused-default')
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })

  it('hydrate (lossy orbat → JS-minted editor dicts reconstructed byte-for-byte)', () => {
    // The lossy orbat rebuild stays JS-side (it mints ids); the flip wrapper feeds its editor-shaped
    // output to the Rust loader. Prove that composite: JS hydrates from orbat, then the Rust loader
    // reproduces the JS-built graph.
    const md = createMissionDoc()
    hydrateMissionDoc(md, {
      orbat: [
        {
          faction: 'BLUFOR',
          callsign: 'A',
          squad: 'Alpha',
          slots: [{ role: 'SL', tag: 'CMD' }, { role: 'AR' }],
        },
        { faction: 'OPFOR', callsign: 'B', squad: 'Bravo', slots: [{ role: 'Rifleman' }] },
      ],
    })
    const snap = docToSnapshot(md)
    const editorPayload = {
      editor: {
        factions: Object.values(snap.factionsById),
        squads: Object.values(snap.squadsById),
        slots: Object.values(snap.slotsById),
        editorLayers: Object.values(snap.editorLayersById),
      },
    }
    const yrs = new wasm.MissionDoc()
    yrs.hydrate(JSON.stringify(editorPayload), 'unused-default')
    expect(snapshotFromShadow(yrs)).toEqual(snap)
    yrs.free()
  })

  it('hydrate (no layers in payload → reseed default with the JS-minted id)', () => {
    const md = createMissionDoc()
    const payload = {
      editor: {
        factions: [{ id: 'f1', key: 'BLUFOR', name: 'US', squadIds: [] }],
        squads: [],
        slots: [],
        editorLayers: [],
      },
    }
    hydrateMissionDoc(md, payload) // ensureDefaultLayer mints a layer
    const layerId = [...md.entities.editorLayers.keys()][0]
    const yrs = new wasm.MissionDoc()
    yrs.hydrate(JSON.stringify(payload), layerId) // Rust reseeds with the same id
    expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))
    yrs.free()
  })
})
