// Pure, memoized transforms: store slices -> per-layer data arrays (Ultra Plan
// §4.4). Memoize on input refs — bindings.ts replaces a dictionary object only when
// it actually changes, so ref-equality caching avoids rebuilding a layer's data
// (and thus the layer) when an unrelated slice mutates.

import type { ID, Selection, Slot } from './schema'

export interface SlotIcon {
  id: ID
  x: number
  y: number
  selected: boolean
}

function memo2<A, B, R>(fn: (a: A, b: B) => R): (a: A, b: B) => R {
  let lastA: A | undefined
  let lastB: B | undefined
  let lastR: R | undefined
  let primed = false
  return (a, b) => {
    if (primed && a === lastA && b === lastB) return lastR as R
    lastA = a
    lastB = b
    lastR = fn(a, b)
    primed = true
    return lastR
  }
}

export const selectSlotIcons = memo2(
  (slotsById: Record<ID, Slot>, selection: Selection): SlotIcon[] =>
    Object.values(slotsById).map((s) => ({
      id: s.id,
      x: s.position.x,
      y: s.position.y,
      selected: selection.kind === 'slot' && selection.id === s.id,
    })),
)
