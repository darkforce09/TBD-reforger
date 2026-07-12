// Pure Arsenal picks panel (T-152 extraction): the search box + the 14 grouped/sorted/
// abstract-and-variant-filtered picker rows over a picks record. No doc, no slot, no
// worker — callers own the state: ArsenalTab adapts it onto Slot.loadout (compat worker
// live), the Faction Manager onto a role template's draft loadout (kind-only degrade sets).

import { useState } from 'react'
import { Search, X } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import type { RegistryItem } from '@/types/models/registry'
import {
  LOADOUT_ROWS,
  buildGroupedRowOptions,
  validateLoadout,
  type CompatSets,
  type LoadoutKey,
} from './arsenalRules'
import { Field, SelectField } from '../layout/RightInspector/fields'

export function ArsenalPicksPanel({
  picks,
  onPick,
  catalog,
  catalogByName,
  sets,
  smart,
}: {
  picks: Record<LoadoutKey, string>
  onPick: (key: LoadoutKey, value: string) => void
  catalog: readonly RegistryItem[]
  catalogByName: ReadonlyMap<string, RegistryItem>
  /** Compat feeds; pass `{ edgeItems: {} }` for kind-only degrade mode. */
  sets: CompatSets
  /** Worker live? Edge rows render + validate only when true. */
  smart: boolean
}) {
  const [query, setQuery] = useState('')
  const validation = smart ? validateLoadout(picks, sets) : { valid: true, errors: {} }
  const rows = smart ? LOADOUT_ROWS : LOADOUT_ROWS.filter((r) => r.source.type === 'kind')

  return (
    <div className="flex flex-col gap-4">
      <div className="relative">
        <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-outline" />
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') setQuery('')
          }}
          placeholder="Filter gear by name…"
          className="w-full rounded-md border border-outline-variant/30 bg-surface-container-lowest/40 py-1.5 pl-8 pr-8 text-label-md text-on-surface placeholder:text-outline focus:border-primary/50 focus:outline-none"
        />
        {query && (
          <button
            type="button"
            onClick={() => setQuery('')}
            aria-label="Clear gear filter"
            className="absolute right-2 top-1/2 -translate-y-1/2 text-outline hover:text-on-surface"
          >
            <X className="size-3.5" />
          </button>
        )}
      </div>

      {rows.map((row) => {
        const disabledDep =
          smart && row.source.type === 'edge' && !picks[row.source.dependsOn]
            ? row.source.dependsOn
            : null
        const error = smart ? validation.errors[row.key] : undefined
        const grouped = disabledDep
          ? null
          : buildGroupedRowOptions(row, picks[row.key], sets, catalog, catalogByName, query)
        return (
          <div key={row.key} className="flex flex-col gap-1">
            {disabledDep || !grouped ? (
              <Field label={row.label}>
                <div className="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 text-label-md text-outline">
                  Pick a {disabledDep} first
                </div>
              </Field>
            ) : (
              <SelectField
                label={row.label}
                value={picks[row.key]}
                options={[grouped.none, ...(grouped.stranded ? [grouped.stranded] : [])]}
                groups={grouped.groups}
                onChange={(v) => onPick(row.key, v)}
              />
            )}
            {error && <Badge variant="error">{error}</Badge>}
          </div>
        )
      })}
    </div>
  )
}
