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
  addSlot,
  updateSlot,
  updateSlotPosition,
  moveEntities,
  removeEntities,
} from '@/features/tactical-map/state/ydoc'
import { docToSnapshot, snapshotFromShadow } from '@/features/tactical-map'
import { getTerrain } from '@/features/tactical-map/coords/terrains'

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
    yrs.move_entities([s1, s3], 12.5, -7.25)
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
