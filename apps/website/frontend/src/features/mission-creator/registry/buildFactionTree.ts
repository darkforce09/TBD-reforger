// Faction palette tree (T-153) — replaces the raw vanilla registry dump: side folders
// (BLUFOR/OPFOR/INDFOR/CIV) → operator-authored library factions → Roles (draggable ORBAT
// templates carrying assetId + tag + pre-authored SlotLoadout v2 + factionRef) and
// Vehicles (listed; map placement lands with T-070, so vehicle leaves carry no payload).

import { Car, Flag, User, Users } from 'lucide-react'
import type { AssetDropPayload } from '@/features/tactical-map'
import type { FactionSide, UserFaction } from '@/types/models/faction'
import { FACTION_SIDES } from '@/types/models/faction'
import type { TreeNodeData } from '../layout/tree/TreeView'

/** Side accent classes (V4 ledger: Aegis tokens — BLUFOR primary, OPFOR error,
 *  INDFOR success, CIV outline). Applied to the side folder's Flag icon. */
export const SIDE_ICON_CLASS: Record<FactionSide, string> = {
  BLUFOR: 'text-primary',
  OPFOR: 'text-error',
  INDFOR: 'text-success',
  CIV: 'text-outline',
}

export interface FactionTree {
  nodes: TreeNodeData[]
  /** Drag payload per draggable leaf id (role leaves only — vehicle leaves are listed
   *  but not placeable until T-070). */
  payloadById: Map<string, AssetDropPayload>
}

export function buildFactionTree(factions: readonly UserFaction[]): FactionTree {
  const payloadById = new Map<string, AssetDropPayload>()
  const nodes: TreeNodeData[] = []

  for (const side of FACTION_SIDES) {
    const own = factions.filter((f) => f.side === side)
    if (own.length === 0) continue

    const factionNodes: TreeNodeData[] = own.map((f) => {
      const roleLeaves: TreeNodeData[] = f.doc.roles.map((r, idx) => {
        const id = `role:${f.id}:${idx}`
        payloadById.set(id, {
          assetId: r.character,
          role: r.role,
          kind: 'slot',
          ...(r.tag ? { tag: r.tag } : {}),
          ...(r.loadout ? { loadout: r.loadout } : {}),
          factionRef: { side: f.side, name: f.name },
        })
        return {
          id,
          label: r.role,
          icon: User,
          ...(r.tag ? { badge: r.tag } : {}),
        }
      })

      const vehicleLeaves: TreeNodeData[] = f.doc.vehicles.map((v, idx) => ({
        id: `veh:${f.id}:${idx}`,
        label: v.label || v.vehicle.split('/').at(-1)?.replace('.et', '') || 'Vehicle',
        icon: Car,
        badge: 'T-070',
      }))

      const children: TreeNodeData[] = []
      if (roleLeaves.length) {
        children.push({
          id: `roles:${f.id}`,
          label: 'Roles',
          icon: Users,
          isFolder: true,
          defaultExpanded: true,
          children: roleLeaves,
        })
      }
      if (vehicleLeaves.length) {
        children.push({
          id: `vehicles:${f.id}`,
          label: 'Vehicles',
          icon: Car,
          isFolder: true,
          children: vehicleLeaves,
        })
      }
      return {
        id: `faction:${f.id}`,
        label: f.name,
        icon: Flag,
        isFolder: true,
        defaultExpanded: true,
        children,
      }
    })

    nodes.push({
      id: `side:${side}`,
      label: side,
      icon: Flag,
      iconClassName: SIDE_ICON_CLASS[side],
      isFolder: true,
      defaultExpanded: true,
      children: factionNodes,
    })
  }

  return { nodes, payloadById }
}
