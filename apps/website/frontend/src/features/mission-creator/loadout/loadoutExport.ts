// Dumb loadout export (T-068.4) — flat gear pick → loadout-export.json.
// Shape mirrors packages/tbd-schema/schema/loadout-export.schema.json. No smart
// equip rules / paper-doll (smart Forge = T-068.10); this is the Phase 1 file handoff.

export type GearSlot = string | null

export interface LoadoutGear {
  primary: GearSlot
  uniform: GearSlot
  vest: GearSlot
  helmet: GearSlot
}

export interface LoadoutExport {
  loadoutVersion: '1'
  modpackId: string
  gear: LoadoutGear
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
