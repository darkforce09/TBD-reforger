// Virtualized left outliner (T-064) — one @tanstack/react-virtual list over BOTH outliner
// trees (ORBAT + Editor Layers), anchored on the sidebar's single scroll container. Only the
// ~viewport rows are mounted, so expanding a 360k-slot folder no longer freezes the tab. Rows
// are resolved on demand from a small OutlinerSegment[] (see flattenOutliner). Folder/slot
// rows render the shared <TreeRow>; section headers, the empty state, and the root drop zone
// are rendered inline. Editor rows are draggable/droppable + carry hover actions; ORBAT is
// read-only.

import { useState } from 'react'
import { FolderPlus } from 'lucide-react'
import type { ID, Slot } from '@/features/tactical-map'
import { cn } from '@/lib/utils'
import { useVirtualizer } from '@tanstack/react-virtual'
import { TreeRow } from './TreeRow'
import { rowAt, totalRowCount } from './flattenOutliner'
import {
  INDENT_PX,
  ROW_HEIGHT,
  type OutlinerSegment,
  type TreeRowModel,
} from './treeRowModel'
import { TREE_MIME } from '../LeftOutliner/EditorLayersSection'

interface VirtualOutlinerProps {
  /** The scroll container element, supplied by the parent via callback-ref state (not a
   *  RefObject) so this hook re-runs once the div has actually mounted — see useVirtualizer. */
  scrollElement: HTMLDivElement | null
  segments: OutlinerSegment[]
  slotsById: Record<ID, Slot>
  toggle: (id: string) => void
  /** Editor-folder selection highlight (active drop target). */
  activeLayerId: string | null
  /** Slot multi-select highlight (shared by both trees). */
  selectedIds?: ReadonlySet<string>
  onSelectRow: (row: TreeRowModel) => void
  onActivateRow: (row: TreeRowModel) => void
  // ── Editor-only interactions ──
  onNodeDragStart: (id: ID, e: React.DragEvent) => void
  onNodeDrop: (targetId: ID, e: React.DragEvent) => void
  onRootDrop: (e: React.DragEvent) => void
  renderRowActions: (id: ID) => React.ReactNode
  renamingId: string | null
  onRenameCommit: (id: ID, value: string) => void
  onRenameCancel: () => void
  onNewFolder: () => void
}

export function VirtualOutliner({
  scrollElement,
  segments,
  slotsById,
  toggle,
  activeLayerId,
  selectedIds,
  onSelectRow,
  onActivateRow,
  onNodeDragStart,
  onNodeDrop,
  onRootDrop,
  renderRowActions,
  renamingId,
  onRenameCommit,
  onRenameCancel,
  onNewFolder,
}: VirtualOutlinerProps) {
  const [dragOverId, setDragOverId] = useState<string | null>(null)
  const [rootOver, setRootOver] = useState(false)

  const count = totalRowCount(segments)
  // `scrollElement` comes from the parent's callback-ref state, so it starts null and flips to
  // the div once mounted — that state change re-runs this hook and paints the rows on load
  // (a RefObject would stay null on first render → zero virtual items → blank outliner; T-064.1).
  // React Compiler can't memoize a component that consumes useVirtualizer (it returns
  // non-memoizable functions); it safely bails out of memoizing VirtualOutliner, which is fine.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count,
    getScrollElement: () => scrollElement,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  })

  return (
    <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
      {virtualizer.getVirtualItems().map((vItem) => {
        const row = rowAt(segments, vItem.index, slotsById)
        if (!row) return null

        const common = {
          key: vItem.key,
          'data-index': vItem.index,
          ref: virtualizer.measureElement,
          className: 'absolute left-0 top-0 w-full',
          style: { transform: `translateY(${vItem.start}px)` },
        } as const

        // ── Section header ──
        if (row.kind === 'section-header') {
          const isEditor = row.section === 'editor'
          return (
            <div {...common}>
              <div className="flex items-center justify-between px-2 py-1.5">
                <h3 className="text-label-sm uppercase tracking-wider text-outline">{row.label}</h3>
                {isEditor && (
                  <button
                    type="button"
                    aria-label="New folder"
                    title="New folder"
                    onClick={onNewFolder}
                    className="rounded p-0.5 text-on-surface-variant transition-colors hover:bg-primary/15 hover:text-primary"
                  >
                    <FolderPlus className="size-3.5" />
                  </button>
                )}
              </div>
            </div>
          )
        }

        // ── Empty state ──
        if (row.kind === 'empty') {
          return (
            <div {...common}>
              <p className="px-2 py-3 text-center text-label-sm normal-case text-outline">{row.label}</p>
            </div>
          )
        }

        // ── Root drop zone (Editor Layers) ──
        if (row.kind === 'root-drop') {
          return (
            <div {...common}>
              <div
                onDragOver={(e) => {
                  if (!e.dataTransfer.types.includes(TREE_MIME)) return
                  e.preventDefault()
                  e.dataTransfer.dropEffect = 'move'
                  setRootOver(true)
                }}
                onDragLeave={() => setRootOver(false)}
                onDrop={(e) => {
                  e.preventDefault()
                  onRootDrop(e)
                  setRootOver(false)
                }}
                className={cn(
                  'mx-1 my-1 rounded border border-dashed px-2 py-1.5 text-center text-label-sm normal-case transition-colors',
                  rootOver
                    ? 'border-primary bg-primary/10 text-primary'
                    : 'border-outline-variant/30 text-outline',
                )}
              >
                {row.label}
              </div>
            </div>
          )
        }

        // ── Folder / slot row ──
        const isEditor = row.section === 'editor'
        const draggable = isEditor && (row.kind === 'folder' || row.kind === 'slot')
        const isDropTarget = isEditor && row.kind === 'folder'
        const selected =
          isEditor && row.kind === 'folder'
            ? activeLayerId === row.id
            : (selectedIds?.has(row.id) ?? false)

        return (
          <div {...common}>
            <TreeRow
              label={row.label}
              icon={row.icon}
              badge={row.badge}
              isFolder={row.isFolder}
              hasChildren={row.hasChildren}
              isOpen={row.isExpanded}
              selected={selected}
              draggable={draggable}
              isDropTarget={isDropTarget}
              dragOver={dragOverId === row.id}
              renaming={renamingId === row.id}
              indentPx={6 + row.depth * INDENT_PX}
              onRowClick={() => {
                onSelectRow(row)
                if (row.hasChildren) toggle(row.id)
              }}
              onRowDoubleClick={() => !row.isFolder && onActivateRow(row)}
              onDragStart={draggable ? (e) => onNodeDragStart(row.id, e) : undefined}
              onDragOver={
                isDropTarget
                  ? (e) => {
                      e.preventDefault()
                      e.dataTransfer.dropEffect = 'move'
                      setDragOverId(row.id)
                    }
                  : undefined
              }
              onDragLeave={isDropTarget ? () => dragOverId === row.id && setDragOverId(null) : undefined}
              onDrop={
                isDropTarget
                  ? (e) => {
                      e.preventDefault()
                      e.stopPropagation()
                      onNodeDrop(row.id, e)
                      setDragOverId(null)
                    }
                  : undefined
              }
              actions={isEditor ? renderRowActions(row.id) : undefined}
              renameDefaultValue={row.label}
              onRenameCommit={(value) => onRenameCommit(row.id, value)}
              onRenameCancel={onRenameCancel}
            />
          </div>
        )
      })}
    </div>
  )
}
