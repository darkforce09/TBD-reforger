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
 * - `kind`: the flat registry catalog filtered by item kind, constrained to the character's
 *   canEquip set (`character_default_loadout` family) when compat data exists.
 * - `edge`: the compat graph via `itemsFor(picks[dependsOn], edge)` — empty until the
 *   dependency is picked.
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

/** Compat context for option building + validation. `equipSet: null` = no per-character
 *  compat data (degrade: kind rows show the full catalog). Edge sets are keyed by row key. */
export interface CompatSets {
  equipSet: Set<string> | null
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
 * opinion (per-kind degrade → full catalog, no validation). Kind rows: the row's kind items ∩
 * the character's canEquip set — `null` when there is no equip set at all OR the intersection is
 * empty (the T-150 export links clothing kinds only, so e.g. `gear_primary` has no equip edges
 * for any character; a hard filter would brick the row to "None"). Edge rows: exactly the
 * dependency's `itemsFor` feed — never null; an empty feed means the host truly accepts nothing.
 */
export function resolveRowAllowed(
  row: LoadoutRow,
  sets: CompatSets,
  catalog: readonly RegistryItem[],
): Set<string> | null {
  if (row.source.type === 'edge') return new Set(sets.edgeItems[row.key] ?? [])
  const equip = sets.equipSet
  if (!equip) return null
  const kind = row.source.kind
  const allowed = new Set(
    catalog
      .filter((i) => i.kind === kind && equip.has(i.resource_name))
      .map((i) => i.resource_name),
  )
  return allowed.size ? allowed : null
}

/**
 * Options for one row. Allowed set (see resolveRowAllowed) when the graph has an opinion, else
 * the full kind catalog. Retains a stranded non-empty `current` at the end, suffixed
 * incompatible, so a native <select> never silently blanks.
 */
export function buildRowOptions(
  row: LoadoutRow,
  current: string,
  sets: CompatSets,
  catalog: readonly RegistryItem[],
  catalogByName: ReadonlyMap<string, RegistryItem>,
): PickOption[] {
  const allowed = resolveRowAllowed(row, sets, catalog)
  let values: string[]
  if (row.source.type === 'kind') {
    const kind = row.source.kind
    values = catalog
      .filter((i) => i.kind === kind && (!allowed || allowed.has(i.resource_name)))
      .map((i) => i.resource_name)
  } else {
    values = [...(sets.edgeItems[row.key] ?? [])]
  }
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
 * are always valid. Kind picks fail when the row's allowed set (resolveRowAllowed — includes the
 * per-kind degrade) exists and rejects them; edge picks additionally fail when their dependency
 * is empty.
 */
export function validateLoadout(
  picks: Record<LoadoutKey, string>,
  sets: CompatSets,
  catalog: readonly RegistryItem[],
): LoadoutValidation {
  const errors: Partial<Record<LoadoutKey, string>> = {}
  for (const row of LOADOUT_ROWS) {
    const value = picks[row.key]
    if (!value) continue
    if (row.source.type === 'edge' && !picks[row.source.dependsOn]) {
      errors[row.key] = `Requires a ${row.source.dependsOn} pick`
      continue
    }
    const allowed = resolveRowAllowed(row, sets, catalog)
    if (allowed && !allowed.has(value)) {
      errors[row.key] =
        row.source.type === 'kind'
          ? 'Not in this character’s compatible gear'
          : `Not compatible with the selected ${row.source.dependsOn}`
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
