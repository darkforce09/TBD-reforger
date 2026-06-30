// Bottom Toolbelt (Ultra Plan §5.3): tool buttons bound to the store's activeTool +
// a live X/Y/Z cursor read-out in JetBrains Mono. Phase 3 wires Select + Place Unit;
// Ruler / Line-of-Sight / Place Objective are visible placeholders (their tools land
// in Phase 8). Floating HudBar-style bar over the map.

import { memo, useEffect, useState } from 'react'
import { Eye, MousePointer2, Ruler } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { useMapStore, type ToolId } from '@/features/tactical-map'
import { cn } from '@/lib/utils'
import { estimateCompiledBytes, formatBytes } from '../lib/missionSize'
import { overlayPanel } from './overlay'

interface Tool {
  id: ToolId
  label: string
  icon: LucideIcon
  enabled: boolean
}

// Select is wired; Ruler / Line-of-Sight land in Phase 8. Unit placement is via the
// Asset Browser, not the Toolbelt (Ultra Plan §5.3).
const TOOLS: Tool[] = [
  { id: 'select', label: 'Select', icon: MousePointer2, enabled: true },
  { id: 'ruler', label: 'Ruler', icon: Ruler, enabled: false },
  { id: 'los', label: 'Line of Sight', icon: Eye, enabled: false },
]

function BottomToolbeltInner() {
  const activeTool = useMapStore((s) => s.activeTool)
  const setActiveTool = useMapStore((s) => s.setActiveTool)
  // The live cursor read-out is store-backed (T-057), so only this toolbelt re-renders on
  // pointer move — not the whole editor page.
  const cursorWorld = useMapStore((s) => s.cursor)

  // Selection-aware readout: when exactly one slot is selected, show its X/Y/Z; otherwise
  // show the live cursor X/Y/Z (cursor Z is 0 on the flat map until Phase 2 DEM). Off-map
  // hover → null cursor → every axis shows —.
  const selectedSlot = useMapStore((s) =>
    s.selection.kind === 'slot' && s.selection.ids.length === 1
      ? s.slotsById[s.selection.ids[0]]
      : undefined,
  )

  // Scale telemetry (T-058): OBJ = total placed slots, SEL = selected count. Both subscribe
  // here in the already-memoized toolbelt — they update on add/remove/paste/delete/selection,
  // never on a cursor move (the cursor lives in its own store slice, T-057). OBJ reads the
  // store's incrementally-maintained slotCount (T-062.0.1) rather than re-counting slotsById,
  // which the add/remove fast paths now mutate in place (ref unchanged).
  const totalSlots = useMapStore((s) => s.slotCount)
  const selectedCount = useMapStore((s) =>
    s.selection.kind === 'slot' ? s.selection.ids.length : 0,
  )

  // SZ = estimated compiled server-save size (T-060.1.3). Debounced 500ms on slot-count change so
  // we don't re-sample on every add/paste; the estimate samples slots (no full compile).
  const [estBytes, setEstBytes] = useState(0)
  useEffect(() => {
    const id = setTimeout(() => setEstBytes(estimateCompiledBytes(useMapStore.getState())), 500)
    return () => clearTimeout(id)
  }, [totalSlots])

  // X/Y/Z all read at 3 dp so manual anchor verification (e.g. coast-w 1000.000, 6400.000)
  // can confirm exact position; tabular-nums + padStart keep the columns aligned (T-091.2).
  const fmtCoord = (n: number) => n.toFixed(3).padStart(9, ' ')

  const showSel = selectedSlot != null
  const x = showSel ? selectedSlot.position.x : cursorWorld?.x
  const y = showSel ? selectedSlot.position.y : cursorWorld?.y
  const z = showSel ? selectedSlot.position.z : cursorWorld?.z

  return (
    <div className={cn(overlayPanel, 'flex items-center gap-1 px-1.5 py-1.5')}>
      {TOOLS.map((t) => {
        const active = activeTool === t.id
        return (
          <button
            key={t.id}
            type="button"
            disabled={!t.enabled}
            onClick={() => setActiveTool(t.id)}
            title={t.enabled ? t.label : `${t.label} (soon)`}
            aria-label={t.label}
            aria-pressed={active}
            className={cn(
              'flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors',
              active ? 'bg-primary/20 text-primary' : 'text-on-surface-variant hover:bg-white/10',
              !t.enabled && 'opacity-30 hover:bg-transparent',
            )}
          >
            <t.icon className="size-4" />
            <span className="hidden sm:inline">{t.label}</span>
          </button>
        )
      })}

      <span className="mx-1 h-5 w-px bg-white/10" />

      <div className="flex items-center gap-2 px-1 font-mono text-code-md text-on-surface-variant">
        <span className="text-outline" title={showSel ? 'Selected entity' : 'Cursor'}>
          {showSel ? 'SEL' : 'CUR'}
        </span>
        <span>
          X
          <span className="ml-1 text-on-surface tabular-nums">
            {x != null ? fmtCoord(x) : '       —'}
          </span>
        </span>
        <span>
          Y
          <span className="ml-1 text-on-surface tabular-nums">
            {y != null ? fmtCoord(y) : '       —'}
          </span>
        </span>
        <span>
          Z
          <span className="ml-1 text-on-surface tabular-nums">
            {z != null ? fmtCoord(z) : '       —'}
          </span>
        </span>
      </div>

      <span className="mx-1 h-5 w-px bg-white/10" />

      <div
        className="flex items-center gap-2 px-1 font-mono text-code-md tabular-nums text-on-surface-variant"
        title="Placed slots on map / current selection"
      >
        <span>
          OBJ<span className="ml-1 text-on-surface">{totalSlots}</span>
        </span>
        <span>
          SEL<span className="ml-1 text-on-surface">{selectedCount}</span>
        </span>
        <span title="Estimated server save size">
          SZ
          <span className="ml-1 text-on-surface">{estBytes > 0 ? formatBytes(estBytes) : '—'}</span>
        </span>
      </div>
    </div>
  )
}

export const BottomToolbelt = memo(BottomToolbeltInner)
