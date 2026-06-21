// Reusable recursive tree view — the Eden Editor nested-file-tree paradigm, shared by
// the left "Editor Layers" / "ORBAT" panels and the right "Asset Browser". Folders
// (nodes with `children`) collapse/expand; leaves select; double-click fires onActivate.
// Optional, opt-in behaviours (Phase 7a): HTML5 drag of nodes (onNodeDragStart) and drop
// onto folders (onNodeDrop) for reparenting, per-row hover actions (renderNodeActions), and
// inline rename (renamingId + onRenameCommit/Cancel). Folder-ness is data-driven via
// TreeNodeData.isFolder so empty folders still accept drops. Consumers that pass none of
// these get the original read-only behaviour.

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
  /** Force folder semantics (drop target, folder chrome) even with no children — an
   *  empty Editor-Layers folder. Defaults to "has children" when unset. */
  isFolder?: boolean
}

interface TreeViewProps {
  nodes: TreeNodeData[]
  selectedId?: string | null
  /** Multi-select highlight set (Phase 7b); a node is selected if it's here OR === selectedId. */
  selectedIds?: ReadonlySet<string>
  onSelect?: (id: string) => void
  onActivate?: (id: string) => void
  /** Native HTML5 drag-start on a node. When omitted, nodes are not draggable. */
  onNodeDragStart?: (node: TreeNodeData, e: React.DragEvent) => void
  /** Also let folders (not just leaves) initiate a drag — outliner reparent (Phase 7a). */
  allowFolderDrag?: boolean
  /** Drop onto a folder node (reparent / refile). Folders become drop targets when set. */
  onNodeDrop?: (node: TreeNodeData, e: React.DragEvent) => void
  /** Right-aligned hover actions per node (rename/delete buttons). */
  renderNodeActions?: (node: TreeNodeData) => React.ReactNode
  /** Node currently being renamed inline; its label becomes an input. */
  renamingId?: string | null
  onRenameCommit?: (id: string, name: string) => void
  onRenameCancel?: () => void
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
  selectedIds,
  onSelect,
  onActivate,
  onNodeDragStart,
  allowFolderDrag,
  onNodeDrop,
  renderNodeActions,
  renamingId,
  onRenameCommit,
  onRenameCancel,
}: TreeViewProps) {
  const [expanded, setExpanded] = useState<Set<string>>(() =>
    collectExpanded(nodes, new Set()),
  )
  const [dragOverId, setDragOverId] = useState<string | null>(null)

  const toggle = (id: string) =>
    setExpanded((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const shared = {
    expanded,
    toggle,
    selectedId,
    selectedIds,
    onSelect,
    onActivate,
    onNodeDragStart,
    allowFolderDrag,
    onNodeDrop,
    renderNodeActions,
    renamingId,
    onRenameCommit,
    onRenameCancel,
    dragOverId,
    setDragOverId,
  }

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
  selectedIds?: ReadonlySet<string>
  onSelect?: (id: string) => void
  onActivate?: (id: string) => void
  onNodeDragStart?: (node: TreeNodeData, e: React.DragEvent) => void
  allowFolderDrag?: boolean
  onNodeDrop?: (node: TreeNodeData, e: React.DragEvent) => void
  renderNodeActions?: (node: TreeNodeData) => React.ReactNode
  renamingId?: string | null
  onRenameCommit?: (id: string, name: string) => void
  onRenameCancel?: () => void
  dragOverId: string | null
  setDragOverId: (id: string | null) => void
}

function TreeNode({
  node,
  expanded,
  toggle,
  selectedId,
  selectedIds,
  onSelect,
  onActivate,
  onNodeDragStart,
  allowFolderDrag,
  onNodeDrop,
  renderNodeActions,
  renamingId,
  onRenameCommit,
  onRenameCancel,
  dragOverId,
  setDragOverId,
}: TreeNodeProps) {
  // Folder-ness is data-driven (empty Editor-Layers folders are still folders); expand
  // chrome keys off whether there's actually anything to expand.
  const isFolder = node.isFolder ?? !!node.children?.length
  const hasChildren = !!node.children?.length
  const isOpen = expanded.has(node.id)
  const selected = selectedId === node.id || (selectedIds?.has(node.id) ?? false)
  // Folders show an "open" glyph when expanded; leaves keep their own icon.
  const Icon = isFolder && isOpen && hasChildren ? FolderOpen : node.icon
  const draggable = !!onNodeDragStart && (allowFolderDrag || !isFolder)
  const isDropTarget = !!onNodeDrop && isFolder
  const renaming = renamingId === node.id

  return (
    <li>
      <div
        draggable={draggable}
        onDragStart={draggable ? (e) => onNodeDragStart!(node, e) : undefined}
        onDragOver={
          isDropTarget
            ? (e) => {
                e.preventDefault()
                e.dataTransfer.dropEffect = 'move'
                setDragOverId(node.id)
              }
            : undefined
        }
        onDragLeave={
          isDropTarget ? () => dragOverId === node.id && setDragOverId(null) : undefined
        }
        onDrop={
          isDropTarget
            ? (e) => {
                e.preventDefault()
                e.stopPropagation()
                onNodeDrop!(node, e)
                setDragOverId(null)
              }
            : undefined
        }
        onClick={() => {
          onSelect?.(node.id)
          if (hasChildren) toggle(node.id)
        }}
        onDoubleClick={() => !isFolder && onActivate?.(node.id)}
        className={cn(
          'group flex items-center gap-1.5 rounded-md border-l-2 py-1 pr-2 pl-1.5 text-label-md transition-colors',
          draggable ? 'cursor-grab' : 'cursor-pointer',
          selected
            ? 'border-primary bg-primary/15 text-on-surface'
            : 'border-transparent text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
          dragOverId === node.id && 'ring-1 ring-inset ring-primary',
        )}
      >
        <ChevronRight
          className={cn(
            'size-3.5 shrink-0 transition-transform',
            hasChildren ? 'text-outline' : 'invisible',
            isOpen && 'rotate-90',
          )}
        />
        {Icon && (
          <Icon className={cn('size-3.5 shrink-0', isFolder ? 'text-tertiary' : 'text-primary')} />
        )}

        {renaming ? (
          <input
            autoFocus
            defaultValue={node.label}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              if (e.key === 'Enter') onRenameCommit?.(node.id, (e.target as HTMLInputElement).value)
              else if (e.key === 'Escape') onRenameCancel?.()
            }}
            onBlur={(e) => onRenameCommit?.(node.id, e.target.value)}
            className="min-w-0 flex-1 rounded bg-surface-container-lowest/60 px-1 text-on-surface outline-none ring-1 ring-primary/60"
          />
        ) : (
          <span className={cn('min-w-0 flex-1 truncate', isFolder && 'font-medium')}>{node.label}</span>
        )}

        {!renaming && node.badge && (
          <span className="shrink-0 rounded bg-surface-variant/60 px-1.5 text-label-sm text-outline">
            {node.badge}
          </span>
        )}
        {!renaming && renderNodeActions && (
          <span className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
            {renderNodeActions(node)}
          </span>
        )}
      </div>

      {hasChildren && isOpen && (
        <ul className="ml-[1.1rem] flex flex-col border-l border-white/5 pl-0">
          {node.children!.map((c) => (
            <TreeNode
              key={c.id}
              node={c}
              expanded={expanded}
              toggle={toggle}
              selectedId={selectedId}
              selectedIds={selectedIds}
              onSelect={onSelect}
              onActivate={onActivate}
              onNodeDragStart={onNodeDragStart}
              allowFolderDrag={allowFolderDrag}
              onNodeDrop={onNodeDrop}
              renderNodeActions={renderNodeActions}
              renamingId={renamingId}
              onRenameCommit={onRenameCommit}
              onRenameCancel={onRenameCancel}
              dragOverId={dragOverId}
              setDragOverId={setDragOverId}
            />
          ))}
        </ul>
      )}
    </li>
  )
}
