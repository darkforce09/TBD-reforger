// T-068.3 — flat registry → Asset Browser tree. Turns the GET /api/v1/registry
// `character` rows into the nested Faction → Category tree the right palette renders.
// Each row's `category` is a slash path (e.g. "NATO/US_Army/Rifleman"); the folders come
// from all-but-the-last segment, and the leaf is the row's display_name with its id set to
// the full Enfusion ResourceName (resource_name) so a drop carries the real classname.

import { Folder, User } from 'lucide-react'
import type { RegistryItem } from '@/types/models/registry'
import type { TreeNodeData } from '../layout/tree/TreeView'

// Build the palette tree. Only `character` rows are placed; gear_* rows feed the Arsenal
// loadout dropdowns (T-068.4), not the map palette. Items arrive pre-sorted by sort_order,
// so building in array order keeps faction/role order stable.
export function buildCatalogTree(items: RegistryItem[]): TreeNodeData[] {
  const roots: TreeNodeData[] = []
  // Folder lookup keyed by accumulated path prefix ("NATO", "NATO/US_Army") for stable ids.
  const folders = new Map<string, TreeNodeData>()

  for (const item of items) {
    if (item.kind !== 'character') continue

    const segs = item.category.split('/').filter(Boolean)
    const folderSegs = segs.slice(0, -1) // drop the role segment; display_name is the leaf

    let parentChildren = roots
    let prefix = ''
    for (let i = 0; i < folderSegs.length; i++) {
      prefix = prefix ? `${prefix}/${folderSegs[i]}` : folderSegs[i]
      let folder = folders.get(prefix)
      if (!folder) {
        folder = {
          id: prefix,
          label: folderSegs[i],
          icon: Folder,
          defaultExpanded: i === 0, // top-level faction folders open by default
          children: [],
        }
        folders.set(prefix, folder)
        parentChildren.push(folder)
      }
      parentChildren = folder.children!
    }

    parentChildren.push({
      id: item.resource_name,
      label: item.display_name,
      icon: User,
    })
  }

  return roots
}
