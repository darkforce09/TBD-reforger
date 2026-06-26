// "Editor Layers" outliner data + handlers (the Eden workflow-folder tree, Ultra Plan §5.1),
// bound to the live Y.Doc `editorLayers` map. As of T-064 the rendering is owned by the
// virtualized VirtualOutliner (in LeftSidebar); this module exposes `useEditorLayersOutliner`
// — the folder tree (`buildTree`) plus reparent DnD, row actions (rename/delete) and the
// "new folder" action. Selecting a folder makes it the active drop target; selecting a slot
// selects it globally (no auto camera move); double-clicking a slot opens its Attributes modal.

import { useMemo, useState } from 'react'
import { Folder, Pencil, Trash2, User } from 'lucide-react'
import {
  addEditorLayer,
  moveSlotToLayer,
  removeEditorLayer,
  removeEntity,
  renameEditorLayer,
  reparentEditorLayer,
  useMapStore,
} from '@/features/tactical-map'
import type { EditorLayer, ID, MissionDoc, Slot } from '@/features/tactical-map'
import { cn } from '@/lib/utils'
import type { TreeNodeData } from '../tree/TreeView'

/** dataTransfer MIME for internal outliner reparent drags (distinct from the asset-onto-map
 *  drag, ASSET_DND_MIME). Payload: which kind of node and its id. */
export const TREE_MIME = 'application/x-tbd-tree-node'
interface TreeDragPayload {
  kind: 'layer' | 'slot'
  id: ID
}

/** Above this many slots a folder/squad virtualizes its leaves (`virtualSlotIds`) instead of
 *  materializing per-slot `TreeNodeData` children — building 360k node objects (and rendering
 *  the rows) froze the tab (T-059 cap → T-064 virtualization). The same threshold is applied
 *  per-squad in the ORBAT tree (OrbatSection). */
export const VIRTUAL_SLOT_THRESHOLD = 50

/** Build the recursive tree: each EditorLayer → a folder node containing its child folders
 *  then its placed slots. Layers nest via parentId; null = root. Past the threshold the slot
 *  leaves are virtualized (`virtualSlotIds`) rather than mapped into `children`. */
function buildTree(
  layersById: Record<ID, EditorLayer>,
  slotsById: Record<ID, Slot>,
): TreeNodeData[] {
  const all = Object.values(layersById)
  const build = (layer: EditorLayer): TreeNodeData => {
    const childFolders = all.filter((l) => l.parentId === layer.id).map(build)
    const useVirtual = layer.entityIds.length > VIRTUAL_SLOT_THRESHOLD
    const entityNodes: TreeNodeData[] = useVirtual
      ? []
      : layer.entityIds
          .map((eid) => slotsById[eid])
          .filter((s): s is Slot => Boolean(s))
          .map((s) => ({
            id: s.id,
            label: s.role || 'Unit',
            icon: User,
            ...(s.tag ? { badge: s.tag } : {}),
          }))
    return {
      id: layer.id,
      label: layer.name,
      icon: Folder,
      isFolder: true,
      defaultExpanded: true,
      children: [...childFolders, ...entityNodes],
      ...(useVirtual ? { virtualSlotIds: layer.entityIds } : {}),
    }
  }
  return all.filter((l) => l.parentId === null).map(build)
}

/** Hook: the Editor Layers tree nodes + every editor-only interaction the VirtualOutliner
 *  needs (reparent DnD, row hover actions, inline rename, "new folder"). */
export function useEditorLayersOutliner(md: MissionDoc) {
  const layersById = useMapStore((s) => s.editorLayersById)
  const slotsById = useMapStore((s) => s.slotsById)
  // Rebuild signal for in-place slot add/remove, where slotsById's ref doesn't change (T-062.0.1).
  const slotsRevision = useMapStore((s) => s.slotsRevision)
  const setSelection = useMapStore((s) => s.setSelection)
  const setActiveLayer = useMapStore((s) => s.setActiveLayer)
  const [renamingId, setRenamingId] = useState<string | null>(null)

  // slotsRevision: rebuild when slotsById is mutated in place (add/remove) without a ref change.
  const nodes = useMemo(
    () => buildTree(layersById, slotsById),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [layersById, slotsById, slotsRevision],
  )

  // ── Reparent drag-and-drop ────────────────────────────────────────────────
  const onNodeDragStart = (id: ID, e: React.DragEvent) => {
    const payload: TreeDragPayload = { kind: layersById[id] ? 'layer' : 'slot', id }
    e.dataTransfer.setData(TREE_MIME, JSON.stringify(payload))
    e.dataTransfer.effectAllowed = 'move'
  }

  const readPayload = (e: React.DragEvent): TreeDragPayload | null => {
    const raw = e.dataTransfer.getData(TREE_MIME)
    if (!raw) return null
    try {
      return JSON.parse(raw) as TreeDragPayload
    } catch {
      return null
    }
  }

  // Drop onto a folder → reparent a folder / refile a slot under it.
  const onNodeDrop = (targetId: ID, e: React.DragEvent) => {
    if (!layersById[targetId]) return
    const p = readPayload(e)
    if (!p) return
    if (p.kind === 'layer') reparentEditorLayer(md, p.id, targetId)
    else moveSlotToLayer(md, p.id, targetId)
  }

  // Drop on empty tree space → a folder returns to root (slots always live in a folder).
  const onRootDrop = (e: React.DragEvent) => {
    const p = readPayload(e)
    if (p?.kind === 'layer') reparentEditorLayer(md, p.id, null)
  }

  // ── Row actions + inline rename ───────────────────────────────────────────
  const deleteSlot = (id: ID) => {
    removeEntity(md, 'slots', id)
    const sel = useMapStore.getState().selection
    if (sel.kind === 'slot' && sel.ids.includes(id)) {
      setSelection({ kind: 'slot', ids: sel.ids.filter((s) => s !== id) })
    }
  }

  const deleteFolder = (id: ID) => {
    removeEditorLayer(md, id)
    // Drop a stale active-folder pointer so new placements fall back to the default layer.
    if (useMapStore.getState().activeLayerId === id) setActiveLayer(null)
    // The destructive delete likely removed selected units — clear the selection.
    if (useMapStore.getState().selection.kind !== 'none') setSelection({ kind: 'none', ids: [] })
  }

  const renderRowActions = (id: ID): React.ReactNode => {
    const stop = (fn: () => void) => (e: React.MouseEvent) => {
      e.stopPropagation()
      fn()
    }
    const btn = 'rounded p-0.5 text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface'
    if (layersById[id]) {
      return (
        <>
          <button type="button" aria-label="Rename folder" title="Rename" className={btn} onClick={stop(() => setRenamingId(id))}>
            <Pencil className="size-3" />
          </button>
          <button type="button" aria-label="Delete folder" title="Delete folder" className={cn(btn, 'hover:text-error')} onClick={stop(() => deleteFolder(id))}>
            <Trash2 className="size-3" />
          </button>
        </>
      )
    }
    if (slotsById[id]) {
      return (
        <button type="button" aria-label="Delete unit" title="Delete unit" className={cn(btn, 'hover:text-error')} onClick={stop(() => deleteSlot(id))}>
          <Trash2 className="size-3" />
        </button>
      )
    }
    return null
  }

  const onRenameCommit = (id: ID, name: string) => {
    const trimmed = name.trim()
    if (trimmed && trimmed !== layersById[id]?.name) renameEditorLayer(md, id, trimmed)
    setRenamingId(null)
  }

  const newFolder = () => setActiveLayer(addEditorLayer(md))

  return {
    nodes,
    onNodeDragStart,
    onNodeDrop,
    onRootDrop,
    renderRowActions,
    renamingId,
    onRenameCommit,
    onRenameCancel: () => setRenamingId(null),
    newFolder,
  }
}
