// "ORBAT" outliner data (the export-truth hierarchy Faction → Squad → Slot, Ultra Plan §5.1;
// Decisions log "both sections visible"). Read-only for now (no ORBAT-authoring UI): a slot
// row selects globally / double-clicks to Attributes, mirroring Editor Layers. Slots live in
// both trees — Editor Layers groups them by workflow folder, ORBAT by their export squad. As
// of T-064 rendering is owned by the virtualized VirtualOutliner (LeftSidebar); this module
// exposes `useOrbatOutliner`, the tree nodes (with virtualized leaves past the threshold).

import { useMemo } from 'react'
import { Flag, User, Users } from 'lucide-react'
import { useMapStore } from '@/features/tactical-map'
import type { Faction, ID, Slot, Squad } from '@/features/tactical-map'
import type { TreeNodeData } from '../tree/TreeView'
import { VIRTUAL_SLOT_THRESHOLD } from './EditorLayersSection'

function buildOrbat(
  factionsById: Record<ID, Faction>,
  squadsById: Record<ID, Squad>,
  slotsById: Record<ID, Slot>,
): TreeNodeData[] {
  return Object.values(factionsById).map((f) => ({
    id: f.id,
    label: f.name || f.key,
    icon: Flag,
    isFolder: true,
    defaultExpanded: true,
    children: f.squadIds
      .map((sid) => squadsById[sid])
      .filter((s): s is Squad => Boolean(s))
      .map((squad) => {
        // Past the threshold, virtualize the squad's slot leaves instead of mapping N rows —
        // rendering 10k+ rows froze the tab (T-059 cap → T-064 virtualization).
        const useVirtual = squad.slotIds.length > VIRTUAL_SLOT_THRESHOLD
        return {
          id: squad.id,
          label: squad.name,
          icon: Users,
          isFolder: true,
          defaultExpanded: true,
          children: useVirtual
            ? []
            : squad.slotIds
                .map((slid) => slotsById[slid])
                .filter((s): s is Slot => Boolean(s))
                .sort((a, b) => a.index - b.index)
                .map((s) => ({
                  id: s.id,
                  label: s.role || 'Slot',
                  icon: User,
                  ...(s.tag ? { badge: s.tag } : {}),
                })),
          // Preserve ORBAT order (by slot index) for the virtualized run.
          ...(useVirtual
            ? {
                virtualSlotIds: [...squad.slotIds].sort(
                  (a, b) => (slotsById[a]?.index ?? 0) - (slotsById[b]?.index ?? 0),
                ),
              }
            : {}),
        }
      }),
  }))
}

/** Hook: the ORBAT tree nodes (read-only). Selection/activation are handled centrally in
 *  LeftSidebar, shared with the Editor Layers tree. */
export function useOrbatOutliner() {
  const factionsById = useMapStore((s) => s.factionsById)
  const squadsById = useMapStore((s) => s.squadsById)
  const slotsById = useMapStore((s) => s.slotsById)
  // Rebuild signal for in-place slot add/remove, where slotsById's ref doesn't change (T-062.0.1).
  const slotsRevision = useMapStore((s) => s.slotsRevision)

  return useMemo(
    () => buildOrbat(factionsById, squadsById, slotsById),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [factionsById, squadsById, slotsById, slotsRevision],
  )
}
