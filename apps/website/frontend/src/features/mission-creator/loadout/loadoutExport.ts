// Loadout export (T-068.4; smart slots T-068.10; v2 doc T-068.10.4) — per-slot Forge picks →
// loadout-export.json. Shape mirrors packages/tbd-schema/schema/loadout-export.schema.json v2:
// wear (open map keyed by engine LoadoutSlotInfo name), slot-indexed weapons, equipment/cargo
// skeletons, PLUS a derived legacy `gear` block — the v1 mod reader (TBD_LoadoutEquipComponent,
// JsonLoadContext ignores unknown fields — U6) keeps dressing the Phase-1 test NPC until
// T-068.12 reads the v2 fields natively.

import type { LoadoutWeapon, SlotLoadoutV2 } from '@/features/tactical-map'

export type GearSlot = string | null

/**
 * The legacy v1 gear slots (each a registry resource_name or null). In v2 exports this block
 * is DERIVED: jacket→uniform, armoredVest (else vest)→vest, headCover→helmet, weapons slot 0
 * →primary/optic/magazine. The armored vest wins the single legacy vest field because it is
 * the visually dominant one on the dressed NPC.
 *
 * @contract loadout-export.schema.json#/$defs/gear
 */
export interface LoadoutGear {
  primary: GearSlot
  uniform: GearSlot
  vest: GearSlot
  helmet: GearSlot
  optic: GearSlot
  magazine: GearSlot
}

/**
 * The v2 loadout handoff written as loadout-export.json.
 *
 * @contract loadout-export.schema.json#/
 */
export interface LoadoutExport {
  loadoutVersion: '2'
  modpackId: string
  wear: Record<string, GearSlot>
  weapons: LoadoutWeapon[]
  equipment: Record<string, GearSlot>
  cargo: { container: string; item: string; qty: number }[]
  gear: LoadoutGear
}

/** Derive the legacy v1 gear block from a v2 loadout (mod back-compat until T-068.12). */
export function slotLoadoutToGear(loadout: SlotLoadoutV2 | undefined): LoadoutGear {
  const wear = loadout?.wear ?? {}
  const w0 = loadout?.weapons.find((w) => w.slotIndex === 0 && w.slotType === 'primary')
  const pick = (v: string | null | undefined): GearSlot => v ?? null
  return {
    primary: pick(w0?.weapon),
    uniform: pick(wear.jacket),
    vest: pick(wear.armoredVest ?? wear.vest),
    helmet: pick(wear.headCover),
    optic: pick(w0?.optic),
    magazine: pick(w0?.magazine),
  }
}

/** Build the v2 export object from the doc's (already migrated) v2 loadout. */
export function buildLoadoutExport(
  loadout: SlotLoadoutV2 | undefined,
  modpackId: string,
): LoadoutExport {
  return {
    loadoutVersion: '2',
    modpackId,
    wear: { ...(loadout?.wear ?? {}) },
    weapons: loadout?.weapons ?? [],
    equipment: { ...(loadout?.equipment ?? {}) },
    cargo: [...(loadout?.cargo ?? [])],
    gear: slotLoadoutToGear(loadout),
  }
}

/** Trigger a browser download of loadout-export.json for the given payload. */
export function downloadLoadoutJson(payload: LoadoutExport): void {
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'loadout-export.json'
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
}
