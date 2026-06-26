// Mock asset catalog tree (Ultra Plan §5.2). Nested Faction → Category → Class hierarchy
// mimicking the Eden Editor asset browser; users will drag leaves onto the map. Replaced
// by the registry-backed catalog (GET /api/v1/registry) in a later phase.

import { Box, Car, Folder, Shield, Truck, User } from 'lucide-react'
import type { TreeNodeData } from '../tree/TreeView'

export const ASSET_CATALOG: TreeNodeData[] = [
  {
    id: 'c-nato',
    label: 'NATO',
    icon: Folder,
    defaultExpanded: true,
    children: [
      {
        id: 'c-nato-men',
        label: 'Men',
        icon: Folder,
        defaultExpanded: true,
        children: [
          { id: 'a-nato-rifleman', label: 'Rifleman', icon: User },
          { id: 'a-nato-sl', label: 'Squad Leader', icon: Shield },
          { id: 'a-nato-medic', label: 'Medic', icon: User },
          { id: 'a-nato-ar', label: 'Autorifleman', icon: User },
          { id: 'a-nato-marksman', label: 'Marksman', icon: User },
        ],
      },
      {
        id: 'c-nato-veh',
        label: 'Vehicles',
        icon: Folder,
        children: [
          {
            id: 'c-nato-cars',
            label: 'Cars',
            icon: Folder,
            children: [
              { id: 'a-nato-hunter', label: 'MRAP (Hunter)', icon: Car },
              { id: 'a-nato-prowler', label: 'LSV (Prowler)', icon: Car },
            ],
          },
          {
            id: 'c-nato-armor',
            label: 'Armored',
            icon: Folder,
            children: [{ id: 'a-nato-ifv', label: 'IFV (Marshall)', icon: Truck }],
          },
        ],
      },
      {
        id: 'c-nato-obj',
        label: 'Objects',
        icon: Folder,
        children: [
          { id: 'a-nato-sandbag', label: 'Sandbag Wall', icon: Box },
          { id: 'a-nato-hbarrier', label: 'H-Barrier', icon: Box },
        ],
      },
    ],
  },
  {
    id: 'c-csat',
    label: 'CSAT',
    icon: Folder,
    children: [
      {
        id: 'c-csat-men',
        label: 'Men',
        icon: Folder,
        children: [
          { id: 'a-csat-rifleman', label: 'Rifleman', icon: User },
          { id: 'a-csat-sl', label: 'Squad Leader', icon: Shield },
        ],
      },
    ],
  },
  {
    id: 'c-empty',
    label: 'Empty / Props',
    icon: Folder,
    children: [{ id: 'a-ammobox', label: 'Ammo Box', icon: Box }],
  },
]
