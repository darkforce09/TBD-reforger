// Presentational outliner row — the chevron / icon / label / badge / hover-actions / inline
// rename chrome shared by the recursive TreeView (Asset Browser) and the virtualized
// VirtualOutliner (left sidebar, T-064). Pure: every behaviour comes in as a pre-bound
// prop so the row stays agnostic to whether it was produced from a TreeNodeData (recursive)
// or a flattened TreeRowModel (virtual). The recursive tree relies on nested <ul> margins
// for indentation (indentPx omitted); the flat virtual list passes indentPx (depth-derived).

import { ChevronRight, FolderOpen } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { cn } from '@/lib/utils'

export interface TreeRowProps {
  label: string
  icon?: LucideIcon
  badge?: string
  isFolder: boolean
  hasChildren: boolean
  isOpen: boolean
  selected: boolean
  draggable: boolean
  isDropTarget: boolean
  dragOver: boolean
  renaming: boolean
  /** Flat (virtual) mode: left padding in px derived from depth. Omit for recursive mode
   *  (nested <ul> margins provide the indent). */
  indentPx?: number
  onRowClick: () => void
  onRowDoubleClick: () => void
  onDragStart?: (e: React.DragEvent) => void
  onDragOver?: (e: React.DragEvent) => void
  onDragLeave?: (e: React.DragEvent) => void
  onDrop?: (e: React.DragEvent) => void
  /** Right-aligned hover actions (rename/delete buttons). */
  actions?: React.ReactNode
  renameDefaultValue?: string
  onRenameCommit?: (value: string) => void
  onRenameCancel?: () => void
}

export function TreeRow({
  label,
  icon,
  badge,
  isFolder,
  hasChildren,
  isOpen,
  selected,
  draggable,
  isDropTarget,
  dragOver,
  renaming,
  indentPx,
  onRowClick,
  onRowDoubleClick,
  onDragStart,
  onDragOver,
  onDragLeave,
  onDrop,
  actions,
  renameDefaultValue,
  onRenameCommit,
  onRenameCancel,
}: TreeRowProps) {
  // Folders show an "open" glyph when expanded; leaves keep their own icon.
  const Icon = isFolder && isOpen && hasChildren ? FolderOpen : icon

  return (
    <div
      draggable={draggable}
      onDragStart={draggable ? onDragStart : undefined}
      onDragOver={isDropTarget ? onDragOver : undefined}
      onDragLeave={isDropTarget ? onDragLeave : undefined}
      onDrop={isDropTarget ? onDrop : undefined}
      onClick={onRowClick}
      onDoubleClick={onRowDoubleClick}
      style={indentPx !== undefined ? { paddingLeft: indentPx } : undefined}
      className={cn(
        'group flex items-center gap-1.5 rounded-md border-l-2 py-1 pr-2 pl-1.5 text-label-md transition-colors',
        draggable ? 'cursor-grab' : 'cursor-pointer',
        selected
          ? 'border-primary bg-primary/15 text-on-surface'
          : 'border-transparent text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
        dragOver && 'ring-1 ring-inset ring-primary',
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
          defaultValue={renameDefaultValue}
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            if (e.key === 'Enter') onRenameCommit?.((e.target as HTMLInputElement).value)
            else if (e.key === 'Escape') onRenameCancel?.()
          }}
          onBlur={(e) => onRenameCommit?.(e.target.value)}
          className="min-w-0 flex-1 rounded bg-surface-container-lowest/60 px-1 text-on-surface outline-none ring-1 ring-primary/60"
        />
      ) : (
        <span className={cn('min-w-0 flex-1 truncate', isFolder && 'font-medium')}>{label}</span>
      )}

      {!renaming && badge && (
        <span className="shrink-0 rounded bg-surface-variant/60 px-1.5 text-label-sm text-outline">
          {badge}
        </span>
      )}
      {!renaming && actions && (
        <span className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
          {actions}
        </span>
      )}
    </div>
  )
}
