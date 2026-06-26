// Right-panel default (Ultra Plan §5.2): the Asset Browser as a nested, collapsible
// Eden-style tree (Faction → Category → Class), NOT flat pills. Leaves are draggable:
// dragging one onto the <TacticalMap> places a slot at the drop point. The catalog is
// still mock; the registry-backed feed (GET /api/v1/registry) lands in T-068.
// T-055: a search field filters the tree live by asset/folder name.

import { useMemo, useState } from 'react'
import { Search, X } from 'lucide-react'
import { ASSET_DND_MIME } from '@/features/tactical-map'
import type { AssetDropPayload } from '@/features/tactical-map'
import { TreeView, type TreeNodeData } from '../tree/TreeView'
import { ASSET_CATALOG } from './assetCatalogMock'

// Recursively filter the catalog by a lowercased query. A leaf is kept on a label match;
// a folder is kept if its own name matches (→ keep its full subtree, so "nato" shows all of
// NATO) or any descendant matches (→ keep only the matching children). Retained folders are
// force-expanded so matches are visible (the TreeView is keyed on the query so its mount-time
// expand pass re-runs over these nodes).
function filterCatalog(nodes: TreeNodeData[], q: string): TreeNodeData[] {
  const out: TreeNodeData[] = []
  for (const n of nodes) {
    const selfMatch = n.label.toLowerCase().includes(q)
    if (n.children) {
      if (selfMatch) {
        out.push({ ...n, defaultExpanded: true })
      } else {
        const kids = filterCatalog(n.children, q)
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

  const nodes = useMemo(() => {
    const q = query.trim().toLowerCase()
    return q ? filterCatalog(ASSET_CATALOG, q) : ASSET_CATALOG
  }, [query])

  const onNodeDragStart = (node: TreeNodeData, e: React.DragEvent) => {
    const payload: AssetDropPayload = { assetId: node.id, role: node.label, kind: 'slot' }
    e.dataTransfer.setData(ASSET_DND_MIME, JSON.stringify(payload))
    e.dataTransfer.effectAllowed = 'copy'
  }

  return (
    <div className="flex flex-col gap-2">
      <header>
        <h2 className="text-headline-sm text-on-surface">Asset Browser</h2>
        <p className="text-label-sm normal-case text-outline">
          Drag an asset onto the map to place it.
        </p>
      </header>

      <div className="relative">
        <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-outline" />
        <input
          type="text"
          value={query}
          placeholder="Search assets…"
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
          No assets match “{query.trim()}”.
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
    </div>
  )
}
