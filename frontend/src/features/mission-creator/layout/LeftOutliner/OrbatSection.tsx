// "ORBAT" section of the left sidebar — the export-truth hierarchy Faction → Squad →
// Slot (Ultra Plan §5.1; Decisions log "both sections visible"). Read-only for now (no
// ORBAT-authoring UI yet): selecting a slot row selects it in global state, mirroring the
// Editor Layers tree. Slots live in both trees — Editor Layers groups them by workflow
// folder, ORBAT groups them by the squad they export under.

import { useMemo } from 'react'
import { Flag, User, Users } from 'lucide-react'
import { useMapStore } from '@/features/tactical-map'
import type { Faction, ID, Slot, Squad } from '@/features/tactical-map'
import { TreeView, type TreeNodeData } from '../tree/TreeView'
import { Section } from './EditorLayersSection'

function buildOrbat(
  factionsById: Record<ID, Faction>,
  squadsById: Record<ID, Squad>,
  slotsById: Record<ID, Slot>,
): TreeNodeData[] {
  return Object.values(factionsById).map((f) => ({
    id: f.id,
    label: f.name || f.key,
    icon: Flag,
    defaultExpanded: true,
    children: f.squadIds
      .map((sid) => squadsById[sid])
      .filter((s): s is Squad => Boolean(s))
      .map((squad) => ({
        id: squad.id,
        label: squad.name,
        icon: Users,
        defaultExpanded: true,
        children: squad.slotIds
          .map((slid) => slotsById[slid])
          .filter((s): s is Slot => Boolean(s))
          .sort((a, b) => a.index - b.index)
          .map((s) => ({
            id: s.id,
            label: s.role || 'Slot',
            icon: User,
            ...(s.tag ? { badge: s.tag } : {}),
          })),
      })),
  }))
}

export function OrbatSection() {
  const factionsById = useMapStore((s) => s.factionsById)
  const squadsById = useMapStore((s) => s.squadsById)
  const slotsById = useMapStore((s) => s.slotsById)
  const selection = useMapStore((s) => s.selection)
  const setSelection = useMapStore((s) => s.setSelection)

  const nodes = useMemo(
    () => buildOrbat(factionsById, squadsById, slotsById),
    [factionsById, squadsById, slotsById],
  )

  const selectedId = selection.kind === 'slot' ? selection.id : null

  const onSelect = (id: string) => {
    if (slotsById[id]) setSelection({ kind: 'slot', id })
  }

  return (
    <Section title="ORBAT">
      {nodes.length === 0 ? (
        <p className="px-2 py-3 text-center text-label-sm normal-case text-outline">
          No factions yet. Placed units are filed under a default squad.
        </p>
      ) : (
        <TreeView nodes={nodes} selectedId={selectedId} onSelect={onSelect} />
      )}
    </Section>
  )
}
