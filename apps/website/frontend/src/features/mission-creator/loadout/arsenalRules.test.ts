// Smart Forge rules (T-068.10) — pure-module tests: option building (filter / stranded-retain),
// data-driven validation, pick↔loadout mapping, and the display summary.

import { describe, expect, it } from 'vitest'
import type { RegistryItem } from '@/types/models/registry'
import {
  EMPTY_PICKS,
  LOADOUT_ROWS,
  buildLoadoutSummary,
  buildRowOptions,
  loadoutToPicks,
  picksToLoadout,
  validateLoadout,
  type CompatSets,
  type LoadoutRow,
} from './arsenalRules'

const RIFLE = '{AAA}Prefabs/Weapons/Rifle_M16A2.et'
const RIFLE_AK = '{AA2}Prefabs/Weapons/Rifle_AK74.et'
const ACOG = '{BBB}Prefabs/Attachments/Optic_Acog.et'
const KOBRA = '{BB2}Prefabs/Attachments/Optic_Kobra.et'
const STANAG = '{CCC}Prefabs/Magazines/Mag_556_30rnd.et'
const BDU = '{DDD}Prefabs/Clothing/Uniform_BDU.et'
const GORKA = '{DD2}Prefabs/Clothing/Uniform_Gorka.et'

function item(
  resource_name: string,
  kind: RegistryItem['kind'],
  display_name: string,
): RegistryItem {
  return {
    id: resource_name,
    modpack_id: 'mp-1',
    resource_name,
    display_name,
    category: 'test',
    kind,
    sort_order: 0,
    created_at: '',
    updated_at: '',
  }
}

const CATALOG: RegistryItem[] = [
  item(RIFLE, 'gear_primary', 'M16A2'),
  item(RIFLE_AK, 'gear_primary', 'AK-74'),
  item(ACOG, 'optic', 'ACOG'),
  item(STANAG, 'magazine', '30rnd STANAG'),
  item(BDU, 'gear_uniform', 'BDU Woodland'),
  item(GORKA, 'gear_uniform', 'Gorka'),
]
const BY_NAME = new Map(CATALOG.map((i) => [i.resource_name, i]))

const primaryRow = LOADOUT_ROWS.find((r) => r.key === 'primary') as LoadoutRow
const uniformRow = LOADOUT_ROWS.find((r) => r.key === 'uniform') as LoadoutRow
const opticRow = LOADOUT_ROWS.find((r) => r.key === 'optic') as LoadoutRow
const magRow = LOADOUT_ROWS.find((r) => r.key === 'magazine') as LoadoutRow

const NO_COMPAT: CompatSets = { edgeItems: {} }

describe('buildRowOptions', () => {
  it('kind row without compat data lists the whole kind (None first)', () => {
    const opts = buildRowOptions(primaryRow, '', NO_COMPAT, CATALOG, BY_NAME)
    expect(opts.map((o) => o.value)).toEqual(['', RIFLE, RIFLE_AK])
    expect(opts[0].label).toBe('— None —')
    expect(opts[1].label).toBe('M16A2')
  })

  it('kind rows are never compat-constrained — full mix-and-match (T-068.10.1)', () => {
    // Any character wears any clothing; the graph only constrains weapon families.
    const sets: CompatSets = { edgeItems: { optic: [ACOG] } }
    expect(buildRowOptions(uniformRow, '', sets, CATALOG, BY_NAME).map((o) => o.value)).toEqual([
      '',
      BDU,
      GORKA,
    ])
    expect(buildRowOptions(primaryRow, '', sets, CATALOG, BY_NAME).map((o) => o.value)).toEqual([
      '',
      RIFLE,
      RIFLE_AK,
    ])
  })

  it('retains a stranded current value with an incompatible suffix (kind row)', () => {
    // A pick that is no longer in the catalog kind (e.g. registry changed) stays listed.
    const opts = buildRowOptions(primaryRow, ACOG, NO_COMPAT, CATALOG, BY_NAME)
    const stranded = opts.at(-1)
    expect(stranded?.value).toBe(ACOG)
    expect(stranded?.label).toBe('ACOG — incompatible')
  })

  it('edge row lists the compat feed with display names, raw resource_name fallback', () => {
    const unknown = '{ZZZ}Prefabs/Attachments/Optic_Unknown.et'
    const sets: CompatSets = { edgeItems: { optic: [ACOG, unknown] } }
    const opts = buildRowOptions(opticRow, '', sets, CATALOG, BY_NAME)
    expect(opts.map((o) => o.label)).toEqual(['— None —', 'ACOG', unknown])
  })

  it('edge row retains a stranded pick after the weapon changed', () => {
    const sets: CompatSets = { edgeItems: { optic: [KOBRA] } }
    const opts = buildRowOptions(opticRow, ACOG, sets, CATALOG, BY_NAME)
    expect(opts.at(-1)).toEqual({ value: ACOG, label: 'ACOG — incompatible' })
  })
})

describe('validateLoadout', () => {
  const READY: CompatSets = {
    edgeItems: { optic: [ACOG], magazine: [STANAG] },
  }

  it('all-empty picks are valid', () => {
    expect(validateLoadout({ ...EMPTY_PICKS }, READY)).toEqual({ valid: true, errors: {} })
  })

  it('a full compatible kit is valid', () => {
    const picks = { ...EMPTY_PICKS, primary: RIFLE, uniform: BDU, optic: ACOG, magazine: STANAG }
    expect(validateLoadout(picks, READY).valid).toBe(true)
  })

  it('clothing is never invalid — any uniform on any character (T-068.10.1)', () => {
    const picks = { ...EMPTY_PICKS, uniform: GORKA, vest: ACOG, helmet: RIFLE }
    expect(validateLoadout(picks, READY).valid).toBe(true)
    expect(validateLoadout(picks, NO_COMPAT).valid).toBe(true)
  })

  it('edge pick without its dependency is invalid', () => {
    const picks = { ...EMPTY_PICKS, optic: ACOG }
    expect(validateLoadout(picks, READY).errors.optic).toMatch(/Requires a primary/)
  })

  it('edge pick the compat feed rejects is invalid (stale optic after weapon swap)', () => {
    const swapped: CompatSets = { ...READY, edgeItems: { optic: [KOBRA], magazine: [] } }
    const picks = { ...EMPTY_PICKS, primary: RIFLE, optic: ACOG }
    const v = validateLoadout(picks, swapped)
    expect(v.valid).toBe(false)
    expect(v.errors.optic).toMatch(/Not compatible with the selected primary/)
  })

  it('is data-driven off LOADOUT_ROWS (magazine follows the same edge rule)', () => {
    const picks = { ...EMPTY_PICKS, primary: RIFLE, magazine: ACOG }
    expect(validateLoadout(picks, READY).errors.magazine).toMatch(/Not compatible/)
    expect(magRow.source.type).toBe('edge')
  })
})

describe('picks ↔ loadout mapping', () => {
  it('all-empty picks map to null (clears the doc field)', () => {
    expect(picksToLoadout({ ...EMPTY_PICKS }, BY_NAME)).toBeNull()
  })

  it('round-trips picks through SlotLoadout with a display summary', () => {
    const picks = { ...EMPTY_PICKS, primary: RIFLE, optic: ACOG, magazine: STANAG, uniform: BDU }
    const loadout = picksToLoadout(picks, BY_NAME)
    expect(loadout).toEqual({
      primary: RIFLE,
      uniform: BDU,
      vest: null,
      helmet: null,
      optic: ACOG,
      magazine: STANAG,
      summary: 'M16A2 · ACOG · 30rnd STANAG',
    })
    expect(loadoutToPicks(loadout ?? undefined)).toEqual(picks)
  })

  it('loadoutToPicks of an unforged slot is all-empty', () => {
    expect(loadoutToPicks(undefined)).toEqual(EMPTY_PICKS)
  })

  it('summary omits empty attach slots and is empty without a primary', () => {
    expect(buildLoadoutSummary({ ...EMPTY_PICKS, primary: RIFLE }, BY_NAME)).toBe('M16A2')
    expect(buildLoadoutSummary({ ...EMPTY_PICKS }, BY_NAME)).toBe('')
  })
})
