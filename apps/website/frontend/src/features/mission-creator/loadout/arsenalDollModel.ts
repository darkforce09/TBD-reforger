// Paper-doll model (T-068.10.7) — pure, worker/React-free (arsenalRules.ts style). Owns the
// silhouette region config (every pickable LoadoutKey exactly one hotspot — completeness is
// vitest-asserted against EMPTY_PICKS) and the honest weight readout math: null weight_kg is
// an engine class default the export never guessed, so it counts as unknown, never as 0-known.

import type { RegistryItem } from '@/types/models/registry'
import type { LoadoutKey } from './arsenalRules'

/** One clickable silhouette region. `kind` selects the hotspot styling family. */
export interface DollRegion {
  key: LoadoutKey
  label: string
  kind: 'wear' | 'weapon'
}

/**
 * The main hotspots, in tab order (head-to-toe wear, then the carried weapons). optic and
 * magazine are deliberately NOT regions — they ride the rifle as PRIMARY_SUB_REGIONS.
 */
export const DOLL_REGIONS: readonly DollRegion[] = [
  { key: 'headCover', label: 'Helmet', kind: 'wear' },
  { key: 'jacket', label: 'Jacket', kind: 'wear' },
  { key: 'vest', label: 'Vest', kind: 'wear' },
  { key: 'armoredVest', label: 'Armored vest', kind: 'wear' },
  { key: 'backpack', label: 'Backpack', kind: 'wear' },
  { key: 'handwear', label: 'Gloves', kind: 'wear' },
  { key: 'pants', label: 'Pants', kind: 'wear' },
  { key: 'boots', label: 'Boots', kind: 'wear' },
  { key: 'primary', label: 'Primary', kind: 'weapon' },
  { key: 'launcher', label: 'Launcher', kind: 'weapon' },
  { key: 'handgun', label: 'Handgun', kind: 'weapon' },
  { key: 'throwable', label: 'Throwable', kind: 'weapon' },
] as const

/** The rifle's sub-hotspots — weapons[0] attachment picks (the arsenalRules edge rows). */
export const PRIMARY_SUB_REGIONS: readonly LoadoutKey[] = ['optic', 'magazine'] as const

export interface LoadoutWeight {
  /** Sum of the equipped items' serialized weight_kg. */
  knownKg: number
  /** Equipped items whose weight the registry does not serialize (engine default). */
  unknownCount: number
  /** All equipped (non-empty) picks. */
  itemCount: number
}

/** Honest loadout weight: sums what the registry serializes, counts what it doesn't. */
export function loadoutWeight(
  picks: Record<LoadoutKey, string>,
  catalogByName: ReadonlyMap<string, RegistryItem>,
): LoadoutWeight {
  let knownKg = 0
  let unknownCount = 0
  let itemCount = 0
  for (const rn of Object.values(picks)) {
    if (!rn) continue
    itemCount += 1
    const w = catalogByName.get(rn)?.weight_kg
    if (typeof w === 'number') knownKg += w
    else unknownCount += 1
  }
  return { knownKg, unknownCount, itemCount }
}

/** The weight readout string ("≥ 0.3 kg · 1 item without weight data" / "0.3 kg · 2 items"). */
export function formatLoadoutWeight(w: LoadoutWeight): string {
  const kg = w.knownKg.toLocaleString(undefined, { maximumFractionDigits: 1 })
  if (w.unknownCount > 0) {
    return `≥ ${kg} kg · ${w.unknownCount} item${w.unknownCount === 1 ? '' : 's'} without weight data`
  }
  return `${kg} kg · ${w.itemCount} item${w.itemCount === 1 ? '' : 's'}`
}
