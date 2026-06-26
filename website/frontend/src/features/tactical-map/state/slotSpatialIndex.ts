// Spatial index for click / marquee picking (T-063) — an rbush R-tree in world meters that
// runs parallel to the dense `slotIconCache`. Pan/zoom (T-057) and drag-move (T-061) are fast
// @ ~360k, but picking was not: every click, dbl-click, drag-start and marquee release ran a
// Deck.gl GPU pick pass over EVERY icon. This tree answers those queries by looking only at
// icons near the cursor / inside the box, so `slot-icons` can drop `pickable` entirely.
//
// Kept in sync on the SAME mutations as `slotIconCache` (called from inside its mutators), so
// the tree mirrors the icon cache through every path — single add, bulk paste, snapshot load,
// delete cascade, drag release. Icons are stored as degenerate point boxes (min === max); the
// per-id Item reference is retained so `tree.remove(item)` matches by reference (rbush default).
//
// Module-level singleton: safe under the single-mounted-doc invariant (same as slotIconCache
// and the LOCAL_ORIGIN / getMarkerIcon() singletons elsewhere in the engine).

import RBush from 'rbush'
import type { ID } from './schema'
import type { SlotIcon } from './selectors'

interface Item {
  minX: number
  minY: number
  maxX: number
  maxY: number
  id: ID
}

/** Minimal viewport surface (mirror of useSelectTool's) — keeps Deck out of this module. */
interface Viewport {
  unproject: (xy: number[]) => number[]
}

const tree = new RBush<Item>()
const items = new Map<ID, Item>()

function makeItem(id: ID, x: number, y: number): Item {
  return { minX: x, minY: y, maxX: x, maxY: y, id }
}

/** O(n) full rebuild via bulk load (faster than n inserts) — on a full snapshot replace. */
export function rebuild(icons: SlotIcon[]): void {
  tree.clear()
  items.clear()
  const arr: Item[] = new Array(icons.length)
  for (let i = 0; i < icons.length; i++) {
    const it = makeItem(icons[i].id, icons[i].x, icons[i].y)
    items.set(icons[i].id, it)
    arr[i] = it
  }
  tree.load(arr)
}

/** O(k) insert newly-placed icons (asset drop / paste). Ids already present are skipped. */
export function insert(icons: { id: ID; x: number; y: number }[]): void {
  for (const s of icons) {
    if (items.has(s.id)) continue
    const it = makeItem(s.id, s.x, s.y)
    items.set(s.id, it)
    tree.insert(it)
  }
}

/** O(k) remove ids from the tree. Ids not present are skipped. */
export function remove(ids: ID[]): void {
  for (const id of ids) {
    const it = items.get(id)
    if (!it) continue
    tree.remove(it)
    items.delete(id)
  }
}

/** O(k) reposition: rbush has no in-place move, so remove the old box and insert a new one. */
export function updatePositions(patches: Record<ID, { x: number; y: number }>): void {
  for (const id in patches) {
    const old = items.get(id)
    if (old) tree.remove(old)
    const it = makeItem(id, patches[id].x, patches[id].y)
    items.set(id, it)
    tree.insert(it)
  }
}

/** Drop everything (store reset / doc unmount). */
export function clear(): void {
  tree.clear()
  items.clear()
}

/** Nearest icon to a screen-pixel click within `radiusPx` (world-projected), else null. */
export function pickNearest(
  px: [number, number],
  viewport: Viewport,
  radiusPx = 4,
): ID | null {
  const center = viewport.unproject(px)
  const cx = center[0]
  const cy = center[1]
  // Convert the screen-pixel hit radius to world meters (same flipY:false unproject math).
  const edge = viewport.unproject([px[0] + radiusPx, px[1]])
  const r = Math.abs(edge[0] - cx)
  const hits = tree.search({ minX: cx - r, minY: cy - r, maxX: cx + r, maxY: cy + r })
  if (!hits.length) return null
  let best: ID | null = null
  let bestD = Infinity
  for (const h of hits) {
    const dx = h.minX - cx
    const dy = h.minY - cy
    const d = dx * dx + dy * dy
    if (d < bestD) {
      bestD = d
      best = h.id
    }
  }
  return best
}

/** All icon ids inside a world-meter rectangle (marquee box select). */
export function pickRect(x0: number, y0: number, x1: number, y1: number): ID[] {
  const minX = Math.min(x0, x1)
  const maxX = Math.max(x0, x1)
  const minY = Math.min(y0, y1)
  const maxY = Math.max(y0, y1)
  return tree.search({ minX, minY, maxX, maxY }).map((h) => h.id)
}
