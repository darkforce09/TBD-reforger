// Dense slot-icon cache (T-061.0.1) — the O(k) boundary engine behind drag pickup/release.
//
// At ~360k slots the base IconLayer can't afford to re-derive its data from `slotsById`
// at every drag boundary: `Object.values(slotsById).map(...)` allocated ~360k fresh
// SlotIcon objects on pickup (GC storm → hitch), and the release flush deep-cloned the
// whole slots Y.Map. This module keeps ONE persistent array of SlotIcon objects so the
// boundaries only ever touch the k dragged ids:
//
//   - exclude/restore  — swap-and-pop in/out of the dense array          O(k)
//   - patchPositions   — in-place x/y on the cached objects (relative)   O(k)
//   - setPositions     — in-place x/y on the cached objects (absolute)   O(k)
//
// Only a full snapshot replace (load / paste / delete cascade) rebuilds O(n). `getBaseIcons`
// hands Deck a fresh array identity (so it re-packs) but reuses the SlotIcon objects — the
// fresh `slice()` is a cheap pointer copy, taken lazily and only when the version moved.
//
// Module-level singleton: safe under the single-mounted-doc invariant (same as the
// LOCAL_ORIGIN / getMarkerIcon() singletons elsewhere in the engine).

import type { ID, Selection, Slot } from './schema'
import type { SlotIcon } from './selectors'
import * as spatialIndex from './slotSpatialIndex'
import * as clusterIndex from './slotClusterIndex'
import { CLUSTER_SLOT_THRESHOLD } from './constants'

let dense: SlotIcon[] = []
const index = new Map<ID, number>() // id -> position in `dense`
const excluded = new Map<ID, SlotIcon>() // ids lifted out of `dense` during a drag

let version = 0
let view: SlotIcon[] = []
let viewVersion = -1

function bump(): void {
  version++
}

function selectedSet(selection: Selection): Set<ID> | null {
  return selection.kind !== 'none' && selection.ids.length ? new Set(selection.ids) : null
}

/** O(n) full rebuild — only on a full snapshot replace (_applySnapshot). Drops any
 *  in-flight excluded ids (a snapshot is authoritative). */
export function rebuildFromSlots(slotsById: Record<ID, Slot>, selection: Selection): void {
  const selected = selectedSet(selection)
  dense = []
  index.clear()
  excluded.clear()
  for (const s of Object.values(slotsById)) {
    index.set(s.id, dense.length)
    dense.push({
      id: s.id,
      x: s.position.x,
      y: s.position.y,
      selected: selected?.has(s.id) ?? false,
    })
  }
  spatialIndex.rebuild(dense)
  // Only maintain the cluster index for missions large enough to ever cluster (T-065.1); clear it
  // otherwise so switching from a large to a small mission leaves no stale points.
  if (dense.length > CLUSTER_SLOT_THRESHOLD) clusterIndex.rebuild(dense)
  else clusterIndex.clear()
  bump()
}

/** O(k) append new slots to `dense` (slot-add fast path, T-062.0). Selection drives the
 *  highlight flag (a freshly-placed slot is normally unselected). Dense-only — never touches
 *  the `excluded` map, so an in-flight drag is unaffected. */
export function append(slots: Slot[], selection: Selection): void {
  const selected = selectedSet(selection)
  const added: SlotIcon[] = []
  for (const s of slots) {
    if (index.has(s.id)) continue // defensive: already present
    index.set(s.id, dense.length)
    const icon = {
      id: s.id,
      x: s.position.x,
      y: s.position.y,
      selected: selected?.has(s.id) ?? false,
    }
    dense.push(icon)
    added.push(icon)
  }
  spatialIndex.insert(added)
  // Cluster index (T-065.1): only when large. If this append crossed the threshold, the earlier
  // points were never inserted — rebuild from the full dense set instead of insert(added).
  if (dense.length > CLUSTER_SLOT_THRESHOLD) {
    if (dense.length - added.length > CLUSTER_SLOT_THRESHOLD) clusterIndex.insert(added)
    else clusterIndex.rebuild(dense)
  }
  bump()
}

/** O(k) remove ids from `dense` via swap-and-pop (slot-remove fast path, T-062.0). Mirror of
 *  `exclude` minus the `excluded` write. Ids not in `dense` are skipped — an id mid-drag lives
 *  in `excluded`; we leave it there (the active drag owns it). */
export function remove(ids: ID[]): void {
  for (const id of ids) {
    const i = index.get(id)
    if (i === undefined) continue
    const last = dense.length - 1
    if (i !== last) {
      const moved = dense[last]
      dense[i] = moved
      index.set(moved.id, i)
    }
    dense.pop()
    index.delete(id)
  }
  spatialIndex.remove(ids)
  if (dense.length > CLUSTER_SLOT_THRESHOLD) clusterIndex.remove(ids)
  bump()
}

/** O(k) swap-and-pop the given ids out of `dense` into `excluded` (drag start). */
export function exclude(ids: ID[]): void {
  for (const id of ids) {
    const i = index.get(id)
    if (i === undefined) continue
    const icon = dense[i]
    const last = dense.length - 1
    if (i !== last) {
      const moved = dense[last]
      dense[i] = moved
      index.set(moved.id, i)
    }
    dense.pop()
    index.delete(id)
    excluded.set(id, icon)
  }
  bump()
}

/** O(k) push the (already position-patched) excluded icons back into `dense` (drag end). */
export function restore(ids: ID[]): void {
  for (const id of ids) {
    const icon = excluded.get(id)
    if (!icon) continue
    excluded.delete(id)
    if (index.has(id)) continue // defensive: already present
    index.set(id, dense.length)
    dense.push(icon)
  }
  bump()
}

/** O(k) relative move — optimistic pointer-up patch before the Y.Doc flush lands. */
export function patchPositions(ids: ID[], delta: { dx: number; dy: number }): void {
  const patches: Record<ID, { x: number; y: number }> = {}
  for (const id of ids) {
    const icon = excluded.get(id) ?? denseIcon(id)
    if (!icon) continue
    icon.x += delta.dx
    icon.y += delta.dy
    patches[id] = { x: icon.x, y: icon.y }
  }
  spatialIndex.updatePositions(patches)
  if (dense.length > CLUSTER_SLOT_THRESHOLD) clusterIndex.updatePositions(patches)
  bump()
}

/** O(k) absolute set — bindings fast path; idempotent vs the relative optimistic patch. */
export function setPositions(patches: Record<ID, { x: number; y: number }>): void {
  for (const id in patches) {
    const icon = excluded.get(id) ?? denseIcon(id)
    if (!icon) continue
    icon.x = patches[id].x
    icon.y = patches[id].y
  }
  spatialIndex.updatePositions(patches)
  if (dense.length > CLUSTER_SLOT_THRESHOLD) clusterIndex.updatePositions(patches)
  bump()
}

/** O(n) refresh of the `selected` flag — once per selection change, never per drag frame. */
export function setSelectionFlags(selection: Selection): void {
  const selected = selectedSet(selection)
  for (const icon of dense) icon.selected = selected?.has(icon.id) ?? false
  for (const icon of excluded.values()) icon.selected = selected?.has(icon.id) ?? false
  bump()
}

function denseIcon(id: ID): SlotIcon | undefined {
  const i = index.get(id)
  return i === undefined ? undefined : dense[i]
}

/** Base-layer data. Fresh array identity (so Deck re-packs) but reused SlotIcon objects;
 *  the slice only runs when the version moved — pan never reaches it. */
export function getBaseIcons(): SlotIcon[] {
  if (viewVersion !== version) {
    view = dense.slice()
    viewVersion = version
  }
  return view
}

export function getVersion(): number {
  return version
}

/** Drop everything (store reset / doc unmount). */
export function clearSlotIconCache(): void {
  dense = []
  index.clear()
  excluded.clear()
  view = []
  viewVersion = -1
  spatialIndex.clear()
  clusterIndex.clear()
  bump()
}

/** Selected icons only (T-065) — the cluster-mode detail layer renders these over the cluster
 *  bubbles so a selection stays visible + highlighted at any zoom. O(n) scan, but only on a
 *  selection / snapshot change (iconCacheVersion), never per pan frame. Includes any excluded
 *  (mid-drag) icons so a dragged selection doesn't blink out. */
export function getSelectedIcons(selectedIds: Set<ID>): SlotIcon[] {
  if (!selectedIds.size) return []
  const out: SlotIcon[] = []
  for (const icon of dense) if (selectedIds.has(icon.id)) out.push(icon)
  for (const icon of excluded.values()) if (selectedIds.has(icon.id)) out.push(icon)
  return out
}
