// T-153 palette tree gates: side ordering + accents, role drag payloads (assetId + tag +
// loadout + factionRef), vehicle leaves listed but payload-less (T-070), and the
// structural vanilla-purge guarantee — the tree is a pure function of the LIBRARY, so no
// registry character can appear in it.

import { describe, expect, it } from 'vitest'
import type { UserFaction } from '@/types/models/faction'
import { SIDE_ICON_CLASS, buildFactionTree } from './buildFactionTree'

const LIB: UserFaction[] = [
  {
    id: 'f-ussr',
    owner_id: 'me',
    side: 'OPFOR',
    name: 'Soviet Army 1980s',
    created_at: '',
    updated_at: '',
    doc: {
      side: 'OPFOR',
      name: 'Soviet Army 1980s',
      roles: [
        {
          role: 'Squad Leader',
          character: '{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et',
          loadout: { version: 2, wear: { jacket: null }, weapons: [] },
        },
        {
          role: 'Rifleman',
          tag: 'AT',
          character: '{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et',
        },
      ],
      vehicles: [
        { vehicle: '{259EE7B78C51B624}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et', label: 'UAZ-469' },
      ],
    },
  },
  {
    id: 'f-us',
    owner_id: 'me',
    side: 'BLUFOR',
    name: 'US Army 1980s',
    created_at: '',
    updated_at: '',
    doc: { side: 'BLUFOR', name: 'US Army 1980s', roles: [], vehicles: [] },
  },
]

describe('buildFactionTree', () => {
  it('groups side → faction → roles/vehicles, sides in fixed order with accents', () => {
    const { nodes } = buildFactionTree(LIB)
    expect(nodes.map((n) => n.label)).toEqual(['BLUFOR', 'OPFOR'])
    expect(nodes[0].iconClassName).toBe(SIDE_ICON_CLASS.BLUFOR)
    expect(nodes[1].iconClassName).toBe(SIDE_ICON_CLASS.OPFOR)
    const ussr = nodes[1].children?.[0]
    expect(ussr?.label).toBe('Soviet Army 1980s')
    expect(ussr?.children?.map((c) => c.label)).toEqual(['Roles', 'Vehicles'])
  })

  it('role leaves carry the full drop payload (assetId, tag, loadout, factionRef)', () => {
    const { nodes, payloadById } = buildFactionTree(LIB)
    const roles = nodes[1].children?.[0].children?.[0].children ?? []
    expect(roles.map((r) => r.label)).toEqual(['Squad Leader', 'Rifleman'])
    expect(roles[1].badge).toBe('AT')

    const sl = payloadById.get(roles[0].id)
    expect(sl?.kind).toBe('slot')
    expect(sl?.assetId).toContain('Character_USSR_Rifleman.et')
    expect(sl?.loadout).toBeTruthy()
    expect(sl?.factionRef).toEqual({ side: 'OPFOR', name: 'Soviet Army 1980s' })

    const rf = payloadById.get(roles[1].id)
    expect(rf?.tag).toBe('AT')
    expect(rf?.loadout).toBeUndefined()
  })

  it('vehicle leaves are listed but carry no payload (placement lands with T-070)', () => {
    const { nodes, payloadById } = buildFactionTree(LIB)
    const vehicles = nodes[1].children?.[0].children?.[1].children ?? []
    expect(vehicles).toHaveLength(1)
    expect(vehicles[0].label).toBe('UAZ-469')
    expect(vehicles[0].badge).toBe('T-070')
    expect(payloadById.has(vehicles[0].id)).toBe(false)
  })

  it('structural vanilla purge: every leaf id derives from library rows, none from the registry', () => {
    const { nodes, payloadById } = buildFactionTree(LIB)
    const ids: string[] = []
    const walk = (ns: typeof nodes) =>
      ns.forEach((n) => {
        ids.push(n.id)
        if (n.children) walk(n.children)
      })
    walk(nodes)
    expect(ids.every((id) => /^(side:|faction:|roles:|vehicles:|role:|veh:)/.test(id))).toBe(true)
    // payload assetIds come from role templates, never enumerated from registry kinds
    for (const p of payloadById.values()) expect(p.factionRef).toBeTruthy()
  })

  it('empty library → empty tree (AssetBrowser renders the Manager CTA)', () => {
    const { nodes, payloadById } = buildFactionTree([])
    expect(nodes).toHaveLength(0)
    expect(payloadById.size).toBe(0)
  })
})
