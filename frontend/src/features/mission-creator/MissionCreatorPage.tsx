// Full-bleed route shell for the 2D Mission Creator. Phase 4 mounts the live map +
// the Y.Doc state foundation (via useMissionDoc) with a temporary debug harness for
// add/move/undo/persist. The real Top Command Strip / Outliner / Inspector / Toolbelt
// (Ultra Plan §5) arrive in later phases. The route carries the `fullBleed` handle so
// AppLayout runs this page full-height with no padding.

import { useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { TacticalMap, moveEntity, useMapStore } from '@/features/tactical-map'
import { useMissionDoc } from './hooks/useMissionDoc'
import { DebugToolbar } from './DebugToolbar'
import { FpsCounter } from './FpsCounter'

export default function MissionCreatorPage() {
  const { id } = useParams<{ id: string }>()
  const { md, undo } = useMissionDoc(id)

  // With a slot selected, clicking empty map moves it there (Phase-7 drag stand-in).
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
      <TacticalMap terrain="everon" onMapClick={onMapClick} />

      <FpsCounter />
      <DebugToolbar md={md} undo={undo} />

      {/* Minimal non-functional marker so the route is self-evidently the editor.
          Replaced by the Top Command Strip in a later phase. */}
      <div className="glass pointer-events-none absolute left-4 top-4 z-10 rounded-md px-3 py-1.5 font-mono text-code-md text-on-surface-variant">
        Mission Creator · {id ?? '—'}
      </div>
    </div>
  )
}
