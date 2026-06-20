// Normalized entity interfaces — the editor's data model (Ultra Plan §2.1).
// Principle: never nest. Every entity lives in a flat, ID-keyed dictionary;
// relationships are ID arrays. This is what lets us mutate one slot without
// re-rendering its squad, and what makes future multiplayer (ADR-3) commute.

export type ID = string

export interface MissionMeta {
  id: ID
  title: string
  terrain: 'everon' | 'arland' | 'custom'
  customTerrainName?: string
  environment: {
    time: string // "HH:MM"
    weather: 'clear' | 'overcast' | 'heavy_rain' | 'dense_fog'
    viewDistance?: number // meters; auto-derived but overridable
    thermals?: boolean
  }
}

export interface Faction {
  id: ID
  key: string // "BLUFOR" | "OPFOR" | "INDFOR" | "CIV" — export side key
  name: string // "US Army 2005"
  squadIds: ID[]
}

export interface Squad {
  id: ID
  factionId: ID
  callsign?: string // "Platoon HQ"
  name: string // "Alpha 1-1" -> exported as squad
  slotIds: ID[]
}

export interface Slot {
  id: ID
  squadId: ID
  index: number // 0-based authored order -> json_payload slot_index
  role: string // "Squad Leader"
  tag?: string // "MED" | "ENG"
  position: { x: number; y: number; z: number; rotation: number } // x/y meters, z from DEM
  stance: 'stand' | 'crouch' | 'prone'
  loadoutId: ID | null
}

export interface Loadout {
  id: ID
  containers: { uniform?: ID; vest?: ID; backpack?: ID; helmet?: ID }
  weapons: { primary?: ID; secondary?: ID; launcher?: ID }
  itemIds: ID[] // loose items (map, first-aid kit, …)
  templateKey?: string // set when applied from a mass-template
}

export interface InventoryItem {
  id: ID
  classname: string // Arma classname; the registry key
  parentId: ID | null // container nesting (a vest holding magazines)
  slotType: string // 'uniform'|'vest'|'optic'|'muzzle'|'magazine'|'item'…
  attachments: Record<string, ID | null> // 'optic'|'muzzle'|'underbarrel' -> InventoryItem
  count: number // stack size (magazines, grenades)
}

export interface Trigger {
  type: 'presence' | 'elimination' | 'timer'
  condition?: string
}

export interface Objective {
  id: ID
  type: 'attack' | 'defend' | 'capture' | 'destroy'
  factionId: ID
  position: { x: number; y: number; z: number }
  radius: number // meters
  triggers: Trigger[]
  text?: string
}

export interface Vehicle {
  id: ID
  classname: string
  factionId: ID
  position: { x: number; y: number; z: number; rotation: number }
  inventoryItemIds: ID[] // crate/cargo contents
}

export interface MapMarker {
  id: ID
  kind: 'line' | 'arrow' | 'phase' | 'icon' | 'polygon'
  points: [number, number][] // world meters
  color: string
  label?: string
  authorId?: string // for the Planner's per-user markers
}

// ── UI / runtime state (not persisted to json_payload) ──────────────────────

export type ToolId = 'select' | 'ruler' | 'los' | 'waypoint' | 'objective'

export type SelectionKind =
  | 'none'
  | 'slot'
  | 'squad'
  | 'objective'
  | 'vehicle'
  | 'marker'

export interface Selection {
  kind: SelectionKind
  id: ID | null
}

/** Names of the nine top-level entity maps — shared by the Y.Doc and the store. */
export const ENTITY_MAPS = [
  'factions',
  'squads',
  'slots',
  'loadouts',
  'items',
  'objectives',
  'vehicles',
  'markers',
] as const
