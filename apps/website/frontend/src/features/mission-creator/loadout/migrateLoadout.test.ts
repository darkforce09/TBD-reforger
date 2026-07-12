// SlotLoadout v1 → v2 migration (T-068.10.4) — L2/L3 gates. Uses the REAL resource_names
// and v3 kinds from the committed census-gated envelope (the USSR rifleman two-vest kit is
// exactly the case the v1 document could not express).

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import type { SlotLoadoutV1 } from '@/features/tactical-map'
import type { RegistryItem } from '@/types/models/registry'
import { loadoutToPicks, picksToLoadout } from './arsenalRules'
import { buildLoadoutExport, slotLoadoutToGear } from './loadoutExport'
import { migrateLoadout } from './migrateLoadout'

const envelope = JSON.parse(
  readFileSync(
    resolve(
      __dirname,
      '../../../../../../../packages/tbd-schema/registry/registry-items.workbench.json',
    ),
    'utf-8',
  ),
) as { items: RegistryItem[] }
const byName = new Map(envelope.items.map((i) => [i.resource_name, i]))

// Real GUIDs from the envelope (asserted below so drift fails loudly).
const AK74 = '{FA5C25BF66A53DCF}Prefabs/Weapons/Rifles/AK74/Rifle_AK74.et'
const VEST_6B2 = '{ADE19B33DCBB9005}Prefabs/Characters/Vests/Vest_6B2/Vest_6B2.et'
const VEST_LIFCHIK = '{9713FE6DDCC9510D}Prefabs/Characters/Vests/Vest_Lifchik/Vest_Lifchik.et'
const JACKET_M88 = '{9F546CCA2582D16F}Prefabs/Characters/Uniforms/Jacket_M88.et'

describe('migrateLoadout (v1 → v2, AreaType-aware)', () => {
  it('sanity: the envelope classifies the fixture kit as expected (v3 kinds)', () => {
    expect(byName.get(AK74)?.kind).toBe('gear_primary')
    expect(byName.get(VEST_6B2)?.kind).toBe('gear_armored_vest')
    expect(byName.get(VEST_LIFCHIK)?.kind).toBe('gear_vest')
    expect(byName.get(JACKET_M88)?.kind).toBe('gear_jacket')
  })

  it('L2: v1 vest routes by registry kind — 6B2 → wear.armoredVest, Lifchik → wear.vest', () => {
    const v1a: SlotLoadoutV1 = {
      primary: AK74,
      uniform: JACKET_M88,
      vest: VEST_6B2,
      helmet: null,
      optic: null,
      magazine: null,
    }
    const a = migrateLoadout(v1a, byName)
    expect(a?.wear.armoredVest).toBe(VEST_6B2)
    expect(a?.wear.vest).toBeNull()

    const b = migrateLoadout({ ...v1a, vest: VEST_LIFCHIK }, byName)
    expect(b?.wear.vest).toBe(VEST_LIFCHIK)
    expect(b?.wear.armoredVest).toBeNull()
  })

  it('L2b: v2 expresses the full USSR rifleman default (BOTH vests at once) — inexpressible in v1', () => {
    const picks = {
      ...loadoutToPicks(undefined),
      primary: AK74,
      jacket: JACKET_M88,
      vest: VEST_LIFCHIK,
      armoredVest: VEST_6B2,
    }
    const v2 = picksToLoadout(picks, byName)
    expect(v2?.wear.vest).toBe(VEST_LIFCHIK)
    expect(v2?.wear.armoredVest).toBe(VEST_6B2)
  })

  it('L3: v1 doc → migrate → picks → picksToLoadout resolves the same equip set', () => {
    const v1: SlotLoadoutV1 = {
      primary: AK74,
      uniform: JACKET_M88,
      vest: VEST_6B2,
      helmet: null,
      optic: null,
      magazine: null,
      summary: 'AK-74',
    }
    const migrated = migrateLoadout(v1, byName)
    const roundTripped = picksToLoadout(loadoutToPicks(migrated), byName)
    expect(roundTripped?.wear).toEqual(migrated?.wear)
    expect(roundTripped?.weapons).toEqual(migrated?.weapons)
  })

  it('passes v2 docs through untouched and undefined stays undefined', () => {
    const v2 = picksToLoadout({ ...loadoutToPicks(undefined), primary: AK74 }, byName)
    expect(migrateLoadout(v2 ?? undefined, byName)).toBe(v2)
    expect(migrateLoadout(undefined, byName)).toBeUndefined()
  })

  it('unknown vest item (not in registry) defaults to wear.vest — never guessed as armored', () => {
    const ghost = '{0000000000000000}Prefabs/Characters/Vests/Vest_Modded.et'
    const m = migrateLoadout(
      { primary: null, uniform: null, vest: ghost, helmet: null, optic: null, magazine: null },
      byName,
    )
    expect(m?.wear.vest).toBe(ghost)
    expect(m?.wear.armoredVest).toBeNull()
  })
})

describe('loadout-export v2 (derived legacy gear — U6 single-file emission)', () => {
  it('derives the v1 gear block: armored vest wins the legacy vest field', () => {
    const picks = {
      ...loadoutToPicks(undefined),
      primary: AK74,
      jacket: JACKET_M88,
      vest: VEST_LIFCHIK,
      armoredVest: VEST_6B2,
    }
    const v2 = picksToLoadout(picks, byName)
    const gear = slotLoadoutToGear(v2 ?? undefined)
    expect(gear).toEqual({
      primary: AK74,
      uniform: JACKET_M88,
      vest: VEST_6B2, // armoredVest ?? vest
      helmet: null,
      optic: null,
      magazine: null,
    })
  })

  it('builds a schema-valid v2 export envelope (loadoutVersion 2 + wear + weapons + gear)', () => {
    const picks = { ...loadoutToPicks(undefined), primary: AK74, armoredVest: VEST_6B2 }
    const v2 = picksToLoadout(picks, byName)
    const exp = buildLoadoutExport(v2 ?? undefined, 'mp-1')
    expect(exp.loadoutVersion).toBe('2')
    expect(exp.modpackId).toBe('mp-1')
    expect(exp.wear.armoredVest).toBe(VEST_6B2)
    expect(exp.weapons[0]?.weapon).toBe(AK74)
    expect(exp.gear.primary).toBe(AK74)
    expect(exp.gear.vest).toBe(VEST_6B2)
    expect(exp.equipment).toEqual({})
    expect(exp.cargo).toEqual([])
  })
})
