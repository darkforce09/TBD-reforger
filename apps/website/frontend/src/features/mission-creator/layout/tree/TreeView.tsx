// Reusable recursive tree view — the Eden Editor nested-file-tree paradigm, shared by
// the left "Editor Layers" / "ORBAT" panels and the right "Asset Browser". Folders
// (nodes with `children`) collapse/expand; leaves select; double-click fires onActivate.
// Optional, opt-in behaviours (Phase 7a): HTML5 drag of nodes (onNodeDragStart) and drop
// onto folders (onNodeDrop) for reparenting, per-row hover actions (renderNodeActions), and
// inline rename (renamingId + onRenameCommit/Cancel). Folder-ness is data-driven via
// TreeNodeData.isFolder so empty folders still accept drops. Consumers that pass none of
// these get the original read-only behaviour.

import { useState } from 'react'
import type { LucideIcon } from 'lucide-react'
import type { ID } from '@/features/tactical-map'
import { TreeRow } from './TreeRow'

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
  /** When set and the folder is expanded, its slot leaves are virtualized from this id list
   *  rather than materialized as `children` — avoids building 360k TreeNodeData objects
   *  (T-064). Consumed by VirtualOutliner; ignored by this recursive renderer. */
  virtualSlotIds?: ID[]
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
  const draggable = !!onNodeDragStart && (allowFolderDrag || !isFolder)
  const isDropTarget = !!onNodeDrop && isFolder
  const renaming = renamingId === node.id

  return (
    <li>
      <TreeRow
        label={node.label}
        icon={node.icon}
        badge={node.badge}
        isFolder={isFolder}
        hasChildren={hasChildren}
        isOpen={isOpen}
        selected={selected}
        draggable={draggable}
        isDropTarget={isDropTarget}
        dragOver={dragOverId === node.id}
        renaming={renaming}
        onRowClick={() => {
          onSelect?.(node.id)
          if (hasChildren) toggle(node.id)
        }}
        onRowDoubleClick={() => !isFolder && onActivate?.(node.id)}
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
        actions={renderNodeActions ? renderNodeActions(node) : undefined}
        renameDefaultValue={node.label}
        onRenameCommit={(value) => onRenameCommit?.(node.id, value)}
        onRenameCancel={onRenameCancel}
      />

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
