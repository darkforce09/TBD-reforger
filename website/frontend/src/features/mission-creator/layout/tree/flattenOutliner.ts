// Flatten the two outliner trees (ORBAT + Editor Layers) into an OutlinerSegment[] for the
// virtualized list (T-064). Walks the small TreeNodeData[] honouring the lifted `expanded`
// set; an expanded folder with `virtualSlotIds` contributes a single `slots` segment (its
// live id array by reference) rather than thousands of row objects. `rowAt` resolves a given
// flat row index to a TreeRowModel, materializing slot rows from `slotsById` on demand.

import { User } from 'lucide-react'
import type { ID, Slot } from '@/features/tactical-map'
import type { TreeNodeData } from './TreeView'
import type { OutlinerSection, OutlinerSegment, TreeRowModel } from './treeRowModel'

interface BuildOpts {
  orbatNodes: TreeNodeData[]
  editorNodes: TreeNodeData[]
  expanded: ReadonlySet<string>
}

function emptyRow(label: string, section: OutlinerSection): OutlinerSegment {
  return {
    kind: 'row',
    row: {
      id: `__empty_${section}`,
      label,
      depth: 0,
      isFolder: false,
      hasChildren: false,
      isExpanded: false,
      kind: 'empty',
      section,
    },
  }
}

function headerRow(label: string, section: OutlinerSection): OutlinerSegment {
  return {
    kind: 'row',
    row: {
      id: `__header_${section}`,
      label,
      depth: 0,
      isFolder: false,
      hasChildren: false,
      isExpanded: false,
      kind: 'section-header',
      section,
    },
  }
}

/** Recursively emit segments for a node list. A folder emits its own row; when expanded it
 *  emits its inline children (recurse) and/or a single `slots` segment for virtualSlotIds. */
function walk(
  nodes: TreeNodeData[],
  section: OutlinerSection,
  depth: number,
  expanded: ReadonlySet<string>,
  out: OutlinerSegment[],
): void {
  for (const n of nodes) {
    const hasVirtual = !!n.virtualSlotIds?.length
    const hasInlineChildren = !!n.children?.length
    const isFolder = n.isFolder ?? (hasInlineChildren || hasVirtual)
    const hasChildren = hasInlineChildren || hasVirtual
    const isExpanded = expanded.has(n.id)
    out.push({
      kind: 'row',
      row: {
        id: n.id,
        label: n.label,
        depth,
        isFolder,
        hasChildren,
        isExpanded,
        icon: n.icon,
        badge: n.badge,
        kind: isFolder ? 'folder' : 'slot',
        section,
      },
    })
    if (isExpanded) {
      if (hasInlineChildren) walk(n.children!, section, depth + 1, expanded, out)
      if (hasVirtual) {
        out.push({ kind: 'slots', ids: n.virtualSlotIds!, depth: depth + 1, parentId: n.id, section })
      }
    }
  }
}

/** Build the full segment list: ORBAT header + tree, then Editor Layers header + tree +
 *  trailing root-drop row. Segment count is O(expanded folders + small inline subtrees). */
export function buildOutlinerSegments({ orbatNodes, editorNodes, expanded }: BuildOpts): OutlinerSegment[] {
  const out: OutlinerSegment[] = []

  out.push(headerRow('ORBAT', 'orbat'))
  if (orbatNodes.length === 0) {
    out.push(emptyRow('No factions yet. Placed units are filed under a default squad.', 'orbat'))
  } else {
    walk(orbatNodes, 'orbat', 0, expanded, out)
  }

  out.push(headerRow('Editor Layers', 'editor'))
  if (editorNodes.length === 0) {
    out.push(emptyRow('No entities yet. Drag an asset from the right panel onto the map.', 'editor'))
  } else {
    walk(editorNodes, 'editor', 0, expanded, out)
    out.push({
      kind: 'row',
      row: {
        id: '__root_drop',
        label: 'Move folder to root',
        depth: 0,
        isFolder: false,
        hasChildren: false,
        isExpanded: false,
        kind: 'root-drop',
        section: 'editor',
      },
    })
  }

  return out
}

/** Total flat row count: 1 per `row` segment, ids.length per `slots` run. */
export function totalRowCount(segments: OutlinerSegment[]): number {
  let n = 0
  for (const seg of segments) n += seg.kind === 'row' ? 1 : seg.ids.length
  return n
}

/** Resolve a flat row index to its TreeRowModel. Linear over segments (O(folders), not
 *  O(slots)); slot rows in a `slots` run are materialized from `slotsById` on demand. */
export function rowAt(
  segments: OutlinerSegment[],
  index: number,
  slotsById: Record<ID, Slot>,
): TreeRowModel | null {
  let offset = 0
  for (const seg of segments) {
    if (seg.kind === 'row') {
      if (index === offset) return seg.row
      offset += 1
      continue
    }
    const len = seg.ids.length
    if (index < offset + len) {
      const id = seg.ids[index - offset]
      const slot = slotsById[id]
      return {
        id,
        label: slot?.role || (seg.section === 'orbat' ? 'Slot' : 'Unit'),
        depth: seg.depth,
        isFolder: false,
        hasChildren: false,
        isExpanded: false,
        icon: User,
        badge: slot?.tag,
        kind: 'slot',
        section: seg.section,
      }
    }
    offset += len
  }
  return null
}
