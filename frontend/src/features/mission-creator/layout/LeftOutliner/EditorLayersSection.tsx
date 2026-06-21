// "Editor Layers" section of the left sidebar — the Eden workflow-folder tree (Ultra
// Plan §5.1), bound to the live Y.Doc `editorLayers` map. Renders the folder hierarchy
// with the real slots filed inside each folder: selecting a slot selects it in global
// state (no auto camera move — Spacebar centers, Phase 3.5 Task 7); selecting a folder
// makes it the active drop target; the "+" creates a new folder; double-clicking a slot
// opens its Attributes modal. Reparent drag-and-drop is Phase 7a.

import { useMemo, useState } from 'react'
import { Folder, FolderPlus, Pencil, Trash2, User } from 'lucide-react'
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
import { TreeView, type TreeNodeData } from '../tree/TreeView'

interface EditorLayersSectionProps {
  md: MissionDoc
  onActivateSlot?: (id: ID) => void
}

/** dataTransfer MIME for internal outliner reparent drags (distinct from the asset-onto-map
 *  drag, ASSET_DND_MIME). Payload: which kind of node and its id. */
const TREE_MIME = 'application/x-tbd-tree-node'
interface TreeDragPayload {
  kind: 'layer' | 'slot'
  id: ID
}

/** Build the recursive tree: each EditorLayer → a folder node containing its child
 *  folders then its placed slots. Layers nest via parentId; null = root. */
function buildTree(
  layersById: Record<ID, EditorLayer>,
  slotsById: Record<ID, Slot>,
): TreeNodeData[] {
  const all = Object.values(layersById)
  const build = (layer: EditorLayer): TreeNodeData => {
    const childFolders = all.filter((l) => l.parentId === layer.id).map(build)
    const entityNodes: TreeNodeData[] = layer.entityIds
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
    }
  }
  return all.filter((l) => l.parentId === null).map(build)
}

export function EditorLayersSection({ md, onActivateSlot }: EditorLayersSectionProps) {
  const layersById = useMapStore((s) => s.editorLayersById)
  const slotsById = useMapStore((s) => s.slotsById)
  const selection = useMapStore((s) => s.selection)
  const activeLayerId = useMapStore((s) => s.activeLayerId)
  const setSelection = useMapStore((s) => s.setSelection)
  const setActiveLayer = useMapStore((s) => s.setActiveLayer)
  const [renamingId, setRenamingId] = useState<string | null>(null)
  const [rootOver, setRootOver] = useState(false)

  const nodes = useMemo(() => buildTree(layersById, slotsById), [layersById, slotsById])

  // Highlight selected slots (multi-select) plus the active folder.
  const selectedIds = useMemo(
    () => (selection.kind === 'slot' ? new Set(selection.ids) : undefined),
    [selection],
  )

  const onSelect = (id: string) => {
    if (layersById[id]) {
      setActiveLayer(id)
      return
    }
    if (slotsById[id]) setSelection({ kind: 'slot', ids: [id] })
  }

  const onActivate = (id: string) => {
    if (slotsById[id]) onActivateSlot?.(id)
  }

  // ── Reparent drag-and-drop ────────────────────────────────────────────────
  const onNodeDragStart = (node: TreeNodeData, e: React.DragEvent) => {
    const payload: TreeDragPayload = {
      kind: layersById[node.id] ? 'layer' : 'slot',
      id: node.id,
    }
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
  const onNodeDrop = (target: TreeNodeData, e: React.DragEvent) => {
    if (!layersById[target.id]) return
    const p = readPayload(e)
    if (!p) return
    if (p.kind === 'layer') reparentEditorLayer(md, p.id, target.id)
    else moveSlotToLayer(md, p.id, target.id)
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

  const renderNodeActions = (node: TreeNodeData) => {
    const stop = (fn: () => void) => (e: React.MouseEvent) => {
      e.stopPropagation()
      fn()
    }
    const btn = 'rounded p-0.5 text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface'
    if (layersById[node.id]) {
      return (
        <>
          <button type="button" aria-label="Rename folder" title="Rename" className={btn} onClick={stop(() => setRenamingId(node.id))}>
            <Pencil className="size-3" />
          </button>
          <button type="button" aria-label="Delete folder" title="Delete folder" className={cn(btn, 'hover:text-error')} onClick={stop(() => deleteFolder(node.id))}>
            <Trash2 className="size-3" />
          </button>
        </>
      )
    }
    if (slotsById[node.id]) {
      return (
        <button type="button" aria-label="Delete unit" title="Delete unit" className={cn(btn, 'hover:text-error')} onClick={stop(() => deleteSlot(node.id))}>
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

  return (
    <Section
      title="Editor Layers"
      action={
        <button
          type="button"
          aria-label="New folder"
          title="New folder"
          onClick={() => setActiveLayer(addEditorLayer(md))}
          className="rounded p-0.5 text-on-surface-variant transition-colors hover:bg-primary/15 hover:text-primary"
        >
          <FolderPlus className="size-3.5" />
        </button>
      }
    >
      {nodes.length === 0 ? (
        <p className="px-2 py-3 text-center text-label-sm normal-case text-outline">
          No entities yet. Drag an asset from the right panel onto the map.
        </p>
      ) : (
        <>
          <TreeView
            nodes={nodes}
            selectedId={activeLayerId}
            selectedIds={selectedIds}
            onSelect={onSelect}
            onActivate={onActivate}
            allowFolderDrag
            onNodeDragStart={onNodeDragStart}
            onNodeDrop={onNodeDrop}
            renderNodeActions={renderNodeActions}
            renamingId={renamingId}
            onRenameCommit={onRenameCommit}
            onRenameCancel={() => setRenamingId(null)}
          />
          <div
            onDragOver={(e) => {
              if (!e.dataTransfer.types.includes(TREE_MIME)) return
              e.preventDefault()
              e.dataTransfer.dropEffect = 'move'
              setRootOver(true)
            }}
            onDragLeave={() => setRootOver(false)}
            onDrop={(e) => {
              e.preventDefault()
              onRootDrop(e)
              setRootOver(false)
            }}
            className={cn(
              'mt-1 rounded border border-dashed px-2 py-1.5 text-center text-label-sm normal-case transition-colors',
              rootOver
                ? 'border-primary bg-primary/10 text-primary'
                : 'border-outline-variant/30 text-outline',
            )}
          >
            Move folder to root
          </div>
        </>
      )}
    </Section>
  )
}

/** Shared sub-section frame for the left sidebar (header row + body). */
export function Section({
  title,
  action,
  children,
}: {
  title: string
  action?: React.ReactNode
  children: React.ReactNode
}) {
  return (
    <section className="flex flex-col">
      <div className="flex items-center justify-between px-2 py-1.5">
        <h3 className="text-label-sm uppercase tracking-wider text-outline">{title}</h3>
        {action}
      </div>
      <div className="px-1">{children}</div>
    </section>
  )
}
