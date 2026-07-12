import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

import {
  ATTACH_EDGE_TYPES,
  EQUIP_EDGE_TYPES,
  buildIndex,
  canAttach,
  canEquip,
  edgesOf,
  hasEdge,
  hostsFor,
  itemsFor,
  stats,
  type CompatEdgeTuple,
} from './registryGraph'

// T-068.9 proof-ledger gates G6 (index losslessness) + G7 (query == oracle) over
// the REAL committed T-150 envelope -- exhaustive on all positives, seeded-LCG
// deterministic sample on the negative complement.

const ENVELOPE_PATH = resolve(
  process.cwd(),
  '../../../packages/tbd-schema/registry/registry-compat.workbench.json',
)

interface Envelope {
  edges: Array<{ from_node: string; to_node: string; edge_type: string; evidence?: string }>
}

const envelope = JSON.parse(readFileSync(ENVELOPE_PATH, 'utf8')) as Envelope
const edges: CompatEdgeTuple[] = envelope.edges
const key = (f: string, t: string, ty: string): string => `${f}|${t}|${ty}`

// Oracle = the raw edge list, as a set + naive scans (no index code involved).
const oracleSet = new Set(edges.map((e) => key(e.from_node, e.to_node, e.edge_type)))
const ix = buildIndex(edges)

const M16A2 = '{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et'
const STANAG_M855 =
  '{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et'
const AK_MAG = '{63C1E699345B24F9}Prefabs/Weapons/Magazines/Magazine_545x39_AK_30rnd_Base.et'
const M60_MOUNTED = '{6AF5FA1A839A4980}Prefabs/Weapons/MachineGuns/M60/MG_M60_Mounted.et'
const M60_BOX = '{AAF51CFA75A9CF8B}Prefabs/Weapons/Magazines/Box_762x51_M60_100rnd_4AP_1Tracer.et'
const PASGT_HELMET =
  '{FE5C49069C2499D9}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover.et'
const GUARD_CHAR =
  '{F6A45EA59BA3C2E8}Prefabs/Characters/Factions/BLUFOR/US_Army/Guard/Character_US_GL_Guard.et'

describe('registryGraph on the committed T-150 envelope', () => {
  it('loads the expected graph (4,685 edges, 6 families — T-068.10.2 census-gated export)', () => {
    const s = stats(ix)
    expect(s.total).toBe(4685)
    expect(s.byType).toEqual({
      character_default_loadout: 2746,
      character_default_weapon: 673,
      // 16 mag edges moved family in T-068.10.2: statics (NSV/mortars/cannons) reclassified
      // vehicle_weapon, so their well-matches emit as mag_in_vehicle_weapon now.
      mag_in_weapon: 529,
      optic_on_weapon: 362,
      attachment_on_weapon: 241,
      mag_in_vehicle_weapon: 134,
    })
  })

  it('G6: index round-trips to the exact input edge set, from both sides', () => {
    const input = new Set(edges.map((e) => key(e.from_node, e.to_node, e.edge_type)))
    const fromSide = new Set(
      edgesOf(ix, 'from').map((e) => key(e.from_node, e.to_node, e.edge_type)),
    )
    const toSide = new Set(edgesOf(ix, 'to').map((e) => key(e.from_node, e.to_node, e.edge_type)))
    expect(fromSide.size).toBe(input.size)
    expect(toSide.size).toBe(input.size)
    for (const k of input) {
      if (!fromSide.has(k) || !toSide.has(k)) throw new Error(`edge lost by index: ${k}`)
    }
  })

  it('G7: hasEdge is true for ALL envelope edges (typed + untyped), exhaustive', () => {
    for (const e of edges) {
      if (!hasEdge(ix, e.from_node, e.to_node, e.edge_type)) {
        throw new Error(`typed hasEdge false for ${key(e.from_node, e.to_node, e.edge_type)}`)
      }
      if (!hasEdge(ix, e.from_node, e.to_node)) {
        throw new Error(`untyped hasEdge false for ${e.from_node} -> ${e.to_node}`)
      }
    }
  })

  it('G7: hasEdge is false for 1,000 seeded non-edges (oracle-verified complement)', () => {
    const froms = [...new Set(edges.map((e) => e.from_node))]
    const tos = [...new Set(edges.map((e) => e.to_node))]
    const types = Object.keys(stats(ix).byType)
    // Deterministic LCG (Numerical Recipes constants) -- reproducible sample.
    let seed = 0x1234_5678
    const next = (n: number): number => {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0
      return seed % n
    }
    let checked = 0
    let guard = 0
    while (checked < 1000 && guard < 100_000) {
      guard += 1
      const f = froms[next(froms.length)]
      const t = tos[next(tos.length)]
      const ty = types[next(types.length)]
      if (oracleSet.has(key(f, t, ty))) continue // real edge -- skip, we want the complement
      if (hasEdge(ix, f, t, ty)) throw new Error(`false positive: ${key(f, t, ty)}`)
      checked += 1
    }
    expect(checked).toBe(1000)
  })

  it('G7: itemsFor/hostsFor equal the oracle groupings for EVERY host and item', () => {
    const oracleItemsFor = new Map<string, Set<string>>()
    const oracleHostsFor = new Map<string, Set<string>>()
    for (const e of edges) {
      let s = oracleItemsFor.get(e.to_node)
      if (!s) oracleItemsFor.set(e.to_node, (s = new Set()))
      s.add(e.from_node)
      let h = oracleHostsFor.get(e.from_node)
      if (!h) oracleHostsFor.set(e.from_node, (h = new Set()))
      h.add(e.to_node)
    }
    for (const [host, want] of oracleItemsFor) {
      const got = itemsFor(ix, host)
      if (got.length !== want.size || !got.every((g) => want.has(g))) {
        throw new Error(`itemsFor mismatch for ${host}`)
      }
    }
    for (const [item, want] of oracleHostsFor) {
      const got = hostsFor(ix, item)
      if (got.length !== want.size || !got.every((g) => want.has(g))) {
        throw new Error(`hostsFor mismatch for ${item}`)
      }
    }
  })

  it('G7: canAttach/canEquip equal the oracle family unions for every edge', () => {
    const attach = new Set<string>(ATTACH_EDGE_TYPES)
    const equip = new Set<string>(EQUIP_EDGE_TYPES)
    for (const e of edges) {
      const wantAttach = edges.some(
        (o) => o.from_node === e.from_node && o.to_node === e.to_node && attach.has(o.edge_type),
      )
      const wantEquip = edges.some(
        (o) => o.from_node === e.from_node && o.to_node === e.to_node && equip.has(o.edge_type),
      )
      if (canAttach(ix, e.from_node, e.to_node) !== wantAttach) {
        throw new Error(`canAttach mismatch for ${e.from_node} -> ${e.to_node}`)
      }
      if (canEquip(ix, e.from_node, e.to_node) !== wantEquip) {
        throw new Error(`canEquip mismatch for ${e.from_node} -> ${e.to_node}`)
      }
    }
  })

  it('answers the named pairs: STANAG fits the M16A2, the AK mag does not', () => {
    expect(canAttach(ix, STANAG_M855, M16A2)).toBe(true)
    expect(hasEdge(ix, STANAG_M855, M16A2, 'mag_in_weapon')).toBe(true)
    expect(canAttach(ix, AK_MAG, M16A2)).toBe(false) // cross-well negative
    expect(hasEdge(ix, AK_MAG, M16A2)).toBe(false)
  })

  it('answers the vehicle-weapon pair: M60 box loads the mounted M60 (6 boxes total)', () => {
    expect(canAttach(ix, M60_BOX, M60_MOUNTED)).toBe(true)
    expect(hasEdge(ix, M60_BOX, M60_MOUNTED, 'mag_in_vehicle_weapon')).toBe(true)
    const boxes = itemsFor(ix, M60_MOUNTED, 'mag_in_vehicle_weapon')
    expect(boxes).toHaveLength(6)
    expect(boxes).toContain(M60_BOX)
  })

  it('answers the equip pair: PASGT helmet is in the Guard default loadout, not a rifle', () => {
    expect(canEquip(ix, PASGT_HELMET, GUARD_CHAR)).toBe(true)
    expect(canEquip(ix, PASGT_HELMET, M16A2)).toBe(false)
    expect(canAttach(ix, PASGT_HELMET, GUARD_CHAR)).toBe(false) // equip != attach family
  })

  it('flows unknown future edge families through untouched (any-mod invariant)', () => {
    const future = buildIndex([
      {
        from_node: '{AB12CD34EF56AB01}Prefabs/X/A.et',
        to_node: '{AB12CD34EF56AB02}Prefabs/X/B.et',
        edge_type: 'gear_in_slot',
      },
    ])
    expect(
      hasEdge(future, '{AB12CD34EF56AB01}Prefabs/X/A.et', '{AB12CD34EF56AB02}Prefabs/X/B.et'),
    ).toBe(true)
    expect(
      hasEdge(
        future,
        '{AB12CD34EF56AB01}Prefabs/X/A.et',
        '{AB12CD34EF56AB02}Prefabs/X/B.et',
        'gear_in_slot',
      ),
    ).toBe(true)
    expect(itemsFor(future, '{AB12CD34EF56AB02}Prefabs/X/B.et', 'gear_in_slot')).toEqual([
      '{AB12CD34EF56AB01}Prefabs/X/A.et',
    ])
    expect(stats(future).byType).toEqual({ gear_in_slot: 1 })
  })
})
