// Arsenal item-detail selector (T-068.10.6) — pure projection of one registry item for the
// ACE-style detail pane: identity, phys attrs (kg/cm³, .10.2 export), container capacity,
// variant relations (variant_of back-link + this item's configurations). No worker, no
// async — everything comes from the already-loaded registry catalog.

import type { RegistryItem } from '@/types/models/registry'

/** Kinds whose exported capacity comes from the weapon-attachment storage (rifles carry a
 *  SCR_WeaponAttachmentsStorageComponent) — not wearable cargo containers. */
const WEAPON_KINDS: readonly RegistryItem['kind'][] = [
  'gear_primary',
  'gear_launcher',
  'gear_handgun',
  'gear_throwable',
  'gear_explosive',
  'vehicle_weapon',
]

export interface ItemDetail {
  resourceName: string
  name: string
  kind: RegistryItem['kind']
  addon?: string
  abstract: boolean
  weightKg: number | null
  volumeCm3: number | null
  maxWeightKg: number | null
  maxVolumeCm3: number | null
  /** True when the item is itself a container (either capacity field exported). */
  isContainer: boolean
  /** Base weapon this item is a factory configuration of (T-068.10.5). */
  variantOf: { resourceName: string; name: string } | null
  /** Factory configurations of THIS item (reverse variant_of), locale-sorted. */
  configurations: { resourceName: string; name: string }[]
}

export function itemDetail(
  resourceName: string,
  catalog: readonly RegistryItem[],
  catalogByName: ReadonlyMap<string, RegistryItem>,
): ItemDetail | null {
  const item = catalogByName.get(resourceName)
  if (!item) return null

  const parent = item.variant_of ? catalogByName.get(item.variant_of) : undefined
  const configurations = catalog
    .filter((i) => i.variant_of === resourceName)
    .map((i) => ({ resourceName: i.resource_name, name: i.display_name }))
    .sort((a, b) => a.name.localeCompare(b.name))

  const maxWeightKg = item.max_weight_kg ?? null
  const maxVolumeCm3 = item.max_volume_cm3 ?? null
  return {
    resourceName,
    name: item.display_name,
    kind: item.kind,
    addon: item.addon ?? undefined,
    abstract: item.abstract === true,
    weightKg: item.weight_kg ?? null,
    volumeCm3: item.volume_cm3 ?? null,
    maxWeightKg,
    maxVolumeCm3,
    // Wearable/crate cargo containers only — weapon-attachment storages don't count.
    isContainer:
      (maxWeightKg !== null || maxVolumeCm3 !== null) && !WEAPON_KINDS.includes(item.kind),
    variantOf: item.variant_of
      ? {
          resourceName: item.variant_of,
          name: parent?.display_name ?? item.variant_of,
        }
      : null,
    configurations,
  }
}
