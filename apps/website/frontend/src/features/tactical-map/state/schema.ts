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
    showGrid?: boolean // procedural grid overlay — default true when undefined (T-091.2)
    showHillshade?: boolean // DEM hillshade overlay — default false when undefined (T-091.2)
  }
}

export interface Faction {
  id: ID
  key: string // "BLUFOR" | "OPFOR" | "INDFOR" | "CIV" — export side key
  name: string // "US Army 2005"
  squadIds: ID[]
}

// An EditorLayer is a purely-organizational folder in the Left "Placed Entities"
// tree — the Eden Editor paradigm (Ultra Plan §5.1). Users create arbitrary nested
// folders to group entities; layers are workflow-only and DO NOT affect the exported
// mission (the compiler reads factions/squads/slots, not layers). Nesting is by
// `parentId` (null = root); `entityIds` lists the slots/vehicles/markers placed
// directly in this folder.
export interface EditorLayer {
  id: ID
  name: string
  parentId: ID | null
  entityIds: ID[]
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
  // Registry resource_name from the palette drop — the full Enfusion ResourceName,
  // e.g. {GUID}Prefabs/.../File.et (not a mock id, not a "classname").
  assetId?: string
  position: { x: number; y: number; z: number; rotation: number } // x/y meters, z from DEM
  stance: 'stand' | 'crouch' | 'prone'
  loadoutId: ID | null
}

// A slot snapshot held on the editor clipboard (Ctrl+C, T-056). Plain/serializable —
// it carries no id; pasteSlots() mints fresh ids and re-resolves the squad/layer so a
// paste re-attaches to the source squad (or the default) and files into the active folder.
export interface ClipboardSlot {
  role: string
  tag?: string
  // Same registry resource_name as Slot.assetId — copy/paste (T-056) preserves the full
  // Enfusion ResourceName ({GUID}Prefabs/.../File.et) carried from the palette drop.
  assetId?: string
  stance: Slot['stance']
  position: { x: number; y: number; z: number; rotation: number }
  squadId: ID // source squad, re-attached on paste if it still exists
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

export type ToolId =
  | 'select'
  | 'place' // place a unit/slot on the map (Phase 3)
  | 'ruler'
  | 'los'
  | 'waypoint'
  | 'objective'

export type SelectionKind =
  | 'none'
  | 'slot'
  | 'squad'
  | 'objective'
  | 'vehicle'
  | 'marker'

export interface Selection {
  kind: SelectionKind
  /** Selected entity ids (multi-select via marquee, Phase 7b). Empty when kind==='none'. */
  ids: ID[]
}

/** Names of the top-level entity maps — shared by the Y.Doc and the store. */
export const ENTITY_MAPS = [
  'factions',
  'squads',
  'slots',
  'loadouts',
  'items',
  'objectives',
  'vehicles',
  'markers',
  'editorLayers',
] as const
