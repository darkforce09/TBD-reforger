// Flat row model for the virtualized left outliner (T-064). The recursive TreeView mounts
// one <li> per visible node, which freezes at ~360k slot leaves; instead the sidebar is
// flattened into a small array of OutlinerSegments — fixed rows plus *runs* of slot ids —
// from which `rowAt` resolves an individual TreeRowModel on demand. The segment array is
// O(expanded folders + small inline subtrees), never O(total slots): a big expanded folder
// contributes a single `slots` segment that holds its live id array by reference.

import type { LucideIcon } from 'lucide-react'
import type { ID } from '@/features/tactical-map'

/** Fixed virtual row height (px). Required by @tanstack/react-virtual for the flat list. */
export const ROW_HEIGHT = 28

/** Left indent applied per depth level in the flat list (px). */
export const INDENT_PX = 14

export type TreeRowKind = 'section-header' | 'folder' | 'slot' | 'root-drop' | 'empty'
export type OutlinerSection = 'orbat' | 'editor'

export interface TreeRowModel {
  id: string
  label: string
  depth: number
  isFolder: boolean
  /** Folder has children OR a virtualSlotIds run (i.e. the chevron should show). */
  hasChildren: boolean
  isExpanded: boolean
  icon?: LucideIcon
  badge?: string
  kind: TreeRowKind
  section: OutlinerSection
}

/** A segment is either a single concrete row or a run of slot ids (resolved lazily by
 *  `rowAt` against the live `slotsById`), so a 360k-slot folder is ONE segment, not 360k. */
export type OutlinerSegment =
  | { kind: 'row'; row: TreeRowModel }
  | { kind: 'slots'; ids: ID[]; depth: number; parentId: ID; section: OutlinerSection }
