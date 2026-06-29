// Pure, memoized transforms: store slices -> per-layer data arrays (Ultra Plan
// §4.4). Memoize on input refs — bindings.ts replaces a dictionary object only when
// it actually changes, so ref-equality caching avoids rebuilding a layer's data
// (and thus the layer) when an unrelated slice mutates.

import type { ID, Slot } from './schema'

export interface SlotIcon {
  id: ID
  x: number
  y: number
  selected: boolean
}

function memo3<A, B, C, R>(fn: (a: A, b: B, c: C) => R): (a: A, b: B, c: C) => R {
  let lastA: A | undefined
  let lastB: B | undefined
  let lastC: C | undefined
  let lastR: R | undefined
  let primed = false
  return (a, b, c) => {
    if (primed && a === lastA && b === lastB && c === lastC) return lastR as R
    lastA = a
    lastB = b
    lastC = c
    lastR = fn(a, b, c)
    primed = true
    return lastR
  }
}

// Drag-preview overlay (T-061): only the dragged ids, offset by the live world delta.
// O(k) in the selection size — the per-frame path during a move — never O(total slots).
export const selectDragOverlayIcons = memo3(
  (
    slotsById: Record<ID, Slot>,
    ids: ID[] | null,
    delta: { dx: number; dy: number } | null,
  ): SlotIcon[] => {
    if (!ids?.length || !delta) return []
    const out: SlotIcon[] = []
    for (const id of ids) {
      const s = slotsById[id]
      if (!s) continue
      out.push({
        id: s.id,
        x: s.position.x + delta.dx,
        y: s.position.y + delta.dy,
        selected: true,
      })
    }
    return out
  },
)
