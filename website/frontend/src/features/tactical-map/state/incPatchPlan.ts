// Incremental-binding transaction classifier (T-062.0) — generalizes the T-061 drag-move
// fast path. At ~360k slots a full `docToSnapshot(md)` (`slots.toJSON()` over every Y.Map)
// costs seconds, so it must NOT run for everyday single-entity edits (asset drop, delete,
// title/env edit, outliner rename/reparent/move-to-layer). `classifyTransaction` inspects a
// Y.Doc transaction and, when it is one of those cheap edits, returns a PatchPlan the store
// can apply O(k); anything structural/bulk/ambiguous returns null → caller does the full
// snapshot (always-correct fallback).
//
// FIRING-ORDER INDEPENDENCE: bindings registers observeDeep on EACH top-level map, so one
// transaction can fire the observer several times (e.g. an asset drop touches the slots map +
// a squad child + a layer child → three firings, same `txn`). The kind decision and ALL data
// gathering read the WHOLE transaction (`txn.changed`) plus post-transaction doc reads — never
// just this firing's `events` — so the same plan is returned regardless of which subtree fired
// first. The only `events` consumer is `slot-fields`, and a pure slot-field txn touches only the
// slots subtree (one firing, complete events) — the preserved T-061 behavior.

import type * as Y from 'yjs'
import type { EditorLayer, ID, MissionMeta, Slot, Squad } from './schema'
import type { MissionDoc } from './ydoc'

/** Largest changed-entity count the slot-fields / slot-add fast paths will handle; above this
 *  the full snapshot is cheaper / safer. Single-entity edits sit far under this. */
export const FAST_PATCH_CAP = 512

/** Slot-remove tolerates a much larger batch (T-062.0.1): the batched `removeEntities` delete is
 *  O(n+k) in the Y.Doc and `_patchRemoveSlots` is O(k) in the store (in-place key deletes + icon
 *  swap-pop), so a multi-thousand delete stays incremental instead of crashing the tab via a full
 *  ~360k snapshot. */
export const REMOVE_PATCH_CAP = 10_000

export type PatchPlan =
  | { kind: 'slot-fields'; ids: ID[] }
  | { kind: 'slot-add'; slot: Slot; squads: Record<ID, Squad>; layers: Record<ID, EditorLayer> }
  | { kind: 'slot-add-bulk'; slots: Slot[]; squads: Record<ID, Squad>; layers: Record<ID, EditorLayer> }
  | { kind: 'slot-remove'; ids: ID[]; squads: Record<ID, Squad>; layers: Record<ID, EditorLayer> }
  | { kind: 'meta'; meta: MissionMeta }
  | { kind: 'editor-layers'; patches: Record<ID, EditorLayer> }

type AnyType = Y.AbstractType<unknown>

/** Build a Record<id, T> from a list of changed entity child Y.Maps (key by their own `id`). */
function childPatches<T>(children: Y.Map<unknown>[]): Record<ID, T> {
  const out: Record<ID, T> = {}
  for (const c of children) {
    const id = c.get('id') as ID | undefined
    if (typeof id === 'string') out[id] = c.toJSON() as T
  }
  return out
}

export function classifyTransaction(
  md: MissionDoc,
  events: Y.YEvent<AnyType>[],
  txn: Y.Transaction,
): PatchPlan | null {
  const e = md.entities
  const slotsMap = e.slots as unknown as AnyType
  const squadsMap = e.squads as unknown as AnyType
  const layersMap = e.editorLayers as unknown as AnyType
  const metaMap = md.meta as unknown as AnyType
  // Structural change to any of these maps → not a fast-path edit (new faction/squad on the
  // empty-doc bootstrap, or any vehicle/marker/objective/loadout/item op).
  const forbiddenMaps = new Set<AnyType>([
    e.factions as unknown as AnyType,
    squadsMap,
    e.loadouts as unknown as AnyType,
    e.items as unknown as AnyType,
    e.objectives as unknown as AnyType,
    e.vehicles as unknown as AnyType,
    e.markers as unknown as AnyType,
  ])

  if (txn.changed.size === 0) return null

  let slotsStructural = false
  let layersStructural = false
  let metaChanged = false
  let forbiddenStructural = false
  let forbiddenChild = false
  const slotChildren: Y.Map<unknown>[] = []
  const squadChildren: Y.Map<unknown>[] = []
  const layerChildren: Y.Map<unknown>[] = []

  for (const type of txn.changed.keys()) {
    const t = type as unknown as AnyType
    if (t === slotsMap) {
      slotsStructural = true
    } else if (t === layersMap) {
      layersStructural = true
    } else if (t === metaMap) {
      metaChanged = true
    } else if (forbiddenMaps.has(t)) {
      forbiddenStructural = true
    } else {
      // A nested entity child Y.Map — classify by its parent map.
      const p = t.parent as unknown as AnyType | null
      if (p === slotsMap) slotChildren.push(t as Y.Map<unknown>)
      else if (p === squadsMap) squadChildren.push(t as Y.Map<unknown>)
      else if (p === layersMap) layerChildren.push(t as Y.Map<unknown>)
      else forbiddenChild = true // faction/vehicle/marker/… child, deeper nesting, or null
    }
  }

  if (forbiddenChild || forbiddenStructural) return null

  // ── slot structural: add or remove ──────────────────────────────────────────
  if (slotsStructural) {
    if (layersStructural) return null // removeEditorLayer cascade (deletes layers + slots)
    if (slotChildren.length > 0) return null // mixed add + field edit — not a known single action
    const changed = txn.changed as unknown as Map<AnyType, Set<string | null>>
    const keys = changed.get(slotsMap)
    // Cap at the larger remove ceiling here; an add (which must be a single slot) is re-checked below.
    if (!keys || keys.size === 0 || keys.size > REMOVE_PATCH_CAP) return null
    const added: ID[] = []
    const removed: ID[] = []
    for (const k of keys) {
      if (typeof k !== 'string') return null
      if (e.slots.get(k)) added.push(k)
      else removed.push(k)
    }
    if (added.length && removed.length) return null // ambiguous mix → safe fallback

    if (added.length) {
      // Single addSlot (asset drop) — the original T-062.0 fast path.
      if (added.length === 1) {
        const slot = e.slots.get(added[0])?.toJSON() as Slot | undefined
        if (!slot) return null
        return {
          kind: 'slot-add',
          slot,
          squads: childPatches<Squad>(squadChildren),
          layers: childPatches<EditorLayer>(layerChildren),
        }
      }
      // Bulk paste (T-067.0): >1 add, already capped at REMOVE_PATCH_CAP by the keys.size guard
      // above. Gather each new slot once so the store applies an O(k) bulk insert instead of a
      // full ~360k docToSnapshot rebuild. Any missing slot → safe fallback to the full snapshot.
      const slots: Slot[] = []
      for (const id of added) {
        const slot = e.slots.get(id)?.toJSON() as Slot | undefined
        if (!slot) return null
        slots.push(slot)
      }
      return {
        kind: 'slot-add-bulk',
        slots,
        squads: childPatches<Squad>(squadChildren),
        layers: childPatches<EditorLayer>(layerChildren),
      }
    }
    return {
      kind: 'slot-remove',
      ids: removed,
      squads: childPatches<Squad>(squadChildren),
      layers: childPatches<EditorLayer>(layerChildren),
    }
  }

  // ── meta-only (setTitle, updateEnvironment, post-boot applyMissionRowMeta) ────
  if (
    metaChanged &&
    !layersStructural &&
    slotChildren.length === 0 &&
    squadChildren.length === 0 &&
    layerChildren.length === 0
  ) {
    return { kind: 'meta', meta: md.meta.toJSON() as MissionMeta }
  }

  // ── editor-layers field edits (rename / reparent / moveSlotToLayer) ──────────
  if (
    !metaChanged &&
    !layersStructural &&
    slotChildren.length === 0 &&
    squadChildren.length === 0 &&
    layerChildren.length > 0
  ) {
    if (layerChildren.length > FAST_PATCH_CAP) return null
    return { kind: 'editor-layers', patches: childPatches<EditorLayer>(layerChildren) }
  }

  // ── slot field edits (updateSlot, updateSlotPosition, moveEntities, drag) ─────
  // Preserve T-061 exactly: every change is a slot child, ids from THIS firing's events.
  if (
    !metaChanged &&
    !layersStructural &&
    squadChildren.length === 0 &&
    layerChildren.length === 0 &&
    slotChildren.length > 0
  ) {
    const ids: ID[] = []
    const seen = new Set<ID>()
    for (const ev of events) {
      const id = ev.path[0]
      if (typeof id !== 'string' || seen.has(id)) continue
      seen.add(id)
      ids.push(id)
      if (ids.length > FAST_PATCH_CAP) return null
    }
    return ids.length ? { kind: 'slot-fields', ids } : null
  }

  return null
}
