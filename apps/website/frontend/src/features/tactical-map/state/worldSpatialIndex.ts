// T-090.5.3 — Spatial index for WORLD objects (terrain buildings/piers/trees), the rbush that
// answers pickNearest/pickRect inside the world-objects worker (W2). Mirrors the query
// semantics of slotSpatialIndex.ts but is deliberately NOT that module: the slot index is a
// single-mounted-doc main-thread singleton owned by authored slots; the world index is a
// factory (one instance per worker core) over a different, much larger, chunk-evictable
// dataset (GAP-H3 / W3 — no shared singleton).
//
// Differences from the slot index, by design:
//  - createWorldSpatialIndex() factory — instantiated inside the worker; nothing module-level.
//  - Radii are WORLD METERS, not screen px: the worker has no viewport; callers convert
//    PICK_RADIUS_PX · 2^-deckZoom on the main thread (contract N4).
//  - Mutations are chunk-granular (insertChunk/removeChunk) to match streaming eviction —
//    per-chunk item lists are retained so rbush remove-by-reference works.
//  - Entries carry their render class so pick can filter to classes visible at the caller's
//    zoom (N4: never pick an invisible class).

import RBush from 'rbush'
import type { Bbox } from '../worldmap/chunkMath'

/** One indexed world instance (point box). `id` = `${chunkId}:${rowIndex}` (stable per export). */
export interface WorldIndexEntry {
  id: string
  x: number
  y: number
  /** Render class (lodGates WorldRenderClass) — pick-time visibility filter input. */
  cls: string
}

interface Item {
  minX: number
  minY: number
  maxX: number
  maxY: number
  id: string
  cls: string
}

export interface WorldSpatialIndex {
  /** Bulk-insert one chunk's instances (idempotent: a chunk already present is replaced). */
  insertChunk(chunkId: string, entries: WorldIndexEntry[]): void
  /** Remove a chunk's instances (LRU eviction / unload). Unknown chunk = no-op. */
  removeChunk(chunkId: string): void
  /** Nearest instance id within `radiusM` of (x, y), optionally class-filtered, else null. */
  pickNearest(x: number, y: number, radiusM: number, clsFilter?: (cls: string) => boolean): string | null
  /** All instance ids inside a world-meter bbox, optionally class-filtered. */
  pickRect(bbox: Bbox, clsFilter?: (cls: string) => boolean): string[]
  /** Drop everything (terrain switch / worker unload). */
  clear(): void
  /** Total indexed instances (tests + budget instrumentation). */
  size(): number
}

export function createWorldSpatialIndex(): WorldSpatialIndex {
  const tree = new RBush<Item>()
  const byChunk = new Map<string, Item[]>()
  let count = 0

  function removeChunk(chunkId: string): void {
    const items = byChunk.get(chunkId)
    if (!items) return
    for (const it of items) tree.remove(it)
    count -= items.length
    byChunk.delete(chunkId)
  }

  return {
    insertChunk(chunkId, entries) {
      removeChunk(chunkId)
      const items: Item[] = new Array(entries.length)
      for (let i = 0; i < entries.length; i++) {
        const e = entries[i]
        items[i] = { minX: e.x, minY: e.y, maxX: e.x, maxY: e.y, id: e.id, cls: e.cls }
      }
      // Bulk load is O(n) vs n inserts (same trade the slot index makes on rebuild).
      tree.load(items)
      byChunk.set(chunkId, items)
      count += items.length
    },

    removeChunk,

    pickNearest(x, y, radiusM, clsFilter) {
      const hits = tree.search({ minX: x - radiusM, minY: y - radiusM, maxX: x + radiusM, maxY: y + radiusM })
      let best: string | null = null
      let bestD = radiusM * radiusM
      for (const h of hits) {
        if (clsFilter && !clsFilter(h.cls)) continue
        const dx = h.minX - x
        const dy = h.minY - y
        const d = dx * dx + dy * dy
        // The box search over-matches its corners; enforce the true circular radius.
        if (d <= bestD && (best === null || d < bestD)) {
          bestD = d
          best = h.id
        }
      }
      return best
    },

    pickRect(bbox, clsFilter) {
      const hits = tree.search({
        minX: Math.min(bbox[0], bbox[2]),
        minY: Math.min(bbox[1], bbox[3]),
        maxX: Math.max(bbox[0], bbox[2]),
        maxY: Math.max(bbox[1], bbox[3]),
      })
      const out: string[] = []
      for (const h of hits) {
        if (clsFilter && !clsFilter(h.cls)) continue
        out.push(h.id)
      }
      return out
    },

    clear() {
      tree.clear()
      byChunk.clear()
      count = 0
    },

    size() {
      return count
    },
  }
}
