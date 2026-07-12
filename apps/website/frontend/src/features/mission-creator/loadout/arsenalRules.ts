// Smart Forge rules (T-068.10) — pure, worker/React-free (mirrors registryGraph.ts style so it
// vitests in the node environment). Owns the declarative row config, option building, validation,
// and the display summary. The Arsenal UI is a dumb render loop over LOADOUT_ROWS: adding a gear
// slot later (backpack, handgun, launcher) is one config row + one SlotLoadout field — no new
// component or validation code.

import type { SlotLoadout } from '@/features/tactical-map'
import type {
  RegistryCompatEdgeType,
  RegistryItem,
  RegistryItemKind,
} from '@/types/models/registry'

/** The pickable SlotLoadout fields (everything but the derived `summary`). */
export type LoadoutKey = keyof Omit<SlotLoadout, 'summary'>

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
 * The Forge rows, in render order. No ammo row — `ammo_in_mag` ships no edges (T-150 OPEN).
 *
 * T-068.10.3: rows are limited to what SlotLoadout v1 can PERSIST. The v3 kinds split
 * Reforger wear areas (jacket/pants/boots, vest vs armored vest, gloves, …) but the doc
 * still has ACE-shaped fields — so the `uniform` field is fed by `gear_jacket` (the v1→v2
 * migration maps uniform→wear.jacket) and `vest` by `gear_vest` (chest rigs). Rows for
 * pants/boots/armored vest/backpack/launcher/handgun/throwable/equipment land in
 * T-068.10.4 together with the SlotLoadout v2 fields that can store them.
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
  { key: 'uniform', label: 'Jacket', source: { type: 'kind', kind: 'gear_jacket' } },
  { key: 'vest', label: 'Vest', source: { type: 'kind', kind: 'gear_vest' } },
  { key: 'helmet', label: 'Helmet', source: { type: 'kind', kind: 'gear_helmet' } },
] as const

/** v3 kinds whose Forge rows wait on SlotLoadout v2 fields (shown as a hint in the tab). */
export const PENDING_V2_KINDS: readonly RegistryItemKind[] = [
  'gear_pants',
  'gear_boots',
  'gear_armored_vest',
  'gear_backpack',
  'gear_launcher',
  'gear_handgun',
  'gear_throwable',
  'gear_glasses',
  'gear_gloves',
  'gear_binoculars',
  'gear_item',
] as const

/** The all-empty pick set (a slot that has never been forged). */
export const EMPTY_PICKS: Record<LoadoutKey, string> = {
  primary: '',
  uniform: '',
  vest: '',
  helmet: '',
  optic: '',
  magazine: '',
}

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
    if (catalogByName.get(v)?.abstract === true) return v === current
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

/** Picks → SlotLoadout ('' → null) + display summary. What updateSlotLoadout persists. */
export function picksToLoadout(
  picks: Record<LoadoutKey, string>,
  catalogByName: ReadonlyMap<string, RegistryItem>,
): SlotLoadout | null {
  if (LOADOUT_ROWS.every((r) => !picks[r.key])) return null // all-empty = clear the doc field
  const summary = buildLoadoutSummary(picks, catalogByName)
  return {
    primary: picks.primary || null,
    uniform: picks.uniform || null,
    vest: picks.vest || null,
    helmet: picks.helmet || null,
    optic: picks.optic || null,
    magazine: picks.magazine || null,
    ...(summary ? { summary } : {}),
  }
}

/** SlotLoadout → picks (the render/edit shape; null → ''). */
export function loadoutToPicks(loadout: SlotLoadout | undefined): Record<LoadoutKey, string> {
  if (!loadout) return { ...EMPTY_PICKS }
  return {
    primary: loadout.primary ?? '',
    uniform: loadout.uniform ?? '',
    vest: loadout.vest ?? '',
    helmet: loadout.helmet ?? '',
    optic: loadout.optic ?? '',
    magazine: loadout.magazine ?? '',
  }
}

/** Weapon-line display summary ("M16A2 · ACOG · 30rnd STANAG") for the orbat loadout string. */
export function buildLoadoutSummary(
  picks: Record<LoadoutKey, string>,
  catalogByName: ReadonlyMap<string, RegistryItem>,
): string {
  return (['primary', 'optic', 'magazine'] as const)
    .map((k) => picks[k])
    .filter(Boolean)
    .map((v) => displayName(v, catalogByName))
    .join(' · ')
}
