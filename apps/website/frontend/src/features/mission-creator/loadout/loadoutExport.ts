// Loadout export (T-068.4, smart slots T-068.10) — per-slot Forge picks → loadout-export.json.
// Shape mirrors packages/tbd-schema/schema/loadout-export.schema.json. optic/magazine are the
// Smart Forge attach slots (validated against the T-068.9 compat graph before download); the
// v1 mod reader (TBD_LoadoutEquipComponent) ignores them until T-068.12.

import type { SlotLoadout } from '@/features/tactical-map'

export type GearSlot = string | null

/**
 * The exported gear slots (each a registry resource_name or null). optic/magazine are
 * schema-optional but always emitted by the web download (null when empty).
 *
 * @contract loadout-export.schema.json#/properties/gear
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
 * The flat loadout handoff written as loadout-export.json (Phase 1 file handoff to the mod,
 * read by TBD_LoadoutEquipComponent).
 *
 * @contract loadout-export.schema.json#/
 */
export interface LoadoutExport {
  loadoutVersion: '1'
  modpackId: string
  gear: LoadoutGear
}

/** Project the doc's per-slot loadout onto the export gear shape (drops the display summary). */
export function slotLoadoutToGear(loadout: SlotLoadout | undefined): LoadoutGear {
  return {
    primary: loadout?.primary ?? null,
    uniform: loadout?.uniform ?? null,
    vest: loadout?.vest ?? null,
    helmet: loadout?.helmet ?? null,
    optic: loadout?.optic ?? null,
    magazine: loadout?.magazine ?? null,
  }
}

/** Build the export object. Each gear value is a registry resource_name or null. */
export function buildLoadoutExport(gear: LoadoutGear, modpackId: string): LoadoutExport {
  return { loadoutVersion: '1', modpackId, gear }
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
