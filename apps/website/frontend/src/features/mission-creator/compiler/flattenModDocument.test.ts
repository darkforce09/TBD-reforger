// T-092.2 — flattenEditorToModDocument invariants: deterministic slot identity, the locked
// editor->mod coordinate mapping, kit-alias resolution + faction fallback, orbat/slots count
// parity (TBD_MissionLoader hard-fails on mismatch), y emission + schemaVersion bump, and the
// synthesized required blocks. Full JSON-Schema validation of the same document shape runs
// server-side (internal/handlers TestGetCompiledMission against the embedded mission.schema.json).
import { describe, expect, it } from 'vitest'

import type { MapSnapshot } from '@/features/tactical-map/state/useMapStore'

import { flattenEditorToModDocument } from './flattenModDocument'

const US_GL = '{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et'
const US_MEDIC = '{C9E4FEAF5AAC8D8C}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Medic.et'
const SOV_RIFLE = '{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et'

function slot(
  id: string,
  squadId: string,
  index: number,
  role: string,
  pos: { x: number; y: number; z?: number; rotation?: number },
  assetId?: string,
) {
  return {
    id,
    squadId,
    index,
    role,
    assetId,
    position: { x: pos.x, y: pos.y, z: pos.z ?? 0, rotation: pos.rotation ?? 0 },
    stance: 'stand' as const,
    loadoutId: null,
  }
}

function baseSnapshot(): MapSnapshot {
  return {
    meta: {
      id: 'A1B2C3D4-0000-4000-8000-00000000FEED',
      title: 'Flatten Fixture',
      terrain: 'everon',
      environment: { time: '05:30', weather: 'overcast' },
    },
    factionsById: {
      f1: { id: 'f1', key: 'BLUFOR', name: 'US Army', squadIds: ['sq1'] },
      f2: { id: 'f2', key: 'OPFOR', name: 'Soviet VDV', squadIds: ['sq2'] },
    },
    squadsById: {
      sq1: { id: 'sq1', factionId: 'f1', callsign: 'Alpha', name: 'Alpha 1-1', slotIds: ['s1', 's2', 's3', 's4'] },
      sq2: { id: 'sq2', factionId: 'f2', name: 'Grom', slotIds: ['s5'] },
    },
    slotsById: {
      s1: slot('s1', 'sq1', 0, 'SL', { x: 4839.2, y: 6620.8, rotation: 270 }, US_GL),
      s2: slot('s2', 'sq1', 1, 'TL', { x: 4836.9, y: 6626.5, rotation: 450 }, US_MEDIC),
      s3: slot('s3', 'sq1', 2, 'TL', { x: 4831.2, y: 6628.8, z: 142.5 }),
      s4: slot('s4', 'sq1', 3, 'RFL', { x: 4825.5, y: 6626.5, rotation: -45 }),
      s5: slot('s5', 'sq2', 0, 'RFL', { x: 6010, y: 7211.5 }, SOV_RIFLE),
    },
    loadoutsById: {},
    itemsById: {},
    objectivesById: {},
    vehiclesById: {},
    markersById: {},
    editorLayersById: {},
  }
}

describe('flattenEditorToModDocument', () => {
  it('emits deterministic slot ids, locked coordinate mapping, and kit aliases', () => {
    const doc = flattenEditorToModDocument(baseSnapshot(), { maxPlayers: 64 })

    expect(doc.slots.map((s) => s.id)).toEqual([
      'blufor:Alpha:SL:0',
      'blufor:Alpha:TL:0',
      'blufor:Alpha:TL:1',
      'blufor:Alpha:RFL:0',
      'opfor:Grom:RFL:0',
    ])

    const sl = doc.slots[0]
    // editor position.x -> x, position.y -> z (locked mapping)
    expect(sl.x).toBe(4839.2)
    expect(sl.z).toBe(6620.8)
    expect(sl.headingDeg).toBe(270)
    expect(sl.kit).toBe('kit:us_sl') // mapped from the US_GL ResourceName

    // unmapped Medic + missing assetIds fall back to the faction default kit
    expect(doc.slots[1].kit).toBe('kit:us_rifleman')
    expect(doc.slots[4].kit).toBe('kit:sov_rifleman')

    // rotation normalized into [0, 360)
    expect(doc.slots[1].headingDeg).toBe(90)
    expect(doc.slots[3].headingDeg).toBe(315)
  })

  it('bumps to schemaVersion 1.2 only when a slot carries a real elevation', () => {
    const doc = flattenEditorToModDocument(baseSnapshot())
    expect(doc.schemaVersion).toBe('1.2')
    expect(doc.slots[2].y).toBe(142.5)
    expect(doc.slots[0].y).toBeUndefined() // z=0 placeholder never emits y

    const flat = baseSnapshot()
    flat.slotsById.s3.position.z = 0
    const flatDoc = flattenEditorToModDocument(flat)
    expect(flatDoc.schemaVersion).toBe('1.1')
    expect(flatDoc.slots.every((s) => s.y === undefined)).toBe(true)
  })

  it('keeps orbat instance counts equal to slots length (loader parity gate)', () => {
    const doc = flattenEditorToModDocument(baseSnapshot())
    const orbatCount = Object.values(doc.orbat)
      .flatMap((f) => f.groups)
      .flatMap((g) => g.roles)
      .reduce((n, r) => n + r.count, 0)
    expect(orbatCount).toBe(doc.slots.length)
    expect(doc.orbat.blufor.groups[0].roles).toEqual([
      { slot: 'SL', kit: 'kit:us_sl', count: 1 },
      { slot: 'TL', kit: 'kit:us_rifleman', count: 2 },
      { slot: 'RFL', kit: 'kit:us_rifleman', count: 1 },
    ])
  })

  it('synthesizes the required blocks the editor never authors', () => {
    const doc = flattenEditorToModDocument(baseSnapshot(), { maxPlayers: 64 })

    expect(doc.meta.id).toBe('msn_a1b2c3d400004000800000000000feed')
    expect(doc.meta.templateId).toBe('editor_v1')
    expect(doc.meta.playerRange).toEqual([1, 64])
    expect(doc.environment?.dateTime).toBe('1989-06-14T05:30:00Z')

    const spawnZones = doc.zones.filter((z) => z.type === 'spawn')
    expect(spawnZones.map((z) => z.faction).sort()).toEqual(['blufor', 'opfor'])
    const blu = spawnZones.find((z) => z.faction === 'blufor')
    expect(blu && 'circle' in blu.shape && blu.shape.circle.r).toBe(150)

    expect(doc.flow.jip).toBe('until_safestart_end')
    expect(doc.winConditions.endOn).toContain('time_limit')
    expect(doc.factions.map((f) => f.presetId)).toEqual(['preset:us_army_82nd', 'preset:sov_vdv'])
  })

  it('pads a stub opposing faction for single-faction drafts (schema needs 2)', () => {
    const s = baseSnapshot()
    delete (s.factionsById as Record<string, unknown>).f2
    delete (s.squadsById as Record<string, unknown>).sq2
    delete (s.slotsById as Record<string, unknown>).s5
    const doc = flattenEditorToModDocument(s)
    expect(doc.factions).toHaveLength(2)
    expect(doc.factions[1].key).toBe('opfor')
    expect(doc.orbat.opfor).toBeUndefined() // stub has no orbat entry — parity preserved
  })

  it('refuses to flatten a mission with no placed slots', () => {
    const s = baseSnapshot()
    s.slotsById = {}
    s.squadsById.sq1.slotIds = []
    s.squadsById.sq2.slotIds = []
    expect(() => flattenEditorToModDocument(s)).toThrow(/without placed slots/)
  })
})
