// Left panel — "Placed Entities" as an Eden-style recursive tree of custom folders
// (Ultra Plan §5.1). Visual shell only this phase: mock folder data + local selection
// highlight; real Y.Doc-backed layers + reparent DnD land later.

import { useState } from 'react'
import { FolderPlus } from 'lucide-react'
import { cn } from '@/lib/utils'
import { overlayPanel } from '../overlay'
import { TreeView } from '../tree/TreeView'
import { PLACED_ENTITIES } from './placedEntitiesMock'

export function OutlinerPanel() {
  const [selectedId, setSelectedId] = useState<string | null>(null)

  return (
    <div className={cn(overlayPanel, 'flex h-full w-60 flex-col overflow-hidden')}>
      <div className="flex shrink-0 items-center justify-between border-b border-white/5 px-3 py-2">
        <span className="text-label-sm uppercase tracking-wider text-outline">
          Placed Entities
        </span>
        <button
          type="button"
          aria-label="New folder"
          title="New folder"
          className="rounded p-0.5 text-on-surface-variant transition-colors hover:bg-primary/15 hover:text-primary"
        >
          <FolderPlus className="size-3.5" />
        </button>
      </div>
      <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-2">
        <TreeView
          nodes={PLACED_ENTITIES}
          selectedId={selectedId}
          onSelect={setSelectedId}
        />
      </div>
    </div>
  )
}
