// Spatial index for click / marquee picking (T-063) — a Rust grid index in world meters (T-145
// Phase 3.1) that runs parallel to the dense `slotIconCache`. Pan/zoom (T-057) and drag-move (T-061)
// are fast @ ~360k, but picking was not: every click, dbl-click, drag-start and marquee release ran a
// Deck.gl GPU pick pass over EVERY icon. This index answers those queries by looking only at icons
// near the cursor / inside the box, so `slot-icons` can drop `pickable` entirely.
//
// The query engine is `map-engine-core::spatial::point_index` (wasm `SlotIndex`) — a uniform CSR grid
// pinned set-equal to `rbush` by `features/_wasm/slotIndex.parity.test.ts`. This module keeps the
// incremental point bookkeeping (world xs/ys columns + a row→id table, so edits stay O(k)) and
// rebuilds the built-once wasm index lazily: only an EDIT (`dirty`) rebuilds it — a pick never does.
// Editor picks fire at human cadence (click / marquee release / drag-start), never per frame, so the
// O(n) rebuild after an edit is imperceptible (and marquee drags don't edit → no rebuild mid-marquee).
//
// **Pick semantics preserved exactly:** `pickNearest` is nearest-IN-BOX (no circular cutoff — unlike
// worldSpatialIndex), so both picks run off the wasm `pick_rect` (≡ rbush.search) + the same JS
// min-distance loop the rbush version used.
//
// Module-level singleton: safe under the single-mounted-doc invariant (same as slotIconCache and the
// LOCAL_ORIGIN / getMarkerIcon() singletons elsewhere in the engine).

import * as wasm from '@/wasm/pkg/map_engine_wasm'
import type { ID } from './schema'
import type { SlotIcon } from './selectors'

/** Minimal viewport surface (mirror of useSelectTool's) — keeps Deck out of this module. */
interface Viewport {
  unproject: (xy: number[]) => number[]
}

/** Grid cell size in world meters (a few hundred metres works for slots; matches the parity probe). */
const GRID_CELL_M = 256

// Incremental point bookkeeping (row = handle): world columns + a row→id table + id→row map.
const xs: number[] = []
const ys: number[] = []
const rowIds: ID[] = []
const index = new Map<ID, number>()
let si: wasm.SlotIndex | null = null
let dirty = true

/** O(n) full rebuild — on a full snapshot replace. */
export function rebuild(icons: SlotIcon[]): void {
  xs.length = 0
  ys.length = 0
  rowIds.length = 0
  index.clear()
  for (let i = 0; i < icons.length; i++) {
    index.set(icons[i].id, i)
    xs.push(icons[i].x)
    ys.push(icons[i].y)
    rowIds.push(icons[i].id)
  }
  dirty = true
}

/** O(k) insert newly-placed icons (asset drop / paste). Ids already present are skipped. */
export function insert(icons: { id: ID; x: number; y: number }[]): void {
  for (const s of icons) {
    if (index.has(s.id)) continue
    index.set(s.id, xs.length)
    xs.push(s.x)
    ys.push(s.y)
    rowIds.push(s.id)
  }
  dirty = true
}

/** O(k) remove ids via swap-and-pop. Ids not present are skipped. */
export function remove(ids: ID[]): void {
  for (const id of ids) {
    const i = index.get(id)
    if (i === undefined) continue
    const last = xs.length - 1
    if (i !== last) {
      xs[i] = xs[last]
      ys[i] = ys[last]
      const movedId = rowIds[last]
      rowIds[i] = movedId
      index.set(movedId, i)
    }
    xs.pop()
    ys.pop()
    rowIds.pop()
    index.delete(id)
  }
  dirty = true
}

/** O(k) reposition: rewrite the world columns in place. */
export function updatePositions(patches: Record<ID, { x: number; y: number }>): void {
  for (const id in patches) {
    const i = index.get(id)
    if (i === undefined) continue
    xs[i] = patches[id].x
    ys[i] = patches[id].y
  }
  dirty = true
}

/** Drop everything (store reset / doc unmount). */
export function clear(): void {
  xs.length = 0
  ys.length = 0
  rowIds.length = 0
  index.clear()
  si?.free()
  si = null
  dirty = true
}

/** Build the wasm grid once; reused across picks. Rebuilds only when an edit dirtied the point set. */
function ensureIndex(): wasm.SlotIndex {
  if (dirty || !si) {
    si?.free()
    si = wasm.SlotIndex.build(new Float32Array(xs), new Float32Array(ys), GRID_CELL_M)
    dirty = false
  }
  return si
}

/** Nearest icon to a screen-pixel click within `radiusPx` (world-projected), else null. Nearest-in-box
 *  (no circular cutoff) — identical to the former rbush `search` + min-distance loop. */
export function pickNearest(px: [number, number], viewport: Viewport, radiusPx = 4): ID | null {
  if (!xs.length) return null
  const center = viewport.unproject(px)
  const cx = center[0]
  const cy = center[1]
  // Convert the screen-pixel hit radius to world meters (same flipY:false unproject math).
  const edge = viewport.unproject([px[0] + radiusPx, px[1]])
  const r = Math.abs(edge[0] - cx)
  const hits = ensureIndex().pick_rect(cx - r, cy - r, cx + r, cy + r)
  if (!hits.length) return null
  let best: ID | null = null
  let bestD = Infinity
  for (const h of hits) {
    const dx = xs[h] - cx
    const dy = ys[h] - cy
    const d = dx * dx + dy * dy
    if (d < bestD) {
      bestD = d
      best = rowIds[h]
    }
  }
  return best
}

/** All icon ids inside a world-meter rectangle (marquee box select). */
export function pickRect(x0: number, y0: number, x1: number, y1: number): ID[] {
  if (!xs.length) return []
  const minX = Math.min(x0, x1)
  const maxX = Math.max(x0, x1)
  const minY = Math.min(y0, y1)
  const maxY = Math.max(y0, y1)
  const hits = ensureIndex().pick_rect(minX, minY, maxX, maxY)
  const out: ID[] = new Array(hits.length)
  for (let i = 0; i < hits.length; i++) out[i] = rowIds[hits[i]]
  return out
}
