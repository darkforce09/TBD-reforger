// SlotLoadout v1 → v2 migration (T-068.10.4). AreaType-aware: the v1 `vest` field held BOTH
// Reforger vest areas (chest rigs AND armored vests were one `gear_vest` kind before the
// T-068.10.2 reclassify), so the item is looked up in the registry and routed to
// `wear.armoredVest` vs `wear.vest` by its v3 kind — never string-guessed. Old docs keep
// their v1 shapes on disk; the editor migrates on read and writes v2 only.

import type { SlotLoadout, SlotLoadoutV2 } from '@/features/tactical-map'
import type { RegistryItem } from '@/types/models/registry'

// Local guard (type-only imports keep this module runtime-free of the tactical-map barrel,
// which node-env vitest cannot resolve at runtime).
function isV2(l: SlotLoadout): l is SlotLoadoutV2 {
  return (l as SlotLoadoutV2).version === 2
}

/** Canonical wear keys (engine LoadoutSlotInfo names on Character_Base.et). The map is
 *  open — mod-added areas ride along — but the editor UI only surfaces these. */
export const WEAR_KEYS = [
  'headCover',
  'jacket',
  'pants',
  'boots',
  'vest',
  'armoredVest',
  'backpack',
  'handwear',
] as const

const EMPTY_WEAR: Record<string, string | null> = Object.fromEntries(
  WEAR_KEYS.map((k) => [k, null]),
)

/** v1 or v2 (or unforged) → v2 (or undefined). Pure; safe on every doc revision. */
export function migrateLoadout(
  loadout: SlotLoadout | undefined,
  catalogByName: ReadonlyMap<string, RegistryItem>,
): SlotLoadoutV2 | undefined {
  if (!loadout) return undefined
  if (isV2(loadout)) return loadout

  const wear: Record<string, string | null> = { ...EMPTY_WEAR }
  wear.jacket = loadout.uniform ?? null
  wear.headCover = loadout.helmet ?? null
  if (loadout.vest) {
    // AreaType-aware routing: armored vests moved to their own kind in T-068.10.2.
    const kind = catalogByName.get(loadout.vest)?.kind
    if (kind === 'gear_armored_vest') wear.armoredVest = loadout.vest
    else wear.vest = loadout.vest
  }

  const weapons: SlotLoadoutV2['weapons'] = []
  if (loadout.primary) {
    weapons.push({
      slotIndex: 0,
      slotType: 'primary',
      weapon: loadout.primary,
      optic: loadout.optic ?? null,
      magazine: loadout.magazine ?? null,
      attachments: [],
    })
  }

  return {
    version: 2,
    wear,
    weapons,
    ...(loadout.summary ? { summary: loadout.summary } : {}),
  }
}
