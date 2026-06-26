// Left sidebar (Eden docked shell, Phase 3.5) — the docked "Outliner" panel. One scroll holds
// both hierarchies: ORBAT (Faction→Squad→Slot) on top, then Editor Layers (workflow folders).
// As of T-064 both trees are flattened into a single virtualized list (VirtualOutliner) over
// the one scroll container, so expanding a 360k-slot folder shows a scrollable list with only
// ~viewport DOM rows instead of freezing the tab. A bottom icon-tab strip is visual-only.
// T-064.1: the scroll container is handed to VirtualOutliner as callback-ref *state* (not a
// RefObject) — a child reads a RefObject before the parent attaches it, so the virtualizer
// would see null on first render and paint nothing until an unrelated re-render.

import { memo, useMemo, useState } from 'react'
import { Boxes, History, ListTree, Layers, Settings2 } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { useMapStore, type ID, type MissionDoc } from '@/features/tactical-map'
import { cn } from '@/lib/utils'
import { overlayDocked } from '../overlay'
import type { TreeNodeData } from '../tree/TreeView'
import type { TreeRowModel } from '../tree/treeRowModel'
import { buildOutlinerSegments } from '../tree/flattenOutliner'
import { VirtualOutliner } from '../tree/VirtualOutliner'
import { useEditorLayersOutliner } from './EditorLayersSection'
import { useOrbatOutliner } from './OrbatSection'

const BOTTOM_TABS: { id: string; icon: LucideIcon; label: string }[] = [
  { id: 'hierarchy', icon: ListTree, label: 'Hierarchy' },
  { id: 'layers', icon: Layers, label: 'Layers' },
  { id: 'assets', icon: Boxes, label: 'Assets' },
  { id: 'history', icon: History, label: 'History' },
  { id: 'settings', icon: Settings2, label: 'Settings' },
]

interface LeftSidebarProps {
  md: MissionDoc
  /** Double-click a slot row → open its Attributes modal. */
  onActivateSlot?: (id: ID) => void
}

/** Seed the expanded set from `defaultExpanded` folders, same as TreeView's mount-time pass.
 *  A mega-folder carries its leaves in `virtualSlotIds` (empty `children`), so folder-ness
 *  here mirrors flattenOutliner's walk(): inline children OR a virtualSlotIds run. */
function collectDefaultExpanded(nodes: TreeNodeData[], acc: Set<string>): Set<string> {
  for (const n of nodes) {
    const hasVirtual = !!n.virtualSlotIds?.length
    const hasInlineChildren = !!n.children?.length
    const isFolder = n.isFolder ?? (hasInlineChildren || hasVirtual)
    if (isFolder && n.defaultExpanded) acc.add(n.id)
    if (n.children) collectDefaultExpanded(n.children, acc)
  }
  return acc
}

function LeftSidebarInner({ md, onActivateSlot }: LeftSidebarProps) {
  const title = useMapStore((s) => s.meta?.title ?? 'Untitled Mission')
  const slotsById = useMapStore((s) => s.slotsById)
  const selection = useMapStore((s) => s.selection)
  const activeLayerId = useMapStore((s) => s.activeLayerId)
  const setSelection = useMapStore((s) => s.setSelection)
  const setActiveLayer = useMapStore((s) => s.setActiveLayer)

  const orbatNodes = useOrbatOutliner()
  const editor = useEditorLayersOutliner(md)

  // Callback-ref state for the scroll container (see file header / VirtualOutliner): null until
  // the div mounts, then the state flip re-runs the virtualizer so rows paint on load.
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null)

  // Expanded state lifted here (the flatten depends on it). Seeded once from `defaultExpanded`
  // folders — LeftSidebar mounts only after the doc is ready (MissionCreatorPage defers it), so
  // the lazy initializer captures the populated trees on first render. Seed-once (the initializer
  // runs once) means a folder the user later collapses stays collapsed across edits.
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(
    () => collectDefaultExpanded([...orbatNodes, ...editor.nodes], new Set()),
  )
  const toggle = (id: string) =>
    setExpanded((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const segments = useMemo(
    () => buildOutlinerSegments({ orbatNodes, editorNodes: editor.nodes, expanded }),
    [orbatNodes, editor.nodes, expanded],
  )

  // Slot multi-select highlight shared by both trees.
  const selectedIds = useMemo(
    () => (selection.kind === 'slot' ? new Set(selection.ids) : undefined),
    [selection],
  )

  // Unified select/activate across both trees: an Editor folder sets the active drop layer; a
  // slot (either tree) selects globally; ORBAT factions/squads only toggle (no select effect).
  const onSelectRow = (row: TreeRowModel) => {
    if (row.section === 'editor' && row.kind === 'folder') setActiveLayer(row.id)
    else if (row.kind === 'slot') setSelection({ kind: 'slot', ids: [row.id] })
  }
  const onActivateRow = (row: TreeRowModel) => {
    if (row.kind === 'slot') onActivateSlot?.(row.id)
  }

  return (
    <div className={cn(overlayDocked, 'flex h-full w-full flex-col overflow-hidden border-r border-white/10')}>
      <header className="shrink-0 border-b border-white/10 px-3 py-2">
        <p className="text-label-sm uppercase tracking-wider text-outline">Outliner</p>
        <p className="truncate text-label-md font-semibold text-on-surface">{title}</p>
      </header>

      <div ref={setScrollEl} className="custom-scrollbar min-h-0 flex-1 overflow-y-auto px-1 py-1">
        <VirtualOutliner
          scrollElement={scrollEl}
          segments={segments}
          slotsById={slotsById}
          toggle={toggle}
          activeLayerId={activeLayerId}
          selectedIds={selectedIds}
          onSelectRow={onSelectRow}
          onActivateRow={onActivateRow}
          onNodeDragStart={editor.onNodeDragStart}
          onNodeDrop={editor.onNodeDrop}
          onRootDrop={editor.onRootDrop}
          renderRowActions={editor.renderRowActions}
          renamingId={editor.renamingId}
          onRenameCommit={editor.onRenameCommit}
          onRenameCancel={editor.onRenameCancel}
          onNewFolder={editor.newFolder}
        />
      </div>

      <nav className="flex shrink-0 items-center justify-around border-t border-white/10 px-1 py-1.5">
        {BOTTOM_TABS.map((t, i) => (
          <button
            key={t.id}
            type="button"
            aria-label={t.label}
            title={t.label}
            className={cn(
              'rounded-md p-1.5 transition-colors',
              i === 0
                ? 'bg-primary/15 text-primary'
                : 'text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
            )}
          >
            <t.icon className="size-4" />
          </button>
        ))}
      </nav>
    </div>
  )
}

// Memoized (T-057): its props (md + stable setAttributesId) don't change per frame, so a
// host re-render won't rebuild the Outliner trees.
export const LeftSidebar = memo(LeftSidebarInner)
