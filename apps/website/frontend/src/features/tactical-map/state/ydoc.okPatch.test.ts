import { describe, it, expect, vi, beforeEach } from 'vitest'

// DEM mocked ready with a fixed elevation so terrainZ is non-zero — the O(k) store patch must carry
// the SAME z the wrapper passes to wasm.
const dem = vi.hoisted(() => ({ ready: true, elevation: 42.5 }))
vi.mock('../dem', () => ({
  sampleElevation: () => dem.elevation,
  isDemReady: () => dem.ready,
}))

import * as wasm from '@/wasm/pkg/map_engine_wasm'
import * as ydoc from './ydoc'
import { useMapStore, pickMapSnapshot } from './useMapStore'
import { snapshotFromShadow } from './docShadow'
import type { MissionDoc } from './ydoc'
import type { ClipboardSlot, ID } from './schema'

// T-145 Phase 3.2 flip F3.1 — the O(k) fast paths construct store patches JS-side instead of the whole
// snapshot. This is the correctness gate: after EVERY mutator the store read-mirror must byte-match the
// authoritative wasm doc (snapshotFromShadow). Because the compiler reads the store, store == wasm
// guarantees a byte-identical compiled payload whether a slot was placed in-session (O(k) patch) or
// loaded from a reload (whole snapshot). Any O(k) construction that diverges from the wasm shape
// (index accumulator, clamp/normalize, omit-empty tag, layer detach order) fails here.

function makeDoc(): MissionDoc {
  const md = ydoc.createMissionDoc()
  md.attach(new wasm.MissionDoc())
  return md
}

/** The store mirror MUST equal the authoritative wasm doc. */
function expectStoreMatchesWasm(md: MissionDoc): void {
  expect(pickMapSnapshot(useMapStore.getState())).toEqual(
    snapshotFromShadow(md.wasm as wasm.MissionDoc),
  )
}

function toClip(id: ID): ClipboardSlot {
  const s = useMapStore.getState().slotsById[id]
  return {
    role: s.role,
    tag: s.tag,
    assetId: s.assetId,
    stance: s.stance,
    position: s.position,
    squadId: s.squadId,
    loadout: s.loadout,
  }
}

/** A doc with meta + 2 layers + 3 slots (the first addSlot fans out → fallback; the rest are O(k)). */
function seeded() {
  const md = makeDoc()
  ydoc.seedMeta(md, { id: 'm1', title: 'Op' })
  ydoc.seedDefaultLayer(md)
  const l2 = ydoc.addEditorLayer(md, { name: 'Bravo' })
  const s1 = ydoc.addSlot(md, { x: 100, y: 200 }, { role: 'Squad Leader', tag: 'CMD' })
  const s2 = ydoc.addSlot(md, { x: 300, y: 400 }, { role: 'Rifleman' })
  const s3 = ydoc.addSlot(md, { x: 500, y: 600 }, { role: 'Medic', tag: 'MED', layerId: l2 })
  return { md, l2, s1, s2, s3 }
}

beforeEach(() => {
  dem.ready = true
  dem.elevation = 42.5
  useMapStore.getState().reset()
})

describe('F3.1 O(k) store patches stay byte-identical to the wasm doc', () => {
  it('addSlot — fan-out fallback (first) + O(k) into existing squad+layer', () => {
    const { md } = seeded()
    expectStoreMatchesWasm(md) // covers the fallback (s1) + two O(k) adds (s2/s3)
    md.detach()
  })

  it('addSlot — O(k) with tag + assetId (omit-when-empty shape)', () => {
    const { md } = seeded()
    ydoc.addSlot(md, { x: 700, y: 800 }, { role: 'AR', tag: 'AR', assetId: '{GUID}x.et' })
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('pasteSlots — O(k) into existing squads (anchor; index continues per squad)', () => {
    const { md, l2, s1, s2, s3 } = seeded()
    const clip = [s1, s2, s3].map(toClip)
    ydoc.pasteSlots(md, clip, { anchorAt: { x: 5000, y: 6000 }, layerId: l2 })
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('pasteSlots — O(k) no anchor (+20 nudge)', () => {
    const { md, s1, s2 } = seeded()
    ydoc.pasteSlots(md, [s1, s2].map(toClip), { anchorAt: null })
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('pasteSlots — fallback when a default squad/layer is minted (empty doc)', () => {
    const md = makeDoc()
    const clip: ClipboardSlot[] = [
      {
        role: 'Rifleman',
        stance: 'stand',
        squadId: 'gone',
        position: { x: 300, y: 400, z: 0, rotation: 0 },
      },
    ]
    ydoc.pasteSlots(md, clip, { anchorAt: { x: 1000, y: 1000 } })
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('moveEntities — O(k) (subset; z re-sampled)', () => {
    const { md, s1, s3 } = seeded()
    dem.elevation = 88.8
    ydoc.moveEntities(md, [s1, s3], { x: 12.5, y: -7.25 })
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('removeEntities — O(k) cascade across two layers', () => {
    const { md, s1, s3 } = seeded() // s1 in default, s3 in l2 → spans two layers + squad detach
    ydoc.removeEntities(md, 'slots', [s1, s3])
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('updateSlot — role / tag set / tag clear / stance', () => {
    const { md, s1, s2 } = seeded()
    ydoc.updateSlot(md, s2, { role: 'Grenadier', tag: 'AR', stance: 'prone' })
    expectStoreMatchesWasm(md)
    ydoc.updateSlot(md, s1, { tag: '' }) // clear the CMD tag → must be omitted like wasm
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('updateSlotLoadout — set / paste-copies / clear (T-068.10)', () => {
    const { md, l2, s1, s2 } = seeded()
    ydoc.updateSlotLoadout(md, s1, {
      primary: '{AAA}Rifle_M16A2.et',
      uniform: null,
      vest: null,
      helmet: '{CCC}Helmet_PASGT.et',
      optic: '{BBB}Optic_Acog.et',
      magazine: null,
      summary: 'M16A2 · ACOG',
    })
    expectStoreMatchesWasm(md)
    // Paste carries the loadout (s1 forged, s2 not) — both shapes must match wasm.
    ydoc.pasteSlots(md, [s1, s2].map(toClip), { anchorAt: { x: 5000, y: 6000 }, layerId: l2 })
    expectStoreMatchesWasm(md)
    // Clear removes the key entirely (never-forged shape), matching wasm slots_json.
    ydoc.updateSlotLoadout(md, s1, null)
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('updateSlotPosition — x-only (z follow) / manual z / rotation-only / out-of-bounds clamp', () => {
    const { md, s2 } = seeded()
    dem.elevation = 11.1
    ydoc.updateSlotPosition(md, s2, { x: 3100.5, rotation: 270 })
    expectStoreMatchesWasm(md)
    ydoc.updateSlotPosition(md, s2, { z: 5 })
    expectStoreMatchesWasm(md)
    ydoc.updateSlotPosition(md, s2, { rotation: 90 })
    expectStoreMatchesWasm(md)
    ydoc.updateSlotPosition(md, s2, { x: -500, y: 9_999_999 }) // clamp to terrain
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('editor layers — add / rename / reparent / moveSlotToLayer', () => {
    const { md, l2, s2 } = seeded()
    const l3 = ydoc.addEditorLayer(md, { name: 'Charlie' })
    expectStoreMatchesWasm(md)
    ydoc.renameEditorLayer(md, l3, 'Charlie Renamed')
    expectStoreMatchesWasm(md)
    ydoc.reparentEditorLayer(md, l3, l2)
    expectStoreMatchesWasm(md)
    ydoc.moveSlotToLayer(md, s2, l2) // s2 was in the default layer
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('removeEditorLayer — whole-resync fallback', () => {
    const { md, l2 } = seeded()
    ydoc.removeEditorLayer(md, l2) // deletes l2 + cascades s3
    expectStoreMatchesWasm(md)
    md.detach()
  })

  it('setTitle / updateEnvironment — O(k) meta patch', () => {
    const { md } = seeded()
    ydoc.setTitle(md, 'Op Nightfall')
    expectStoreMatchesWasm(md)
    ydoc.updateEnvironment(md, { weather: 'overcast', viewDistance: 3200 })
    expectStoreMatchesWasm(md)
    md.detach()
  })
})
