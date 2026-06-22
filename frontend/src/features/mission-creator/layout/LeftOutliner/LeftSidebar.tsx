// Left sidebar (Eden docked shell, Phase 3.5) — the docked "Outliner" panel. One scroll
// holds both export-truth + workflow hierarchies: ORBAT (Faction→Squad→Slot) on top, then
// Editor Layers (workflow folders), then stub sections for tools that land in Phase 8. A
// bottom icon-tab strip (Hierarchy/Layers/Assets/History/Settings) is visual-only for now.

import { Boxes, History, ListTree, Layers, Settings2 } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { useMapStore, type ID, type MissionDoc } from '@/features/tactical-map'
import { cn } from '@/lib/utils'
import { overlayDocked } from '../overlay'
import { EditorLayersSection } from './EditorLayersSection'
import { OrbatSection } from './OrbatSection'

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

export function LeftSidebar({ md, onActivateSlot }: LeftSidebarProps) {
  const title = useMapStore((s) => s.meta?.title ?? 'Untitled Mission')

  return (
    <div className={cn(overlayDocked, 'flex h-full w-full flex-col overflow-hidden border-r border-white/10')}>
      <header className="shrink-0 border-b border-white/10 px-3 py-2">
        <p className="text-label-sm uppercase tracking-wider text-outline">Outliner</p>
        <p className="truncate text-label-md font-semibold text-on-surface">{title}</p>
      </header>

      <div className="custom-scrollbar min-h-0 flex-1 divide-y divide-white/5 overflow-y-auto py-1">
        <OrbatSection onActivateSlot={onActivateSlot} />
        <EditorLayersSection md={md} onActivateSlot={onActivateSlot} />
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
