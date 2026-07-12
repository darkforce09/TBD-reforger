// Right-panel Factions palette (T-152): the operator's faction LIBRARY — side folders →
// authored factions → draggable ORBAT role templates + vehicle pool — replaces the raw
// vanilla registry character dump entirely (T-074). Role drags carry assetId + tag +
// pre-authored SlotLoadout v2 + factionRef; vehicle leaves list only (placement = T-070).
// T-055 search filters the tree live.

import { useMemo, useState } from 'react'
import { Loader2, Search, Settings2, X } from 'lucide-react'
import { ASSET_DND_MIME } from '@/features/tactical-map'
import type { AssetDropPayload } from '@/features/tactical-map'
import { useFactionLibrary } from '@/hooks/queries'
import { TreeView, type TreeNodeData } from '../tree/TreeView'
import { buildFactionTree } from '../../registry/buildFactionTree'
import { FactionManagerDialog } from '../../factions/FactionManagerDialog'

// Recursively filter the tree by a lowercased query. A leaf is kept on a label match; a
// folder on a self or descendant match; retained folders force-expand (TreeView keyed on
// the query re-runs its mount-time expand pass).
function filterTree(nodes: TreeNodeData[], q: string): TreeNodeData[] {
  const out: TreeNodeData[] = []
  for (const n of nodes) {
    const selfMatch = n.label.toLowerCase().includes(q)
    if (n.children) {
      if (selfMatch) {
        out.push({ ...n, defaultExpanded: true })
      } else {
        const kids = filterTree(n.children, q)
        if (kids.length) out.push({ ...n, defaultExpanded: true, children: kids })
      }
    } else if (selfMatch) {
      out.push(n)
    }
  }
  return out
}

export function AssetBrowser() {
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  const [managerOpen, setManagerOpen] = useState(false)

  const { data, isLoading, isError, refetch } = useFactionLibrary()
  const { nodes: allNodes, payloadById } = useMemo(
    () => buildFactionTree(data?.data ?? []),
    [data],
  )

  const nodes = useMemo(() => {
    const q = query.trim().toLowerCase()
    return q ? filterTree(allNodes, q) : allNodes
  }, [query, allNodes])

  const onNodeDragStart = (node: TreeNodeData, e: React.DragEvent) => {
    const payload: AssetDropPayload | undefined = payloadById.get(node.id)
    if (!payload) {
      // Vehicle leaves: listed, not placeable until T-070.
      e.preventDefault()
      return
    }
    e.dataTransfer.setData(ASSET_DND_MIME, JSON.stringify(payload))
    e.dataTransfer.effectAllowed = 'copy'
  }

  const header = (
    <header className="flex items-start justify-between gap-2">
      <div>
        <h2 className="text-headline-sm text-on-surface">Factions</h2>
        <p className="text-label-sm normal-case text-outline">
          Drag a role onto the map to place its slot.
        </p>
      </div>
      <button
        type="button"
        onClick={() => setManagerOpen(true)}
        title="Manage factions"
        className="inline-flex items-center gap-1.5 rounded-md border border-outline-variant/40 px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
      >
        <Settings2 className="size-3.5" />
        Manage
      </button>
    </header>
  )

  const manager = <FactionManagerDialog open={managerOpen} onOpenChange={setManagerOpen} />

  if (isLoading && !data) {
    return (
      <div className="flex flex-col gap-2">
        {header}
        <div className="flex items-center justify-center gap-2 px-2 py-6 text-label-sm normal-case text-outline">
          <Loader2 className="size-3.5 animate-spin" />
          Loading factions…
        </div>
        {manager}
      </div>
    )
  }

  if (isError) {
    return (
      <div className="flex flex-col gap-2">
        {header}
        <div className="flex flex-col items-center gap-2 px-2 py-6 text-center text-label-sm normal-case text-outline">
          Could not load the faction library.
          <button
            type="button"
            onClick={() => refetch()}
            className="rounded-md border border-outline-variant/40 px-2 py-1 text-label-md text-on-surface transition-colors hover:bg-white/10"
          >
            Retry
          </button>
        </div>
        {manager}
      </div>
    )
  }

  if (allNodes.length === 0) {
    return (
      <div className="flex flex-col gap-2">
        {header}
        <div className="flex flex-col items-center gap-2 px-2 py-6 text-center text-label-sm normal-case text-outline">
          No factions yet — author your first one (side, roles, vehicles) and it shows up
          here.
          <button
            type="button"
            onClick={() => setManagerOpen(true)}
            className="rounded-md border border-primary/40 bg-primary/10 px-2 py-1 text-label-md text-primary transition-colors hover:bg-primary/20"
          >
            Open the Faction Manager
          </button>
        </div>
        {manager}
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-2">
      {header}

      <div className="relative">
        <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-outline" />
        <input
          type="text"
          value={query}
          placeholder="Search factions…"
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') setQuery('')
          }}
          className="w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 py-1.5 pl-8 pr-7 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60"
        />
        {query && (
          <button
            type="button"
            aria-label="Clear search"
            title="Clear"
            onClick={() => setQuery('')}
            className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
          >
            <X className="size-3.5" />
          </button>
        )}
      </div>

      {nodes.length === 0 ? (
        <p className="px-2 py-6 text-center text-label-sm normal-case break-words text-outline">
          Nothing matches “{query.trim()}”.
        </p>
      ) : (
        <TreeView
          key={query.trim() || 'all'}
          nodes={nodes}
          selectedId={selectedId}
          onSelect={setSelectedId}
          onNodeDragStart={onNodeDragStart}
        />
      )}
      {manager}
    </div>
  )
}
