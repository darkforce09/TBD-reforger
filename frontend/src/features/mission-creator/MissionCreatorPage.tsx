// Full-bleed route shell for the 2D Mission Creator. The Aegis-glass shell (Ultra Plan
// §5) frames the live map as floating frosted overlays: Top Command Strip, Left
// "Placed Entities" tree, Right Asset Browser tree, Bottom Toolbelt — plus the Mission
// Settings + Attributes modals. The route carries the `fullBleed` handle so AppLayout
// runs it full-height.

import { useCallback, useState } from 'react'
import { useParams } from 'react-router-dom'
import { TacticalMap, moveEntity, useMapStore } from '@/features/tactical-map'
import { useMissionDoc } from './hooks/useMissionDoc'
import { TopCommandStrip } from './layout/TopCommandStrip'
import { BottomToolbelt } from './layout/BottomToolbelt'
import { OutlinerPanel } from './layout/LeftOutliner/OutlinerPanel'
import { InspectorPanel } from './layout/RightInspector/InspectorPanel'
import { AttributesModal } from './layout/AttributesModal'
import { FpsCounter } from './FpsCounter'

export default function MissionCreatorPage() {
  const { id } = useParams<{ id: string }>()
  const { md, undo } = useMissionDoc(id)

  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null)
  const [attributesId, setAttributesId] = useState<string | null>(null)

  // Click empty map with a slot selected → reposition it. (Placement is drag-and-drop
  // from the Asset Browser; the Toolbelt no longer places — Ultra Plan §5.3.)
  const onMapClick = useCallback(
    (world: { x: number; y: number }) => {
      const { selection } = useMapStore.getState()
      if (selection.kind === 'slot' && selection.id) {
        moveEntity(md, 'slots', selection.id, world)
      }
    },
    [md],
  )

  return (
    <div className="relative h-full w-full overflow-hidden bg-background">
      {/* Full-bleed map behind everything. */}
      <TacticalMap
        terrain="everon"
        className="absolute inset-0 z-0"
        onMapClick={onMapClick}
        onCursorMove={setCursor}
        onEntityActivate={setAttributesId}
      />

      {/* Floating overlay layer: spans the screen and ignores pointer events so
          empty space (incl. the padded gaps) pans the map; each panel re-enables
          hits via `pointer-events-auto` in the `overlayPanel` recipe. */}
      <div className="pointer-events-none absolute inset-0 z-10">
        <div className="absolute inset-x-4 top-4">
          <TopCommandStrip md={md} undo={undo} />
        </div>

        <div className="absolute bottom-[5.5rem] left-4 top-[4.75rem]">
          <OutlinerPanel />
        </div>

        <div className="absolute bottom-[5.5rem] right-4 top-[4.75rem]">
          <InspectorPanel md={md} />
        </div>

        <div className="absolute bottom-5 left-1/2 -translate-x-1/2">
          <BottomToolbelt cursorWorld={cursor} />
        </div>

        <FpsCounter />
      </div>

      <AttributesModal
        slotId={attributesId}
        onOpenChange={(open) => !open && setAttributesId(null)}
      />
    </div>
  )
}
