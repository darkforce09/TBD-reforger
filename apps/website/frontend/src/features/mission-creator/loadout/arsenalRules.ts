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

/** The Forge rows, in render order. No ammo row — `ammo_in_mag` ships no edges (T-150 OPEN). */
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
  { key: 'uniform', label: 'Uniform', source: { type: 'kind', kind: 'gear_uniform' } },
  { key: 'vest', label: 'Vest', source: { type: 'kind', kind: 'gear_vest' } },
  { key: 'helmet', label: 'Helmet', source: { type: 'kind', kind: 'gear_helmet' } },
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
 * Options for one row. Kind rows: the full kind catalog. Edge rows: the compat feed. Retains a
 * stranded non-empty `current` at the end, suffixed incompatible, so a native <select> never
 * silently blanks.
 */
export function buildRowOptions(
  row: LoadoutRow,
  current: string,
  sets: CompatSets,
  catalog: readonly RegistryItem[],
  catalogByName: ReadonlyMap<string, RegistryItem>,
): PickOption[] {
  const src = row.source
  const values =
    src.type === 'kind'
      ? catalog.filter((i) => i.kind === src.kind).map((i) => i.resource_name)
      : [...(sets.edgeItems[row.key] ?? [])]
  const options = [
    NONE_OPTION,
    ...values.map((v) => ({ value: v, label: displayName(v, catalogByName) })),
  ]
  if (current && !values.includes(current)) {
    options.push({ value: current, label: displayName(current, catalogByName) + INCOMPAT_SUFFIX })
  }
  return options
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
