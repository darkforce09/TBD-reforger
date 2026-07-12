// Smart Forge rules (T-068.10; picker UX T-068.10.3) — pure-module tests: option building
// (abstract filter / sort / search / grouping / stranded-retain), data-driven validation,
// pick↔loadout mapping, the display summary, and the F-gates against the committed
// registry envelope (counts inline from the T-068.10.2 census-gated export).

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import type { RegistryItem } from '@/types/models/registry'
import {
  EMPTY_PICKS,
  LOADOUT_ROWS,
  buildGroupedRowOptions,
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

const RIFLE_BASE = '{AA9}Prefabs/Weapons/Rifle_M16A2_base.et'

const CATALOG: RegistryItem[] = [
  item(RIFLE, 'gear_primary', 'M16A2'),
  item(RIFLE_AK, 'gear_primary', 'AK-74'),
  { ...item(RIFLE_BASE, 'gear_primary', 'Rifle M16A2 base'), abstract: true },
  item(ACOG, 'optic', 'ACOG'),
  item(STANAG, 'magazine', '30rnd STANAG'),
  item(BDU, 'gear_jacket', 'BDU Woodland'),
  item(GORKA, 'gear_jacket', 'Gorka'),
]
const BY_NAME = new Map(CATALOG.map((i) => [i.resource_name, i]))

const primaryRow = LOADOUT_ROWS.find((r) => r.key === 'primary') as LoadoutRow
const uniformRow = LOADOUT_ROWS.find((r) => r.key === 'jacket') as LoadoutRow
const opticRow = LOADOUT_ROWS.find((r) => r.key === 'optic') as LoadoutRow
const magRow = LOADOUT_ROWS.find((r) => r.key === 'magazine') as LoadoutRow

const NO_COMPAT: CompatSets = { edgeItems: {} }

describe('buildRowOptions', () => {
  it('kind row lists the kind locale-sorted, None first, abstracts excluded', () => {
    const opts = buildRowOptions(primaryRow, '', NO_COMPAT, CATALOG, BY_NAME)
    expect(opts.map((o) => o.value)).toEqual(['', RIFLE_AK, RIFLE]) // AK-74 < M16A2
    expect(opts[0].label).toBe('— None —')
    expect(opts.some((o) => o.value === RIFLE_BASE)).toBe(false) // abstract template hidden
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
      RIFLE_AK,
      RIFLE,
    ])
  })

  it('an abstract pick already on the slot stays selectable (never silently blanks)', () => {
    const opts = buildRowOptions(primaryRow, RIFLE_BASE, NO_COMPAT, CATALOG, BY_NAME)
    expect(opts.some((o) => o.value === RIFLE_BASE)).toBe(true)
  })

  it('query filters by display-name substring but never removes the current pick', () => {
    const opts = buildRowOptions(primaryRow, RIFLE_AK, NO_COMPAT, CATALOG, BY_NAME, 'm16')
    expect(opts.map((o) => o.value)).toEqual(['', RIFLE_AK, RIFLE])
    const none = buildRowOptions(primaryRow, '', NO_COMPAT, CATALOG, BY_NAME, 'zzz')
    expect(none.map((o) => o.value)).toEqual([''])
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
    const labels = buildRowOptions(opticRow, '', sets, CATALOG, BY_NAME).map((o) => o.label)
    expect(labels[0]).toBe('— None —')
    expect(labels.slice(1).sort((a, b) => a.localeCompare(b))).toEqual(
      ['ACOG', unknown].sort((a, b) => a.localeCompare(b)),
    )
    expect(labels.slice(1)).toEqual([...labels.slice(1)].sort((a, b) => a.localeCompare(b)))
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

describe('buildGroupedRowOptions', () => {
  const GROUPED: RegistryItem[] = [
    { ...item(RIFLE, 'gear_primary', 'M16A2'), category: 'ArmaReforger/Weapons/Rifles/M16' },
    { ...item(RIFLE_AK, 'gear_primary', 'AK-74'), category: 'ArmaReforger/Weapons/Rifles/AK74' },
    {
      ...item('{MG1}Prefabs/Weapons/MG_M60.et', 'gear_primary', 'M60'),
      category: 'ArmaReforger/Weapons/MachineGuns/M60',
    },
  ]
  const G_BY_NAME = new Map(GROUPED.map((i) => [i.resource_name, i]))

  it('buckets by category (addon segment dropped), groups and options locale-sorted', () => {
    const g = buildGroupedRowOptions(primaryRow, '', NO_COMPAT, GROUPED, G_BY_NAME)
    expect(g.groups.map((x) => x.label)).toEqual(['Weapons/MachineGuns', 'Weapons/Rifles'])
    expect(g.groups[1].options.map((o) => o.label)).toEqual(['AK-74', 'M16A2'])
    expect(g.none.label).toBe('— None —')
    expect(g.stranded).toBeNull()
  })

  it('collapses to a single unlabeled group when every option shares a bucket', () => {
    const rifles = GROUPED.filter((i) => i.category.includes('Rifles'))
    const g = buildGroupedRowOptions(
      primaryRow,
      '',
      NO_COMPAT,
      rifles,
      new Map(rifles.map((i) => [i.resource_name, i])),
    )
    expect(g.groups).toHaveLength(1)
    expect(g.groups[0].label).toBe('')
  })

  it('reports a stranded current pick separately', () => {
    const g = buildGroupedRowOptions(primaryRow, ACOG, NO_COMPAT, GROUPED, G_BY_NAME)
    expect(g.stranded?.value).toBe(ACOG)
    expect(g.stranded?.label).toMatch(/incompatible$/)
  })
})

describe('F-gates against the committed registry envelope (T-068.10.3)', () => {
  // The census-gated T-068.10.2 export — counts here are the frozen row-freeze numbers.
  const envelope = JSON.parse(
    readFileSync(
      resolve(__dirname, '../../../../../../../packages/tbd-schema/registry/registry-items.workbench.json'),
      'utf-8',
    ),
  ) as { items: (RegistryItem & { abstract?: boolean })[] }
  const catalog = envelope.items.map((raw, idx) => ({
    ...item(raw.resource_name, raw.kind, raw.display_name),
    category: raw.category,
    abstract: raw.abstract,
    variant_of: raw.variant_of,
    sort_order: idx,
  }))
  const byName = new Map(catalog.map((i) => [i.resource_name, i]))

  it('F1: per-row option counts equal the envelope non-abstract, non-variant kind counts', () => {
    const visible = (kind: string) =>
      catalog.filter((i) => i.kind === kind && i.abstract !== true && !i.variant_of).length
    // Weapon rows collapsed by variant_of (T-068.10.5 family census keep-list).
    expect(visible('gear_primary')).toBe(21)
    expect(visible('gear_launcher')).toBe(10)
    expect(visible('gear_handgun')).toBe(2)
    expect(visible('gear_jacket')).toBe(46)
    expect(visible('gear_vest')).toBe(28)
    expect(visible('gear_helmet')).toBe(68)
    for (const [key, kind, count] of [
      ['primary', 'gear_primary', 21],
      ['launcher', 'gear_launcher', 10],
      ['handgun', 'gear_handgun', 2],
      ['throwable', 'gear_throwable', 10],
      ['jacket', 'gear_jacket', 46],
      ['vest', 'gear_vest', 28],
      ['headCover', 'gear_helmet', 68],
    ] as const) {
      const row = LOADOUT_ROWS.find((r) => r.key === key) as LoadoutRow
      const opts = buildRowOptions(row, '', NO_COMPAT, catalog, byName)
      expect(opts, `${kind} row`).toHaveLength(count + 1) // + None
    }
  })

  it('F5 (T-068.10.5): factory configurations hide behind their base weapon', () => {
    const base = catalog.find((i) => i.display_name === 'Rifle AK74N')
    const cfg = catalog.find((i) => i.display_name === 'Rifle AK74N 1P29')
    expect(base?.variant_of ?? null).toBeNull()
    expect(cfg?.variant_of).toBe(base?.resource_name)
    const opts = buildRowOptions(primaryRow, '', NO_COMPAT, catalog, byName)
    expect(opts.some((o) => o.value === base?.resource_name)).toBe(true)
    expect(opts.some((o) => o.value === cfg?.resource_name)).toBe(false)
    // PGO-7 is an optic config of the RPG-7 (operator call, census-verified)
    const rpg = catalog.find((i) => i.display_name === 'Launcher RPG7 PGO7')
    expect(rpg?.variant_of).toBeTruthy()
    // a live variant pick never blanks
    const withCurrent = buildRowOptions(
      primaryRow,
      cfg?.resource_name ?? '',
      NO_COMPAT,
      catalog,
      byName,
    )
    expect(withCurrent.some((o) => o.value === cfg?.resource_name)).toBe(true)
  })

  it('F2: options are locale-sorted (sorted copy equality)', () => {
    const opts = buildRowOptions(primaryRow, '', NO_COMPAT, catalog, byName).slice(1)
    const labels = opts.map((o) => o.label)
    expect(labels).toEqual([...labels].sort((a, b) => a.localeCompare(b)))
  })

  it('F3: abstract templates are excluded (named example + exact count)', () => {
    const opts = buildRowOptions(primaryRow, '', NO_COMPAT, catalog, byName)
    expect(opts.some((o) => o.label.endsWith(' base') || o.label.endsWith(' Base'))).toBe(false)
    const abstractPrimaries = catalog.filter(
      (i) => i.kind === 'gear_primary' && i.abstract === true,
    )
    expect(abstractPrimaries.length).toBe(26)
    expect(abstractPrimaries.some((i) => i.display_name === 'Rifle M16A2 base')).toBe(true)
  })

  it('F4: grenades and smokes are gear_throwable, absent from the primary row', () => {
    const rgd5 = catalog.find((i) => i.display_name === 'Grenade RGD5')
    const m18 = catalog.find((i) => i.display_name === 'Smoke M18 Red')
    expect(rgd5?.kind).toBe('gear_throwable')
    expect(m18?.kind).toBe('gear_throwable')
    const primaries = buildRowOptions(primaryRow, '', NO_COMPAT, catalog, byName)
    expect(primaries.some((o) => o.value === rgd5?.resource_name)).toBe(false)
    expect(primaries.some((o) => o.value === m18?.resource_name)).toBe(false)
    expect(primaries.some((o) => /smoke|grenade|pod |mortar|cannon/i.test(o.label))).toBe(false)
  })
})

describe('picks ↔ loadout mapping (SlotLoadout v2)', () => {
  it('all-empty picks map to null (clears the doc field)', () => {
    expect(picksToLoadout({ ...EMPTY_PICKS }, BY_NAME)).toBeNull()
  })

  it('round-trips picks through SlotLoadout v2 with a display summary', () => {
    const picks = { ...EMPTY_PICKS, primary: RIFLE, optic: ACOG, magazine: STANAG, jacket: BDU }
    const loadout = picksToLoadout(picks, BY_NAME)
    expect(loadout).toEqual({
      version: 2,
      wear: {
        headCover: null,
        jacket: BDU,
        pants: null,
        boots: null,
        vest: null,
        armoredVest: null,
        backpack: null,
        handwear: null,
      },
      weapons: [
        {
          slotIndex: 0,
          slotType: 'primary',
          weapon: RIFLE,
          optic: ACOG,
          magazine: STANAG,
          attachments: [],
        },
      ],
      summary: 'M16A2 · ACOG · 30rnd STANAG',
    })
    expect(loadoutToPicks(loadout ?? undefined)).toEqual(picks)
  })

  it('weapon slots are slot-indexed: launcher = 2nd primary, handgun = secondary, throwable = grenade', () => {
    const LAW = '{LLL}Prefabs/Weapons/Launchers/M72.et'
    const PM = '{PPP}Prefabs/Weapons/Handguns/PM.et'
    const RGD = '{GGG}Prefabs/Weapons/Grenades/RGD5.et'
    const picks = { ...EMPTY_PICKS, primary: RIFLE, launcher: LAW, handgun: PM, throwable: RGD }
    const loadout = picksToLoadout(picks, BY_NAME)
    expect(loadout?.weapons).toEqual([
      { slotIndex: 0, slotType: 'primary', weapon: RIFLE, optic: null, magazine: null, attachments: [] },
      { slotIndex: 1, slotType: 'primary', weapon: LAW },
      { slotIndex: 2, slotType: 'secondary', weapon: PM },
      { slotIndex: 3, slotType: 'grenade', weapon: RGD },
    ])
    expect(loadoutToPicks(loadout ?? undefined)).toEqual(picks)
  })

  it('loadoutToPicks of an unforged slot is all-empty', () => {
    expect(loadoutToPicks(undefined)).toEqual(EMPTY_PICKS)
  })

  it('summary omits empty slots and appends the 2nd weapon', () => {
    expect(buildLoadoutSummary({ ...EMPTY_PICKS, primary: RIFLE }, BY_NAME)).toBe('M16A2')
    expect(
      buildLoadoutSummary({ ...EMPTY_PICKS, primary: RIFLE, launcher: RIFLE_AK }, BY_NAME),
    ).toBe('M16A2 · AK-74')
    expect(buildLoadoutSummary({ ...EMPTY_PICKS }, BY_NAME)).toBe('')
  })
})
