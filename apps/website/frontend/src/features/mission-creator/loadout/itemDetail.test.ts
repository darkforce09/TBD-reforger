// T-068.10.6 detail-selector gates against the committed census-gated envelope: phys
// attrs flow (kg/cm³), container detection, variant back-link + reverse configurations —
// all with real GUIDs.

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import type { RegistryItem } from '@/types/models/registry'
import { itemDetail } from './itemDetail'

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

describe('itemDetail', () => {
  it('weapons expose null phys attrs (engine class defaults are never guessed) and are not containers', () => {
    const ak = find('Rifle AK74')
    const d = itemDetail(ak.resource_name, catalog, byName)
    expect(d?.kind).toBe('gear_primary')
    // Data truth: no vanilla primary serializes Weight/ItemVolume in its prefab chain —
    // the exporter writes null rather than guessing (census U1/U5 rule).
    expect(d?.weightKg).toBeNull()
    expect(d?.volumeCm3).toBeNull()
    expect(d?.isContainer).toBe(false)
    expect(d?.addon).toBe('ArmaReforger')
  })

  it('phys attrs flow when serialized (RGD5: 0.31 kg / 100 cm³ from the prefab text)', () => {
    const rgd = find('Grenade RGD5')
    const d = itemDetail(rgd.resource_name, catalog, byName)
    expect(d?.weightKg).toBeCloseTo(0.31, 2)
    expect(d?.volumeCm3).toBe(100)
  })

  it('detects containers via exported capacity (a concrete backpack has max fields)', () => {
    const packs = catalog.filter(
      (i) => i.kind === 'gear_backpack' && i.abstract !== true && i.max_volume_cm3 != null,
    )
    expect(packs.length).toBeGreaterThan(0)
    const d = itemDetail(packs[0].resource_name, catalog, byName)
    expect(d?.isContainer).toBe(true)
    expect(d?.maxVolumeCm3).toBeGreaterThan(0)
  })

  it('variant back-link: AK74N 1P29 → configuration of AK74N', () => {
    const cfg = find('Rifle AK74N 1P29')
    const base = find('Rifle AK74N')
    const d = itemDetail(cfg.resource_name, catalog, byName)
    expect(d?.variantOf?.resourceName).toBe(base.resource_name)
    expect(d?.variantOf?.name).toBe('Rifle AK74N')
  })

  it('reverse configurations: AK74N lists its factory configs, locale-sorted', () => {
    const base = find('Rifle AK74N')
    const d = itemDetail(base.resource_name, catalog, byName)
    const names = d?.configurations.map((c) => c.name) ?? []
    expect(names).toContain('Rifle AK74N 1P29')
    expect(names).toContain('Rifle AK74N GP25')
    expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b)))
  })

  it('unknown resource → null; abstract flag surfaces', () => {
    expect(itemDetail('{0000000000000000}Prefabs/Nope.et', catalog, byName)).toBeNull()
    const ab = must(catalog.find((i) => i.abstract === true), 'any abstract row')
    expect(itemDetail(ab.resource_name, catalog, byName)?.abstract).toBe(true)
  })
})
