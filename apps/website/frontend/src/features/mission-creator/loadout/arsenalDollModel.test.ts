// T-068.10.7 doll-model gates: region completeness against EMPTY_PICKS (every pickable key
// exactly one hotspot; optic/magazine ride the rifle) and honest weight math on real GUIDs
// from the committed census-gated envelope (RGD5 serializes 0.31 kg; no vanilla primary
// serializes weight — unknown, never guessed).

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import type { RegistryItem } from '@/types/models/registry'
import {
  DOLL_REGIONS,
  PRIMARY_SUB_REGIONS,
  formatLoadoutWeight,
  loadoutWeight,
} from './arsenalDollModel'
import { EMPTY_PICKS } from './arsenalRules'

const envelope = JSON.parse(
  readFileSync(
    resolve(
      __dirname,
      '../../../../../../../packages/tbd-schema/registry/registry-items.workbench.json',
    ),
    'utf-8',
  ),
) as { items: RegistryItem[] }
const catalog = envelope.items
const byName = new Map(catalog.map((i) => [i.resource_name, i]))

function must<T>(v: T | undefined, what: string): T {
  if (v === undefined) throw new Error(`fixture missing: ${what}`)
  return v
}
const find = (name: string) => must(catalog.find((i) => i.display_name === name), name)

describe('DOLL_REGIONS', () => {
  it('covers every pickable key exactly once (main regions + rifle sub-hotspots)', () => {
    const covered = [...DOLL_REGIONS.map((r) => r.key), ...PRIMARY_SUB_REGIONS]
    expect(new Set(covered).size).toBe(covered.length)
    expect([...covered].sort()).toEqual(Object.keys(EMPTY_PICKS).sort())
  })

  it('models optic/magazine as rifle sub-hotspots, not standalone regions', () => {
    expect(PRIMARY_SUB_REGIONS).toEqual(['optic', 'magazine'])
    const main = new Set(DOLL_REGIONS.map((r) => r.key))
    for (const k of PRIMARY_SUB_REGIONS) expect(main.has(k)).toBe(false)
    expect(main.has('primary')).toBe(true)
  })
})

describe('loadoutWeight', () => {
  it('sums serialized weights (RGD5 0.31 kg from the prefab text)', () => {
    const rgd = find('Grenade RGD5')
    const w = loadoutWeight({ ...EMPTY_PICKS, throwable: rgd.resource_name }, byName)
    expect(w.knownKg).toBeCloseTo(0.31, 2)
    expect(w.unknownCount).toBe(0)
    expect(w.itemCount).toBe(1)
  })

  it('counts null-weight items as unknown, never 0-known (AK-74 serializes no weight)', () => {
    const ak = find('Rifle AK74')
    // Data-truth guard: if the export ever starts serializing rifle weights, this test
    // must be revisited rather than silently passing.
    expect(ak.weight_kg ?? null).toBeNull()
    const w = loadoutWeight({ ...EMPTY_PICKS, primary: ak.resource_name }, byName)
    expect(w).toEqual({ knownKg: 0, unknownCount: 1, itemCount: 1 })
  })

  it('mixed picks: known sum + unknown count + total items', () => {
    const w = loadoutWeight(
      {
        ...EMPTY_PICKS,
        throwable: find('Grenade RGD5').resource_name,
        primary: find('Rifle AK74').resource_name,
      },
      byName,
    )
    expect(w.knownKg).toBeCloseTo(0.31, 2)
    expect(w.unknownCount).toBe(1)
    expect(w.itemCount).toBe(2)
  })

  it('empty picks → zeros; a resource missing from the catalog counts unknown', () => {
    expect(loadoutWeight(EMPTY_PICKS, byName)).toEqual({
      knownKg: 0,
      unknownCount: 0,
      itemCount: 0,
    })
    const w = loadoutWeight(
      { ...EMPTY_PICKS, primary: '{0000000000000000}Prefabs/Nope.et' },
      byName,
    )
    expect(w).toEqual({ knownKg: 0, unknownCount: 1, itemCount: 1 })
  })
})

describe('formatLoadoutWeight', () => {
  it('renders ≥ + unknown count when data is missing, plain sum otherwise', () => {
    expect(formatLoadoutWeight({ knownKg: 0.31, unknownCount: 1, itemCount: 2 })).toBe(
      '≥ 0.3 kg · 1 item without weight data',
    )
    expect(formatLoadoutWeight({ knownKg: 0.62, unknownCount: 0, itemCount: 2 })).toBe(
      '0.6 kg · 2 items',
    )
  })
})
