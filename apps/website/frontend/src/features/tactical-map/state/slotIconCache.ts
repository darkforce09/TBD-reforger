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
import type { TerrainDef } from '../coords/terrains'
import { getTerrain } from '../coords/terrains'
import * as spatialIndex from './slotSpatialIndex'
import * as clusterIndex from './slotClusterIndex'
import { CLUSTER_SLOT_THRESHOLD } from './constants'
import { chunkColRow, chunkKey, chunkRectForBbox, terrainChunkCols } from './spatialChunks'

let dense: SlotIcon[] = []
const index = new Map<ID, number>() // id -> position in `dense`
const excluded = new Map<ID, SlotIcon>() // ids lifted out of `dense` during a drag

let version = 0
let view: SlotIcon[] = []
let viewVersion = -1

// Viewport cull (T-067.0): geographic chunk membership for getBaseIconsForBbox. `chunkBuckets`
// maps a 512m chunk key → the dense SlotIcon objects sitting in it (reused, not copied — a
// position patch mutates the shared object in place); `iconChunk` is the id → current chunk so a
// move/remove is O(1). Maintained in every dense mutator below (kept out of the `excluded` map:
// a dragged icon leaves its bucket so the culled base layer can't ghost it under the drag
// overlay). `cullView`/`lastCullSig` cache the last culled array so an intra-chunk pan returns the
// same reference and Deck never re-packs (the ~160 fps contract). Default terrain = Everon (same
// default the store/view open with) so a prime that runs before setChunkTerrain still buckets.
let cullTerrain: TerrainDef = getTerrain()
const chunkBuckets = new Map<number, Set<SlotIcon>>()
const iconChunk = new Map<ID, number>()
let cullView: SlotIcon[] = []
let lastCullSig = ''

function bucketAdd(icon: SlotIcon): void {
  const [cx, cy] = chunkColRow(icon.x, icon.y, cullTerrain)
  const key = chunkKey(cx, cy, terrainChunkCols(cullTerrain))
  iconChunk.set(icon.id, key)
  let set = chunkBuckets.get(key)
  if (!set) chunkBuckets.set(key, (set = new Set()))
  set.add(icon)
}

function bucketRemove(icon: SlotIcon): void {
  const key = iconChunk.get(icon.id)
  if (key !== undefined) chunkBuckets.get(key)?.delete(icon)
  iconChunk.delete(icon.id)
}

/** Re-home a dense icon whose x/y just changed; no-op when it didn't cross a chunk boundary. */
function bucketMove(icon: SlotIcon): void {
  const [cx, cy] = chunkColRow(icon.x, icon.y, cullTerrain)
  const next = chunkKey(cx, cy, terrainChunkCols(cullTerrain))
  const prev = iconChunk.get(icon.id)
  if (prev === next) return
  if (prev !== undefined) chunkBuckets.get(prev)?.delete(icon)
  iconChunk.set(icon.id, next)
  let set = chunkBuckets.get(next)
  if (!set) chunkBuckets.set(next, (set = new Set()))
  set.add(icon)
}

function rebuildBuckets(): void {
  chunkBuckets.clear()
  iconChunk.clear()
  for (const icon of dense) bucketAdd(icon)
  lastCullSig = '' // buckets changed without a version-bearing edit → force next cull recompute
}

/** Align the chunk grid to the active terrain (mirrors slotClusterIndex.setTerrain). Rebuilds
 *  buckets when the column count changes so keys stay valid (Everon 25 cols vs Arland 20). */
export function setChunkTerrain(terrain: TerrainDef): void {
  if (terrain === cullTerrain) return
  cullTerrain = terrain
  rebuildBuckets()
}

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
  rebuildBuckets() // T-067.0: chunk membership for the viewport cull
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
    bucketAdd(icon) // T-067.0: chunk membership for the viewport cull
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
    bucketRemove(dense[i]) // T-067.0: drop from chunk membership before the swap-pop
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
    bucketRemove(icon) // T-067.0: a dragged icon leaves its bucket (overlay layer renders it)
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
    bucketAdd(icon) // T-067.0: re-home into its (patched-position) chunk
  }
  bump()
}

/** O(k) relative move — optimistic pointer-up patch before the Y.Doc flush lands. */
export function patchPositions(ids: ID[], delta: { dx: number; dy: number }): void {
  const patches: Record<ID, { x: number; y: number }> = {}
  for (const id of ids) {
    const ex = excluded.get(id)
    const icon = ex ?? denseIcon(id)
    if (!icon) continue
    icon.x += delta.dx
    icon.y += delta.dy
    patches[id] = { x: icon.x, y: icon.y }
    if (!ex) bucketMove(icon) // T-067.0: re-home a dense icon if it crossed a chunk seam
  }
  spatialIndex.updatePositions(patches)
  if (dense.length > CLUSTER_SLOT_THRESHOLD) clusterIndex.updatePositions(patches)
  bump()
}

/** O(k) absolute set — bindings fast path; idempotent vs the relative optimistic patch. */
export function setPositions(patches: Record<ID, { x: number; y: number }>): void {
  for (const id in patches) {
    const ex = excluded.get(id)
    const icon = ex ?? denseIcon(id)
    if (!icon) continue
    icon.x = patches[id].x
    icon.y = patches[id].y
    if (!ex) bucketMove(icon) // T-067.0: re-home a dense icon if it crossed a chunk seam
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
  chunkBuckets.clear()
  iconChunk.clear()
  cullView = []
  lastCullSig = ''
  spatialIndex.clear()
  clusterIndex.clear()
  bump()
}

/** Viewport-culled base-layer data (T-067.0). Chunk-rect variant — preferred at runtime so the
 *  layer hook can key on a stable `cx0,cy0,cx1,cy1` signature instead of a fresh bbox array each
 *  pan frame (T-067.0.1 pan-stability fix). */
export function getBaseIconsForChunkRect(
  cx0: number,
  cy0: number,
  cx1: number,
  cy1: number,
  extraIds: Set<ID>,
): SlotIcon[] {
  const sig = `${version}|${cx0},${cy0},${cx1},${cy1}`
  if (sig === lastCullSig) return cullView

  const cols = terrainChunkCols(cullTerrain)
  const out: SlotIcon[] = []
  for (let cy = cy0; cy <= cy1; cy++) {
    for (let cx = cx0; cx <= cx1; cx++) {
      const set = chunkBuckets.get(chunkKey(cx, cy, cols))
      if (set) for (const icon of set) out.push(icon)
    }
  }
  for (const id of extraIds) {
    const key = iconChunk.get(id)
    if (key === undefined) continue
    const cx = key % cols
    const cy = (key - cx) / cols
    if (cx >= cx0 && cx <= cx1 && cy >= cy0 && cy <= cy1) continue
    const icon = denseIcon(id)
    if (icon) out.push(icon)
  }

  cullView = out
  lastCullSig = sig
  return cullView
}

/** Viewport-culled base-layer data (T-067.0). Bbox wrapper — derives chunk rect then delegates. */
export function getBaseIconsForBbox(
  bbox: [number, number, number, number],
  extraIds: Set<ID>,
): SlotIcon[] {
  const [cx0, cy0, cx1, cy1] = chunkRectForBbox(bbox, cullTerrain, 0)
  return getBaseIconsForChunkRect(cx0, cy0, cx1, cy1, extraIds)
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
