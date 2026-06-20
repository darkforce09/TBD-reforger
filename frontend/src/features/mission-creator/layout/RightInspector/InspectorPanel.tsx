// Right panel — defaults to the Asset Browser and switches to a contextual Inspector
// on selection (Ultra Plan §5.2). Phase 3 handles `none` (Asset Browser) and `slot`
// (SlotInspector — the ArsenalInspector stub); objective/vehicle inspectors arrive
// with their tools later. Mission Settings live in a dedicated dialog, not here.

import { useMapStore, type MissionDoc } from '@/features/tactical-map'
import { cn } from '@/lib/utils'
import { overlayPanel } from '../overlay'
import { AssetBrowser } from './AssetBrowser'
import { SlotInspector } from './SlotInspector'

export function InspectorPanel({ md }: { md: MissionDoc }) {
  const selection = useMapStore((s) => s.selection)
  const slot = useMapStore((s) =>
    selection.kind === 'slot' && selection.id ? s.slotsById[selection.id] : undefined,
  )

  return (
    <div className={cn(overlayPanel, 'flex h-full w-72 flex-col overflow-hidden')}>
      <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-4">
        {slot ? <SlotInspector md={md} slot={slot} /> : <AssetBrowser />}
      </div>
    </div>
  )
}
