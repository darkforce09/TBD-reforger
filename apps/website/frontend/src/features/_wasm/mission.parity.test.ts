import { describe, it, expect } from 'vitest'
import * as wasm from '@/wasm/pkg/map_engine_wasm'

// The wasm flatten is the SAME Rust code the Axum backend runs for GET /missions/:id/compiled
// (map-engine-core::mission::flatten). This exercises it through the JS boundary on the locked
// fixture (two factions, callsigned squads, duplicate role, one slot with real elevation) and
// asserts the mod-document contract — the same values pinned by the Rust core test and the TS
// flattenModDocument.test.ts, so all three agree by construction.
const META = {
  id: '11112222333344445555666677778888',
  title: 'Compiled Fixture',
  author: 'maker',
  terrain: 'everon',
  customTerrainName: '',
  maxPlayers: 64,
  timeOfDay: '05:30',
  weatherPreset: 'clear',
}

const PAYLOAD = {
  schemaVersion: 1,
  map: { terrain: 'everon', bounds: [0, 0, 12800, 12800] },
  editor: {
    factions: [
      { id: 'f1', key: 'BLUFOR', name: 'US Army', squadIds: ['sq1'] },
      { id: 'f2', key: 'OPFOR', name: 'Soviet VDV', squadIds: ['sq2'] },
    ],
    squads: [
      {
        id: 'sq1',
        factionId: 'f1',
        callsign: 'Alpha',
        name: 'Alpha 1-1',
        slotIds: ['s1', 's2', 's3'],
      },
      { id: 'sq2', factionId: 'f2', name: 'Grom', slotIds: ['s4'] },
    ],
    slots: [
      {
        id: 's1',
        squadId: 'sq1',
        index: 0,
        role: 'SL',
        assetId: '{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et',
        position: { x: 4839.2, y: 6620.8, z: 0, rotation: 270 },
      },
      {
        id: 's2',
        squadId: 'sq1',
        index: 1,
        role: 'TL',
        position: { x: 4836.9, y: 6626.5, z: 142.5, rotation: 450 },
      },
      {
        id: 's3',
        squadId: 'sq1',
        index: 2,
        role: 'TL',
        position: { x: 4831.2, y: 6628.8, z: 0, rotation: 0 },
      },
      {
        id: 's4',
        squadId: 'sq2',
        index: 0,
        role: 'RFL',
        assetId:
          '{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et',
        position: { x: 6010, y: 7211.5, z: 0, rotation: 90 },
      },
    ],
    editorLayers: [],
  },
}

interface ModSlot {
  id: string
  kit: string
  x: number
  z: number
  y?: number
  headingDeg: number
}
interface ModDoc {
  schemaVersion: string
  meta: { playerRange: [number, number] }
  slots: ModSlot[]
  orbat: Record<string, { groups: { roles: { count: number }[] }[] }>
}

function flatten(meta: object, payload: object): ModDoc {
  const enc = new TextEncoder()
  const out = wasm.flatten_mod_document(
    enc.encode(JSON.stringify(meta)),
    enc.encode(JSON.stringify(payload)),
  )
  return JSON.parse(new TextDecoder().decode(out)) as ModDoc
}

describe('map-engine-wasm flatten_mod_document — mod-document contract (Class R)', () => {
  it('produces the locked mod document from the fixture', () => {
    const doc = flatten(META, PAYLOAD)

    // One slot carries y → schemaVersion 1.2.
    expect(doc.schemaVersion).toBe('1.2')

    // Deterministic slot ids (faction:callsign:role:occurrence).
    expect(doc.slots.map((s) => s.id)).toEqual([
      'blufor:Alpha:SL:0',
      'blufor:Alpha:TL:0',
      'blufor:Alpha:TL:1',
      'opfor:Grom:RFL:0',
    ])

    // Locked mapping: x→x, y→z, z→y (optional), rotation→headingDeg (mod 360).
    const s0 = doc.slots[0]
    expect(s0.x).toBeCloseTo(4839.2, 9)
    expect(s0.z).toBeCloseTo(6620.8, 9)
    expect(s0.y).toBeUndefined()
    expect(s0.headingDeg).toBeCloseTo(270, 9)
    expect(doc.slots[1].y).toBeCloseTo(142.5, 9)
    expect(doc.slots[1].headingDeg).toBeCloseTo(90, 9) // 450 % 360

    // Kit aliases: mapped assetId → kit; unmapped → faction default.
    expect(s0.kit).toBe('kit:us_sl')
    expect(doc.slots[1].kit).toBe('kit:us_rifleman')
    expect(doc.slots[3].kit).toBe('kit:sov_rifleman')

    // ORBAT instance count == slots length (loader parity gate).
    const orbatCount = Object.values(doc.orbat)
      .flatMap((f) => f.groups)
      .flatMap((g) => g.roles)
      .reduce((sum, r) => sum + r.count, 0)
    expect(orbatCount).toBe(doc.slots.length)

    expect(doc.meta.playerRange).toEqual([1, 64])
  })

  it('throws on an editor with no placed slots', () => {
    const empty = { editor: { factions: [], squads: [], slots: [], editorLayers: [] } }
    expect(() => flatten(META, empty)).toThrow()
  })
})
