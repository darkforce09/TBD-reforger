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
    hillshadeOpacity?: number // hillshade blend strength 0–1; default 0.4 when undefined (T-090.1.2.6)
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

// Per-slot Smart Forge picks — v1 (T-068.10): ACE-shaped fixed fields. Persisted verbatim in
// the doc, so it rides Save Version (`editor.slots`), mission Export, IDB, and copy/paste.
// Distinct from the legacy `Slot.loadoutId` template ref. Existing docs keep v1 shapes; the
// editor migrates to v2 on read (migrateLoadout) and writes v2 only.
export interface SlotLoadoutV1 {
  primary: string | null
  uniform: string | null
  vest: string | null
  helmet: string | null
  optic: string | null // attaches to primary via optic_on_weapon
  magazine: string | null // loads into primary via mag_in_weapon
  // Display summary from registry display_names ("M16A2 · ACOG"), built at pick time —
  // feeds the compiled orbat[].slots[].loadout string (T-068.11 alignment).
  summary?: string
}

// One slot-indexed weapon (T-068.10.4). Vanilla characters carry two UNTYPED "primary"
// slots (two rifles legal; a launcher is just a weapon in the second one), a "secondary"
// pistol slot and grenade/throwable slots — Character_Base.et evidence in the T-068.10.2
// census. T-068.12 must equip via slot-indexed SetWeapon, not blind EquipWeapon.
export interface LoadoutWeapon {
  slotIndex: number
  slotType: string // "primary" | "secondary" | "grenade" | "throwable" (engine strings)
  weapon: string
  optic?: string | null // optic_on_weapon-validated (weapons[0] only until the attachments slice)
  magazine?: string | null // mag_in_weapon-validated (weapons[0] only until the attachments slice)
  attachments?: string[]
}

// v2 (T-068.10.4): Reforger-shaped loadout. `wear` is an OPEN map keyed by engine
// LoadoutSlotInfo name — canonical keys headCover/jacket/pants/boots/vest/armoredVest/
// backpack/handwear (both vests are separate simultaneous slots!); mod-added areas are
// representable without a schema change. equipment/cargo are forward skeletons (their UI
// lands in later slices). Mirrors loadout-export.schema.json v2.
export interface SlotLoadoutV2 {
  version: 2
  wear: Record<string, string | null>
  weapons: LoadoutWeapon[]
  equipment?: Record<string, string | null>
  cargo?: { container: string; item: string; qty: number }[]
  summary?: string
}

// The doc field holds either shape (old missions load untouched); pickers and export always
// go through migrateLoadout → v2.
export type SlotLoadout = SlotLoadoutV1 | SlotLoadoutV2

export function isLoadoutV2(l: SlotLoadout | undefined): l is SlotLoadoutV2 {
  return !!l && (l as SlotLoadoutV2).version === 2
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
  loadout?: SlotLoadout // Smart Forge picks (T-068.10); omitted until first forged
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
  loadout?: SlotLoadout // Smart Forge picks travel with the copy (T-068.10)
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

export type SelectionKind = 'none' | 'slot' | 'squad' | 'objective' | 'vehicle' | 'marker'

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
