// Smart Forge rules (T-068.10) — pure, worker/React-free (mirrors registryGraph.ts style so it
// vitests in the node environment). Owns the declarative row config, option building, validation,
// and the display summary. The Arsenal UI is a dumb render loop over LOADOUT_ROWS: adding a gear
// slot later (backpack, handgun, launcher) is one config row + one SlotLoadout field — no new
// component or validation code.

import type { LoadoutWeapon, SlotLoadoutV2 } from '@/features/tactical-map'
import type {
  RegistryCompatEdgeType,
  RegistryItem,
  RegistryItemKind,
} from '@/types/models/registry'

/**
 * The pickable Forge fields (T-068.10.4, SlotLoadout v2): four weapon slots (mirroring
 * Character_Base.et — two untyped primaries, a secondary, a grenade slot), weapons[0]'s
 * optic/magazine, and the canonical wear areas. Callers migrate v1 docs via migrateLoadout
 * before mapping to picks.
 */
export type LoadoutKey =
  | 'primary'
  | 'launcher'
  | 'handgun'
  | 'throwable'
  | 'optic'
  | 'magazine'
  | 'headCover'
  | 'jacket'
  | 'pants'
  | 'boots'
  | 'vest'
  | 'armoredVest'
  | 'backpack'
  | 'handwear'

/** Pick key → weapons[] slot identity (engine slot indexes/types on Character_Base.et). */
export const WEAPON_SLOTS: readonly { key: LoadoutKey; slotIndex: number; slotType: string }[] = [
  { key: 'primary', slotIndex: 0, slotType: 'primary' },
  { key: 'launcher', slotIndex: 1, slotType: 'primary' }, // 2nd untyped primary slot
  { key: 'handgun', slotIndex: 2, slotType: 'secondary' },
  { key: 'throwable', slotIndex: 3, slotType: 'grenade' },
] as const

/**
 * Where a row's options come from:
 * - `kind`: the flat registry catalog filtered by item kind — never compat-constrained
 *   (clothing mix-and-match is a feature; T-068.10.1 operator call).
 * - `edge`: the compat graph via `itemsFor(picks[dependsOn], edge)` — empty until the
 *   dependency is picked. Compatibility only constrains the weapon families.
 */
export type LoadoutRowSource =
  | { type: 'kind'; kind: RegistryItemKind }
  | { type: 'edge'; edge: RegistryCompatEdgeType; dependsOn: LoadoutKey }

export interface LoadoutRow {
  key: LoadoutKey
  label: string
  source: LoadoutRowSource
}

/**
 * The Forge rows, in render order (T-068.10.4, SlotLoadout v2). Weapons first — four engine
 * slots incl. the 2nd untyped primary ("Launcher / 2nd rifle": Character_USSR_LAT.et carries
 * AK74 + RPG22 in the two primary slots) — then the wear areas, incl. BOTH simultaneous vest
 * slots. No ammo row — `ammo_in_mag` ships no edges (engine `.conf` linkage, T-150 OPEN).
 * optic/magazine stay bound to weapons[0] until the attachments slice.
 */
export const LOADOUT_ROWS: readonly LoadoutRow[] = [
  { key: 'primary', label: 'Primary', source: { type: 'kind', kind: 'gear_primary' } },
  {
    key: 'optic',
    label: 'Optic',
    source: { type: 'edge', edge: 'optic_on_weapon', dependsOn: 'primary' },
  },
  {
    key: 'magazine',
    label: 'Magazine',
    source: { type: 'edge', edge: 'mag_in_weapon', dependsOn: 'primary' },
  },
  {
    key: 'launcher',
    label: 'Launcher / 2nd rifle',
    source: { type: 'kind', kind: 'gear_launcher' },
  },
  { key: 'handgun', label: 'Handgun', source: { type: 'kind', kind: 'gear_handgun' } },
  { key: 'throwable', label: 'Throwable', source: { type: 'kind', kind: 'gear_throwable' } },
  { key: 'headCover', label: 'Helmet', source: { type: 'kind', kind: 'gear_helmet' } },
  { key: 'jacket', label: 'Jacket', source: { type: 'kind', kind: 'gear_jacket' } },
  { key: 'pants', label: 'Pants', source: { type: 'kind', kind: 'gear_pants' } },
  { key: 'boots', label: 'Boots', source: { type: 'kind', kind: 'gear_boots' } },
  { key: 'vest', label: 'Vest (chest rig)', source: { type: 'kind', kind: 'gear_vest' } },
  {
    key: 'armoredVest',
    label: 'Armored vest',
    source: { type: 'kind', kind: 'gear_armored_vest' },
  },
  { key: 'backpack', label: 'Backpack', source: { type: 'kind', kind: 'gear_backpack' } },
  { key: 'handwear', label: 'Gloves', source: { type: 'kind', kind: 'gear_gloves' } },
] as const

/** v3 kinds still waiting on their own slice (equipment micro-slots / cargo / unknown engine
 *  slot name for glasses) — surfaced as a hint in the tab. */
export const PENDING_V2_KINDS: readonly RegistryItemKind[] = [
  'gear_glasses',
  'gear_binoculars',
  'gear_item',
] as const

/** The all-empty pick set (a slot that has never been forged). */
export const EMPTY_PICKS: Record<LoadoutKey, string> = {
  primary: '',
  launcher: '',
  handgun: '',
  throwable: '',
  optic: '',
  magazine: '',
  headCover: '',
  jacket: '',
  pants: '',
  boots: '',
  vest: '',
  armoredVest: '',
  backpack: '',
  handwear: '',
}

/** The wear-map key for each wear row (pick key == canonical LoadoutSlotInfo name). */
const WEAR_PICK_KEYS: readonly LoadoutKey[] = [
  'headCover',
  'jacket',
  'pants',
  'boots',
  'vest',
  'armoredVest',
  'backpack',
  'handwear',
] as const

/** Compat context for option building + validation: the edge feeds (`itemsFor` results),
 *  keyed by row key. Kind rows are never compat-constrained. */
export interface CompatSets {
  edgeItems: Partial<Record<LoadoutKey, readonly string[]>>
}

export interface PickOption {
  value: string
  label: string
}

const NONE_OPTION: PickOption = { value: '', label: '— None —' }

/** Suffix marking a selected value the compat graph rejects (stranded pick — e.g. an optic kept
 *  after its weapon changed). The option stays listed so a native <select> never silently blanks. */
const INCOMPAT_SUFFIX = ' — incompatible'

function displayName(
  resourceName: string,
  catalogByName: ReadonlyMap<string, RegistryItem>,
): string {
  return catalogByName.get(resourceName)?.display_name ?? resourceName
}

/**
 * The compat verdict for one row: the set of allowed values, or `null` when the graph has no
 * opinion (→ full catalog, no validation). Kind rows are always `null` — any character wears
 * any clothing (mix-and-match is a feature). Edge rows: exactly the dependency's `itemsFor`
 * feed — never null; an empty feed means the host truly accepts nothing.
 */
export function resolveRowAllowed(row: LoadoutRow, sets: CompatSets): Set<string> | null {
  if (row.source.type === 'edge') return new Set(sets.edgeItems[row.key] ?? [])
  return null
}

/**
 * The pickable resource_names for a row (T-068.10.3 semantics): kind rows are the kind
 * catalog with `abstract` template prefabs excluded; edge rows are the compat feed, also
 * abstract-filtered (magazine feeds contain `* Base` templates). Locale-alpha sorted by
 * display name. `query` narrows by case-insensitive display-name substring — but never
 * removes the currently selected value (a filter must not blank a live pick).
 */
function rowValues(
  row: LoadoutRow,
  current: string,
  sets: CompatSets,
  catalog: readonly RegistryItem[],
  catalogByName: ReadonlyMap<string, RegistryItem>,
  query = '',
): string[] {
  const src = row.source
  const raw =
    src.type === 'kind'
      ? catalog.filter((i) => i.kind === src.kind).map((i) => i.resource_name)
      : [...(sets.edgeItems[row.key] ?? [])]
  const q = query.trim().toLowerCase()
  const values = raw.filter((v) => {
    const item = catalogByName.get(v)
    // Abstract templates AND factory variants (T-068.10.5: variant_of set — attachment/camo
    // configurations of a base weapon) hide from pickers; a live pick never blanks.
    if (item?.abstract === true || item?.variant_of) return v === current
    if (q && v !== current && !displayName(v, catalogByName).toLowerCase().includes(q))
      return false
    return true
  })
  return values.sort((a, b) =>
    displayName(a, catalogByName).localeCompare(displayName(b, catalogByName)),
  )
}

/**
 * Options for one row. Kind rows: the abstract-filtered kind catalog. Edge rows: the compat
 * feed. Locale-sorted; retains a stranded non-empty `current` at the end, suffixed
 * incompatible, so a native <select> never silently blanks.
 */
export function buildRowOptions(
  row: LoadoutRow,
  current: string,
  sets: CompatSets,
  catalog: readonly RegistryItem[],
  catalogByName: ReadonlyMap<string, RegistryItem>,
  query = '',
): PickOption[] {
  const values = rowValues(row, current, sets, catalog, catalogByName, query)
  const options = [
    NONE_OPTION,
    ...values.map((v) => ({ value: v, label: displayName(v, catalogByName) })),
  ]
  if (current && !values.includes(current)) {
    options.push({ value: current, label: displayName(current, catalogByName) + INCOMPAT_SUFFIX })
  }
  return options
}

export interface PickOptionGroup {
  label: string
  options: PickOption[]
}

/** Optgroup label from a browse category: drop the addon segment, keep the next two
 *  ("ArmaReforger/Weapons/Rifles/M16" → "Weapons/Rifles"). */
function groupLabel(item: RegistryItem | undefined): string {
  if (!item) return 'Other'
  const segs = item.category.split('/')
  return segs.slice(1, 3).join('/') || segs[0] || 'Other'
}

/**
 * Grouped options for the native <select> (T-068.10.3): options bucketed by category-derived
 * optgroups, groups and options both locale-sorted. Collapses to a single unlabeled group
 * when everything shares one bucket (no pointless optgroup chrome). The leading None option
 * and any stranded current pick are returned separately (rendered outside the groups).
 */
export function buildGroupedRowOptions(
  row: LoadoutRow,
  current: string,
  sets: CompatSets,
  catalog: readonly RegistryItem[],
  catalogByName: ReadonlyMap<string, RegistryItem>,
  query = '',
): { none: PickOption; groups: PickOptionGroup[]; stranded: PickOption | null } {
  const values = rowValues(row, current, sets, catalog, catalogByName, query)
  const buckets = new Map<string, PickOption[]>()
  for (const v of values) {
    const label = groupLabel(catalogByName.get(v))
    const bucket = buckets.get(label) ?? []
    bucket.push({ value: v, label: displayName(v, catalogByName) })
    buckets.set(label, bucket)
  }
  let groups = [...buckets.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([label, options]) => ({ label, options }))
  if (groups.length === 1) groups = [{ label: '', options: groups[0].options }]
  const stranded =
    current && !values.includes(current)
      ? { value: current, label: displayName(current, catalogByName) + INCOMPAT_SUFFIX }
      : null
  return { none: NONE_OPTION, groups, stranded }
}

export interface LoadoutValidation {
  valid: boolean
  /** Human-readable reason per invalid row key. */
  errors: Partial<Record<LoadoutKey, string>>
}

/**
 * Validate the pick set against the compat context, data-driven off LOADOUT_ROWS. Empty picks
 * are always valid; kind picks (clothing) are never invalid. Edge picks fail when their
 * dependency is empty or the compat feed rejects them.
 */
export function validateLoadout(
  picks: Record<LoadoutKey, string>,
  sets: CompatSets,
): LoadoutValidation {
  const errors: Partial<Record<LoadoutKey, string>> = {}
  for (const row of LOADOUT_ROWS) {
    const value = picks[row.key]
    if (!value) continue
    if (row.source.type !== 'edge') continue
    if (!picks[row.source.dependsOn]) {
      errors[row.key] = `Requires a ${row.source.dependsOn} pick`
      continue
    }
    const allowed = resolveRowAllowed(row, sets)
    if (allowed && !allowed.has(value)) {
      errors[row.key] = `Not compatible with the selected ${row.source.dependsOn}`
    }
  }
  return { valid: Object.keys(errors).length === 0, errors }
}

/** Picks → SlotLoadout v2 ('' → null/absent) + display summary. What updateSlotLoadout
 *  persists — the editor writes v2 only (v1 docs are migrated on read). */
export function picksToLoadout(
  picks: Record<LoadoutKey, string>,
  catalogByName: ReadonlyMap<string, RegistryItem>,
): SlotLoadoutV2 | null {
  if (LOADOUT_ROWS.every((r) => !picks[r.key])) return null // all-empty = clear the doc field

  const weapons: LoadoutWeapon[] = []
  for (const slot of WEAPON_SLOTS) {
    const weapon = picks[slot.key]
    if (!weapon) continue
    weapons.push({
      slotIndex: slot.slotIndex,
      slotType: slot.slotType,
      weapon,
      // optic/magazine bind to weapons[0] until the attachments slice
      ...(slot.key === 'primary'
        ? { optic: picks.optic || null, magazine: picks.magazine || null, attachments: [] }
        : {}),
    })
  }

  const wear: Record<string, string | null> = {}
  for (const key of WEAR_PICK_KEYS) wear[key] = picks[key] || null

  const summary = buildLoadoutSummary(picks, catalogByName)
  return {
    version: 2,
    wear,
    weapons,
    ...(summary ? { summary } : {}),
  }
}

/** SlotLoadout v2 → picks (the render/edit shape; callers migrate v1 docs first). */
export function loadoutToPicks(loadout: SlotLoadoutV2 | undefined): Record<LoadoutKey, string> {
  const picks = { ...EMPTY_PICKS }
  if (!loadout) return picks
  for (const slot of WEAPON_SLOTS) {
    const w = loadout.weapons.find(
      (x) => x.slotIndex === slot.slotIndex && x.slotType === slot.slotType,
    )
    if (!w) continue
    picks[slot.key] = w.weapon
    if (slot.key === 'primary') {
      picks.optic = w.optic ?? ''
      picks.magazine = w.magazine ?? ''
    }
  }
  for (const key of WEAR_PICK_KEYS) picks[key] = loadout.wear[key] ?? ''
  return picks
}

/** Weapon-line display summary ("M16A2 · ACOG · 30rnd STANAG · M72 LAW") for the orbat
 *  loadout string — primary chain plus the second weapon slot when filled. */
export function buildLoadoutSummary(
  picks: Record<LoadoutKey, string>,
  catalogByName: ReadonlyMap<string, RegistryItem>,
): string {
  return (['primary', 'optic', 'magazine', 'launcher'] as const)
    .map((k) => picks[k])
    .filter(Boolean)
    .map((v) => displayName(v, catalogByName))
    .join(' · ')
}
