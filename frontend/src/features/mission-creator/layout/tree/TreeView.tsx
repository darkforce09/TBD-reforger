// Reusable recursive tree view — the Eden Editor nested-file-tree paradigm, shared by
// the left "Placed Entities" panel and the right "Asset Browser". Presentation-only:
// folders (nodes with `children`) collapse/expand; leaves select; double-click fires
// onActivate. Drag affordance is cosmetic (cursor-grab) — no DnD logic is wired yet.
// Indentation comes purely from nested <ul> margins (one guide line per level).

import { useState } from 'react'
import { ChevronRight, FolderOpen } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { cn } from '@/lib/utils'

export interface TreeNodeData {
  id: string
  label: string
  icon?: LucideIcon
  badge?: string
  children?: TreeNodeData[]
  defaultExpanded?: boolean
}

interface TreeViewProps {
  nodes: TreeNodeData[]
  selectedId?: string | null
  onSelect?: (id: string) => void
  onActivate?: (id: string) => void
  /** Native HTML5 drag-start on a leaf (e.g. drag an asset onto the map). When
   *  omitted, leaves are not draggable. */
  onNodeDragStart?: (node: TreeNodeData, e: React.DragEvent) => void
}

function collectExpanded(nodes: TreeNodeData[], acc: Set<string>): Set<string> {
  for (const n of nodes) {
    if (n.children && n.defaultExpanded) acc.add(n.id)
    if (n.children) collectExpanded(n.children, acc)
  }
  return acc
}

export function TreeView({
  nodes,
  selectedId,
  onSelect,
  onActivate,
  onNodeDragStart,
}: TreeViewProps) {
  const [expanded, setExpanded] = useState<Set<string>>(() =>
    collectExpanded(nodes, new Set()),
  )

  const toggle = (id: string) =>
    setExpanded((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const shared = { expanded, toggle, selectedId, onSelect, onActivate, onNodeDragStart }

  return (
    <ul className="flex flex-col">
      {nodes.map((n) => (
        <TreeNode key={n.id} node={n} {...shared} />
      ))}
    </ul>
  )
}

interface TreeNodeProps {
  node: TreeNodeData
  expanded: Set<string>
  toggle: (id: string) => void
  selectedId?: string | null
  onSelect?: (id: string) => void
  onActivate?: (id: string) => void
  onNodeDragStart?: (node: TreeNodeData, e: React.DragEvent) => void
}

function TreeNode({
  node,
  expanded,
  toggle,
  selectedId,
  onSelect,
  onActivate,
  onNodeDragStart,
}: TreeNodeProps) {
  const isFolder = !!node.children?.length
  const isOpen = expanded.has(node.id)
  const selected = selectedId === node.id
  // Folders show an "open" glyph when expanded; leaves keep their own icon.
  const Icon = isFolder && isOpen ? FolderOpen : node.icon
  const draggable = !isFolder && !!onNodeDragStart

  return (
    <li>
      <div
        draggable={draggable}
        onDragStart={draggable ? (e) => onNodeDragStart!(node, e) : undefined}
        onClick={() => {
          onSelect?.(node.id)
          if (isFolder) toggle(node.id)
        }}
        onDoubleClick={() => !isFolder && onActivate?.(node.id)}
        className={cn(
          'group flex items-center gap-1.5 rounded-md border-l-2 py-1 pr-2 pl-1.5 text-label-md transition-colors',
          draggable ? 'cursor-grab' : 'cursor-pointer',
          selected
            ? 'border-primary bg-primary/15 text-on-surface'
            : 'border-transparent text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
        )}
      >
        <ChevronRight
          className={cn(
            'size-3.5 shrink-0 transition-transform',
            isFolder ? 'text-outline' : 'invisible',
            isOpen && 'rotate-90',
          )}
        />
        {Icon && (
          <Icon className={cn('size-3.5 shrink-0', isFolder ? 'text-tertiary' : 'text-primary')} />
        )}
        <span className="min-w-0 flex-1 truncate">{node.label}</span>
        {node.badge && (
          <span className="shrink-0 rounded bg-surface-variant/60 px-1.5 text-label-sm text-outline">
            {node.badge}
          </span>
        )}
      </div>

      {isFolder && isOpen && (
        <ul className="ml-[1.1rem] flex flex-col border-l border-white/5 pl-0">
          {node.children!.map((c) => (
            <TreeNode
              key={c.id}
              node={c}
              expanded={expanded}
              toggle={toggle}
              selectedId={selectedId}
              onSelect={onSelect}
              onActivate={onActivate}
              onNodeDragStart={onNodeDragStart}
            />
          ))}
        </ul>
      )}
    </li>
  )
}
