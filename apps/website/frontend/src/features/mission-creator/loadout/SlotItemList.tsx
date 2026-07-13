// Arsenal left column (T-068.10.7): the ACE item list for the active silhouette region —
// click a row to equip it (no dropdowns). Options come from the same grouped/sorted/
// abstract-and-variant-filtered pipeline as the pickers (buildGroupedRowOptions, incl. the
// never-drop-current + stranded-incompatible semantics). Mount keyed on activeKey so the
// search query resets per region. Pure over props: callers own picks/sets state.

import { useState } from 'react'
import { Check, Search, X } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { RegistryItem } from '@/types/models/registry'
import {
  LOADOUT_ROWS,
  buildGroupedRowOptions,
  type CompatSets,
  type LoadoutKey,
  type PickOption,
} from './arsenalRules'

function Row({
  option,
  current,
  onPick,
}: {
  option: PickOption
  current: boolean
  onPick: (value: string) => void
}) {
  return (
    <button
      type="button"
      onClick={() => onPick(option.value)}
      className={cn(
        'flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-label-md transition-colors',
        current
          ? 'bg-primary/20 text-primary'
          : 'text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
      )}
    >
      <span className="min-w-0 flex-1 truncate normal-case">{option.label}</span>
      {current && <Check className="size-3.5 shrink-0" />}
    </button>
  )
}

function EmptyState({ children }: { children: string }) {
  return (
    <div className="rounded-md border border-dashed border-outline-variant/30 p-3 text-center text-label-sm normal-case text-outline">
      {children}
    </div>
  )
}

type EdgeBlock = 'compat-down' | 'no-dependency' | null

/** Why an edge region (optic/magazine) cannot list right now, if at all. */
function edgeBlockReason(
  row: (typeof LOADOUT_ROWS)[number],
  picks: Record<LoadoutKey, string>,
  smart: boolean,
): EdgeBlock {
  if (row.source.type !== 'edge') return null
  if (!smart) return 'compat-down'
  return picks[row.source.dependsOn] ? null : 'no-dependency'
}

function GroupedList({
  grouped,
  current,
  query,
  onPick,
}: {
  grouped: NonNullable<ReturnType<typeof buildGroupedRowOptions>>
  current: string
  query: string
  onPick: (value: string) => void
}) {
  const count = grouped.groups.reduce((n, g) => n + g.options.length, 0)
  return (
    <div className="flex flex-col gap-0.5">
      <Row option={grouped.none} current={current === ''} onPick={onPick} />
      {grouped.groups.map((g) => (
        <div key={g.label || 'all'} className="flex flex-col gap-0.5">
          {g.label && (
            <p className="mt-2 px-2 text-label-sm uppercase tracking-wide text-outline">
              {g.label}
            </p>
          )}
          {g.options.map((o) => (
            <Row key={o.value} option={o} current={o.value === current} onPick={onPick} />
          ))}
        </div>
      ))}
      {grouped.stranded && (
        <Row option={grouped.stranded} current={grouped.stranded.value === current} onPick={onPick} />
      )}
      {count === 0 && !grouped.stranded && (
        <EmptyState>
          {query ? 'Nothing matches the filter.' : 'Nothing available for this slot.'}
        </EmptyState>
      )}
    </div>
  )
}

export function SlotItemList({
  activeKey,
  picks,
  onPick,
  catalog,
  catalogByName,
  sets,
  smart,
}: {
  activeKey: LoadoutKey
  picks: Record<LoadoutKey, string>
  onPick: (key: LoadoutKey, value: string) => void
  catalog: readonly RegistryItem[]
  catalogByName: ReadonlyMap<string, RegistryItem>
  /** Compat feeds; `{ edgeItems: {} }` in kind-only degrade mode. */
  sets: CompatSets
  /** Worker live? Edge regions (optic/magazine) list only when true. */
  smart: boolean
}) {
  const [query, setQuery] = useState('')
  const row = LOADOUT_ROWS.find((r) => r.key === activeKey)
  if (!row) return null // unreachable: every LoadoutKey has a row (doll-model completeness)

  const current = picks[activeKey]
  const pick = (value: string) => onPick(activeKey, value)

  const edgeBlocked = edgeBlockReason(row, picks, smart)
  const grouped = edgeBlocked
    ? null
    : buildGroupedRowOptions(row, current, sets, catalog, catalogByName, query)
  const count = grouped ? grouped.groups.reduce((n, g) => n + g.options.length, 0) : 0

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-baseline justify-between gap-2">
        <h3 className="text-title-sm text-on-surface">{row.label}</h3>
        <span className="font-mono text-label-sm tabular-nums text-outline">{count}</span>
      </div>

      <div className="relative">
        <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-outline" />
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') setQuery('')
          }}
          placeholder={`Filter ${row.label.toLowerCase()}…`}
          className="w-full rounded-md border border-outline-variant/30 bg-surface-container-lowest/40 py-1.5 pl-8 pr-8 text-label-md text-on-surface placeholder:text-outline focus:border-primary/50 focus:outline-none"
        />
        {query && (
          <button
            type="button"
            onClick={() => setQuery('')}
            aria-label="Clear filter"
            className="absolute right-2 top-1/2 -translate-y-1/2 text-outline hover:text-on-surface"
          >
            <X className="size-3.5" />
          </button>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto pr-1">
        {edgeBlocked === 'compat-down' && (
          <EmptyState>Compatibility unavailable — this list needs the compat graph.</EmptyState>
        )}
        {edgeBlocked === 'no-dependency' && row.source.type === 'edge' && (
          <EmptyState>{`Pick a ${row.source.dependsOn} first — then compatible ${row.label.toLowerCase()}s list here.`}</EmptyState>
        )}
        {grouped && <GroupedList grouped={grouped} current={current} query={query} onPick={pick} />}
      </div>
    </div>
  )
}
