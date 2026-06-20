// "Editor Layers" section of the left sidebar — the Eden workflow-folder tree (Ultra
// Plan §5.1), bound to the live Y.Doc `editorLayers` map. Renders the folder hierarchy
// with the real slots filed inside each folder: selecting a slot selects it in global
// state (no auto camera move — Spacebar centers, Phase 3.5 Task 7); selecting a folder
// makes it the active drop target; the "+" creates a new folder; double-clicking a slot
// opens its Attributes modal. Reparent drag-and-drop is Phase 7a.

import { useMemo } from 'react'
import { Folder, FolderPlus, User } from 'lucide-react'
import { addEditorLayer, useMapStore } from '@/features/tactical-map'
import type { EditorLayer, ID, MissionDoc, Slot } from '@/features/tactical-map'
import { TreeView, type TreeNodeData } from '../tree/TreeView'

interface EditorLayersSectionProps {
  md: MissionDoc
  onActivateSlot?: (id: ID) => void
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

  const nodes = useMemo(() => buildTree(layersById, slotsById), [layersById, slotsById])

  // Highlight the selected slot, or — when nothing is selected — the active folder.
  const selectedId = selection.kind === 'slot' ? selection.id : activeLayerId

  const onSelect = (id: string) => {
    if (layersById[id]) {
      setActiveLayer(id)
      return
    }
    if (slotsById[id]) setSelection({ kind: 'slot', id })
  }

  const onActivate = (id: string) => {
    if (slotsById[id]) onActivateSlot?.(id)
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
        <TreeView
          nodes={nodes}
          selectedId={selectedId}
          onSelect={onSelect}
          onActivate={onActivate}
        />
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
