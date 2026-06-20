// Mock "Placed Entities" tree (Ultra Plan §5.1). Demonstrates that users can build any
// arbitrary folder structure (layers) to organize entities — these are workflow-only and
// don't affect the export. Replaced by real Y.Doc-backed layers in a later phase.

import { Folder, Shield, Stethoscope, Target, User } from 'lucide-react'
import type { TreeNodeData } from '../tree/TreeView'

export const PLACED_ENTITIES: TreeNodeData[] = [
  {
    id: 'f-blufor',
    label: 'BLUFOR',
    icon: Folder,
    defaultExpanded: true,
    children: [
      {
        id: 'f-assault',
        label: 'Assault Team',
        icon: Folder,
        defaultExpanded: true,
        children: [
          { id: 'e-a1', label: 'Alpha-1', icon: Shield, badge: 'SL' },
          { id: 'e-a2', label: 'Alpha-2', icon: User },
          { id: 'e-a3', label: 'Alpha-3', icon: Stethoscope, badge: 'MED' },
          { id: 'e-a4', label: 'Alpha-4', icon: User },
        ],
      },
      {
        id: 'f-support',
        label: 'Support',
        icon: Folder,
        children: [
          { id: 'e-mortar', label: 'Mortar Team', icon: Target, badge: 'IDF' },
          { id: 'e-mg', label: 'MG Nest', icon: Target, badge: 'MG' },
        ],
      },
    ],
  },
  {
    id: 'f-opfor',
    label: 'OPFOR',
    icon: Folder,
    children: [
      {
        id: 'f-recon',
        label: 'Recon',
        icon: Folder,
        children: [
          { id: 'e-sniper', label: 'Sniper', icon: Target, badge: 'DMR' },
          { id: 'e-spotter', label: 'Spotter', icon: User },
        ],
      },
    ],
  },
]
